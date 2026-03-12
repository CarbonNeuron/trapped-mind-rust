//! Application entry point and async event loop.
//!
//! Sets up the terminal, spawns background tasks for system polling, terminal
//! event reading, and animation ticking, then runs the main event loop that
//! dispatches [`AppEvent`]s to the [`App`] state machine.

mod app;
mod config;
mod error;
mod history;
mod llm;
mod ollama;
mod pet_states;
mod system;
mod ui;

use app::{App, AppEvent, AppMode, HandleResult};
use config::{AppConfig, CliArgs};
use llm::LlmClient;
use system::SystemReader;

use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Set up file logging (TUI owns stdout/stderr)
    let log_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from(".config"))
        .join("trapped-mind");
    let file_appender = tracing_appender::rolling::daily(&log_dir, "trapped-mind.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    tracing::info!("trapped-mind starting");

    let cli = CliArgs::parse();
    let config = AppConfig::load(&cli);
    config
        .validate()
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let mut app = App::new(config.clone());

    // Display which sensors are real vs simulated at startup
    {
        let sys_reader = SystemReader::new();
        app.add_system_message(sys_reader.sensor_status_message());
    }

    app.log_startup();

    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();

    // SystemReader is !Send (battery crate uses Rc), so it must live on a dedicated OS thread
    let tx_sys = tx.clone();
    std::thread::spawn(move || {
        let mut reader = SystemReader::new();
        loop {
            std::thread::sleep(Duration::from_millis(200));
            let info = reader.read();
            if tx_sys.send(AppEvent::SystemTick(info)).is_err() {
                break;
            }
        }
    });

    // Forward crossterm terminal events into the unified channel
    let tx_term = tx.clone();
    tokio::spawn(async move {
        loop {
            match tokio::task::spawn_blocking(|| event::poll(Duration::from_millis(50))).await {
                Ok(Ok(true)) => {
                    if let Ok(evt) = event::read() {
                        if tx_term.send(AppEvent::Terminal(evt)).is_err() {
                            break;
                        }
                    }
                }
                Ok(Ok(false)) => {}
                _ => break,
            }
        }
    });

    // Drive pet face animation at 2 fps
    let tx_anim = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(500));
        loop {
            interval.tick().await;
            if tx_anim.send(AppEvent::AnimationTick).is_err() {
                break;
            }
        }
    });

    // Graceful shutdown on Ctrl+C
    let tx_ctrlc = tx.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            let _ = tx_ctrlc.send(AppEvent::Shutdown);
        }
    });

    // Graceful shutdown on SIGTERM (Unix only)
    #[cfg(unix)]
    {
        let tx_sigterm = tx.clone();
        tokio::spawn(async move {
            use tokio::signal::unix::{signal, SignalKind};
            if let Ok(mut sig) = signal(SignalKind::terminate()) {
                sig.recv().await;
                let _ = tx_sigterm.send(AppEvent::Shutdown);
            }
        });
    }

    let llm: Arc<dyn LlmClient> = Arc::new(ollama::OllamaClient::new(
        &config.ollama_host,
        config.ollama_port,
        config.ollama_timeout_secs,
    ));

    let mut terminal = ratatui::init();
    let result = run_app(&mut terminal, &mut app, &mut rx, &tx, &llm).await;
    ratatui::restore();
    result
}

/// Main event loop — draws the UI and dispatches events until the app exits.
async fn run_app(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    rx: &mut mpsc::UnboundedReceiver<AppEvent>,
    tx: &mpsc::UnboundedSender<AppEvent>,
    llm: &Arc<dyn LlmClient>,
) -> anyhow::Result<()> {
    terminal.draw(|frame| ui::draw(frame, app))?;

    loop {
        if app.should_auto_think() {
            spawn_generation(llm, app, tx, None);
        }

        match rx.recv().await {
            Some(AppEvent::Terminal(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                handle_key(app, key, llm, tx);
                if app.should_quit {
                    break;
                }
            }
            Some(AppEvent::Terminal(Event::Resize(_, _))) => {}
            Some(AppEvent::Terminal(_)) => {}
            Some(AppEvent::SystemTick(info)) => {
                app.system_info = info;
            }
            Some(AppEvent::Token(token)) => {
                app.append_token(&token);
            }
            Some(AppEvent::GenerationDone) => {
                app.finish_ai_message();
                // Trigger canvas generation after each completed thought/response
                spawn_canvas_generation(llm, app, tx);
            }
            Some(AppEvent::GenerationError(err)) => {
                app.handle_generation_error(err);
            }
            Some(AppEvent::AnimationTick) => {
                app.pet_frame_index = app.pet_frame_index.wrapping_add(1);
            }
            Some(AppEvent::CanvasToken(token)) => {
                if app.canvas_generating {
                    app.canvas_buffer.push_str(&token);
                    let raw: Vec<String> = app.canvas_buffer.lines().map(String::from).collect();
                    let target_h = app.canvas_height as usize;
                    // Check if buffer ends with a newline (meaning current line is complete)
                    let have_complete_lines = app.canvas_buffer.ends_with('\n');
                    let complete_line_count = if have_complete_lines {
                        raw.len()
                    } else {
                        raw.len().saturating_sub(1)
                    };
                    app.canvas_lines = fit_canvas(raw, app.canvas_width, app.canvas_height);
                    // Cut off early once we have enough complete lines
                    if complete_line_count >= target_h {
                        app.canvas_buffer.clear();
                        app.canvas_generating = false;
                        // Abort the background task to free the Ollama connection
                        if let Some(handle) = app.canvas_task.take() {
                            handle.abort();
                        }
                    }
                }
            }
            Some(AppEvent::CanvasDone) => {
                if app.canvas_generating {
                    let raw: Vec<String> = app.canvas_buffer.lines().map(String::from).collect();
                    app.canvas_lines = fit_canvas(raw, app.canvas_width, app.canvas_height);
                    app.canvas_buffer.clear();
                    app.canvas_generating = false;
                    app.canvas_task.take();
                }
            }
            Some(AppEvent::Shutdown) => {
                tracing::info!("shutdown signal received");
                app.log_shutdown();
                app.config.save();
                break;
            }
            None => break,
        }

        terminal.draw(|frame| ui::draw(frame, app))?;
    }

    Ok(())
}

/// Dispatches a single key press to the appropriate [`App`] method.
fn handle_key(
    app: &mut App,
    key: KeyEvent,
    llm: &Arc<dyn LlmClient>,
    tx: &mpsc::UnboundedSender<AppEvent>,
) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.log_shutdown();
        app.should_quit = true;
        return;
    }

    if app.mode == AppMode::Config {
        handle_config_key(app, key);
        return;
    }

    match key.code {
        KeyCode::Enter => {
            let result = app.submit_input();
            match result {
                HandleResult::GenerateResponse(text) => {
                    spawn_generation(llm, app, tx, Some(text));
                }
                HandleResult::RunUpdate => {
                    spawn_update(tx.clone());
                }
                HandleResult::ForceThink => {
                    spawn_generation(llm, app, tx, None);
                }
                HandleResult::RegenCanvas => {
                    spawn_canvas_generation(llm, app, tx);
                }
                _ => {}
            }
        }
        KeyCode::Char(c) => {
            app.insert_char(c);
        }
        KeyCode::Backspace => {
            app.delete_char_before_cursor();
        }
        KeyCode::Left => {
            app.move_cursor_left();
        }
        KeyCode::Right => {
            app.move_cursor_right();
        }
        KeyCode::Home => {
            app.input_cursor = 0;
        }
        KeyCode::End => {
            app.input_cursor = app.input_buffer.len();
        }
        KeyCode::Up => {
            app.history_up();
        }
        KeyCode::Down => {
            app.history_down();
        }
        KeyCode::PageUp => {
            let current = app.manual_scroll.unwrap_or(u16::MAX);
            app.manual_scroll = Some(current.saturating_sub(5));
        }
        KeyCode::PageDown => {
            if let Some(offset) = app.manual_scroll {
                app.manual_scroll = Some(offset.saturating_add(5));
            }
        }
        KeyCode::Esc => {
            app.manual_scroll = None;
        }
        _ => {}
    }
}

/// Handles key presses while the config menu is open.
fn handle_config_key(app: &mut App, key: KeyEvent) {
    if app.config_editing {
        match key.code {
            KeyCode::Enter => {
                app.config_apply_edit();
            }
            KeyCode::Esc => {
                app.config_editing = false;
                app.config_edit_buffer.clear();
            }
            KeyCode::Backspace => {
                app.config_edit_buffer.pop();
            }
            KeyCode::Char(c) => {
                app.config_edit_buffer.push(c);
            }
            _ => {}
        }
    } else {
        match key.code {
            KeyCode::Up => app.config_up(),
            KeyCode::Down => app.config_down(),
            KeyCode::Enter => app.config_start_edit(),
            KeyCode::Esc => app.exit_config_mode(),
            _ => {}
        }
    }
}

/// Starts an LLM streaming chat generation in a background task.
///
/// If `user_message` is `Some`, builds a response request; otherwise builds an
/// autonomous thought request. The [`LlmClient`] implementation handles retry
/// logic and timeouts internally.
fn spawn_generation(
    llm: &Arc<dyn LlmClient>,
    app: &mut App,
    tx: &mpsc::UnboundedSender<AppEvent>,
    user_message: Option<String>,
) {
    if app.is_generating || app.canvas_generating {
        return;
    }

    let history_entries = app.history.last_n(10).to_vec();
    let info = app.system_info.clone();
    let model = app.model.clone();
    let sys_prompt = app.config.system_prompt.clone();
    let stats_vis = app.config.stats.clone();

    let request = match &user_message {
        Some(msg) => crate::ollama::build_response_request(
            &info,
            &history_entries,
            msg,
            &model,
            sys_prompt.as_deref(),
            &stats_vis,
        ),
        None => crate::ollama::build_autonomous_request(
            &info,
            &history_entries,
            &model,
            sys_prompt.as_deref(),
            &stats_vis,
        ),
    };

    app.start_ai_message();
    app.last_user_input_time = std::time::Instant::now();

    let delay_min = app.config.think_delay_min_ms;
    let delay_max = app.config.think_delay_max_ms;

    let llm = Arc::clone(llm);
    let tx = tx.clone();

    tokio::spawn(async move {
        // Pause before streaming to simulate thinking
        if delay_max > 0 {
            let ms = if delay_min >= delay_max {
                delay_min
            } else {
                rand::random_range(delay_min..=delay_max)
            };
            tokio::time::sleep(Duration::from_millis(ms)).await;
        }

        match llm.stream_chat(request.clone(), tx.clone()).await {
            Ok(()) => {}
            Err(e) => {
                let err_str = e.to_string();
                // Auto-pull model if not found, then retry
                if err_str.contains("not found") || err_str.contains("pull") {
                    let _ = tx.send(AppEvent::GenerationError(format!(
                        "Model not found, pulling {}...",
                        request.model
                    )));
                    match llm.pull_model(&request.model).await {
                        Ok(()) => {
                            let _ = tx.send(AppEvent::GenerationError(format!(
                                "Model {} pulled, retrying...",
                                request.model
                            )));
                            if let Err(e) = llm.stream_chat(request, tx.clone()).await {
                                let _ = tx.send(AppEvent::GenerationError(format!(
                                    "LLM error: {}",
                                    e
                                )));
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::GenerationError(format!("{}", e)));
                        }
                    }
                } else {
                    let _ = tx.send(AppEvent::GenerationError(format!("LLM error: {}", e)));
                }
            }
        }
    });
}

/// Starts a canvas art generation in a background task.
///
/// Asks the model to generate ASCII art for the canvas panel based on
/// the current system state and the last AI thought. Sends tokens as
/// `CanvasToken` and completion as `CanvasDone`.
fn spawn_canvas_generation(
    llm: &Arc<dyn LlmClient>,
    app: &mut App,
    tx: &mpsc::UnboundedSender<AppEvent>,
) {
    // Don't overlap canvas generations, and need valid dimensions
    if app.canvas_generating || app.canvas_width == 0 || app.canvas_height == 0 {
        return;
    }

    let info = app.system_info.clone();
    let model = app.model.clone();
    let stats_vis = app.config.stats.clone();
    let width = app.canvas_width;
    let height = app.canvas_height;

    // Get the mood from pet_states for context
    let mood = crate::pet_states::PetMood::from_state(
        &app.system_info,
        app.is_generating,
        app.is_user_typing,
    );
    let mood_str = format!("{:?}", mood);

    // Get the last AI message for context
    let last_thought = app
        .chat_messages
        .iter()
        .rev()
        .find(|m| m.role == crate::history::Role::Ai && m.complete)
        .map(|m| m.text.clone());

    let request = crate::ollama::build_canvas_request(
        &info,
        &mood_str,
        last_thought.as_deref(),
        width,
        height,
        &model,
        &stats_vis,
    );

    // Abort any previous canvas task that's still running
    if let Some(handle) = app.canvas_task.take() {
        handle.abort();
    }

    app.canvas_generating = true;
    app.canvas_buffer.clear();
    app.canvas_lines.clear();

    let llm = Arc::clone(llm);
    let tx = tx.clone();

    app.canvas_task = Some(tokio::spawn(async move {
        // Use a wrapper that sends CanvasToken/CanvasDone instead of Token/GenerationDone
        let (inner_tx, mut inner_rx) = mpsc::unbounded_channel::<AppEvent>();

        let stream_handle = tokio::spawn({
            let llm = Arc::clone(&llm);
            async move {
                let _ = llm.stream_chat(request, inner_tx).await;
            }
        });

        // Relay tokens as CanvasToken events
        while let Some(evt) = inner_rx.recv().await {
            match evt {
                AppEvent::Token(token) => {
                    if tx.send(AppEvent::CanvasToken(token)).is_err() {
                        break;
                    }
                }
                AppEvent::GenerationDone => {
                    let _ = tx.send(AppEvent::CanvasDone);
                    break;
                }
                AppEvent::GenerationError(_) => {
                    // Canvas generation failed silently — keep previous canvas
                    let _ = tx.send(AppEvent::CanvasDone);
                    break;
                }
                _ => {}
            }
        }

        let _ = stream_handle.await;
    }));
}

/// Normalizes canvas output to exact panel dimensions.
///
/// - Truncates or pads each line to exactly `width` visible characters
///   (color tags like `{red}` are preserved but not counted).
/// - Truncates or pads line count to exactly `height` lines.
fn fit_canvas(raw: Vec<String>, width: u16, height: u16) -> Vec<String> {
    let w = width as usize;
    let h = height as usize;

    let mut lines: Vec<String> = raw
        .into_iter()
        .take(h)
        .map(|line| fit_line_width(&line, w))
        .collect();

    // Pad with empty lines if too few
    while lines.len() < h {
        lines.push(" ".repeat(w));
    }

    lines
}

/// Truncates or pads a single line to exactly `target_width` visible characters.
/// Color tags (`{red}`, `{/}`, etc.) are not counted toward width.
/// Non-printable and non-ASCII characters are replaced with spaces.
fn fit_line_width(line: &str, target_width: usize) -> String {
    let mut result = String::new();
    let mut visible = 0usize;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if visible >= target_width {
            break;
        }

        if ch == '{' {
            // Try to read a color tag
            let mut tag_content = String::new();
            let mut lookahead = vec![ch]; // include '{'
            let mut found_close = false;
            for _ in 0..10 {
                if let Some(&next) = chars.peek() {
                    lookahead.push(next);
                    chars.next();
                    if next == '}' {
                        found_close = true;
                        break;
                    }
                    tag_content.push(next);
                } else {
                    break;
                }
            }

            if found_close && is_color_tag(&tag_content) {
                // Emit the color tag without counting toward visible width
                for c in &lookahead {
                    result.push(*c);
                }
            } else {
                // Not a valid tag, emit as literal visible characters
                for c in &lookahead {
                    if visible >= target_width {
                        break;
                    }
                    result.push(sanitize_char(*c));
                    visible += 1;
                }
            }
        } else {
            result.push(sanitize_char(ch));
            visible += 1;
        }
    }

    // Pad with spaces if too short
    while visible < target_width {
        result.push(' ');
        visible += 1;
    }

    result
}

/// Returns true if the tag name is a recognized color tag.
fn is_color_tag(tag: &str) -> bool {
    matches!(
        tag.to_lowercase().as_str(),
        "red" | "green" | "blue" | "yellow" | "cyan" | "magenta" | "white"
            | "gray" | "grey" | "/" | "reset"
    )
}

/// Replaces non-printable or wide characters with a space.
///
/// Keeps printable ASCII (0x20..=0x7E) and common box-drawing / block
/// element Unicode chars. Everything else becomes a space to prevent
/// alignment issues or rendering artifacts in the terminal.
fn sanitize_char(ch: char) -> char {
    if ch.is_ascii_graphic() || ch == ' ' {
        return ch;
    }
    // Allow common Unicode box-drawing (U+2500..U+257F) and block elements (U+2580..U+259F)
    let code = ch as u32;
    if (0x2500..=0x257F).contains(&code) || (0x2580..=0x259F).contains(&code) {
        return ch;
    }
    ' '
}

/// Runs `git pull && cargo build --release` in a background task.
fn spawn_update(tx: mpsc::UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        let output = tokio::process::Command::new("bash")
            .args([
                "-c",
                "cd $(dirname $(which trapped-mind 2>/dev/null || echo .)) && cd .. && git pull && cargo build --release 2>&1",
            ])
            .output()
            .await;

        match output {
            Ok(out) => {
                let msg = String::from_utf8_lossy(&out.stdout).to_string()
                    + &String::from_utf8_lossy(&out.stderr);
                let _ = tx.send(AppEvent::GenerationError(format!(
                    "Update output:\n{}",
                    msg
                )));
            }
            Err(e) => {
                let _ = tx.send(AppEvent::GenerationError(format!("Update failed: {}", e)));
            }
        }
    });
}
