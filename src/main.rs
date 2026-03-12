//! Application entry point and async event loop.
//!
//! Sets up the terminal, spawns background tasks for system polling, terminal
//! event reading, and animation ticking, then runs the main event loop that
//! dispatches [`AppEvent`]s to the [`App`] state machine.

mod app;
mod config;
mod history;
mod ollama;
mod pet_states;
mod system;
mod ui;

use app::{App, AppEvent, AppMode, HandleResult};
use config::{AppConfig, CliArgs};
use system::SystemReader;

use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ollama_rs::Ollama;
use ratatui::DefaultTerminal;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let cli = CliArgs::parse();
    let config = AppConfig::load(&cli);
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

    let ollama = Ollama::new(&config.ollama_host, config.ollama_port);

    let mut terminal = ratatui::init();
    let result = run_app(&mut terminal, &mut app, &mut rx, &tx, &ollama).await;
    ratatui::restore();
    result
}

/// Main event loop — draws the UI and dispatches events until the app exits.
async fn run_app(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    rx: &mut mpsc::UnboundedReceiver<AppEvent>,
    tx: &mpsc::UnboundedSender<AppEvent>,
    ollama: &Ollama,
) -> std::io::Result<()> {
    terminal.draw(|frame| ui::draw(frame, app))?;

    loop {
        if app.should_auto_think() {
            spawn_generation(ollama, app, tx, None);
        }

        match rx.recv().await {
            Some(AppEvent::Terminal(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                handle_key(app, key, ollama, tx);
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
            }
            Some(AppEvent::GenerationError(err)) => {
                app.handle_generation_error(err);
            }
            Some(AppEvent::AnimationTick) => {
                app.pet_frame_index = app.pet_frame_index.wrapping_add(1);
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
    ollama: &Ollama,
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
                    spawn_generation(ollama, app, tx, Some(text));
                }
                HandleResult::RunUpdate => {
                    spawn_update(tx.clone());
                }
                HandleResult::ForceThink => {
                    spawn_generation(ollama, app, tx, None);
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
        // Editing a field value
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
        // Navigating the menu
        match key.code {
            KeyCode::Up => app.config_up(),
            KeyCode::Down => app.config_down(),
            KeyCode::Enter => app.config_start_edit(),
            KeyCode::Esc => app.exit_config_mode(),
            _ => {}
        }
    }
}

/// Streams a chat request and sends tokens back through the event channel.
/// Returns `Ok(())` on success, or the Ollama error if the request fails.
async fn stream_chat(
    ollama: &Ollama,
    request: ollama_rs::generation::chat::request::ChatMessageRequest,
    tx: &mpsc::UnboundedSender<AppEvent>,
) -> Result<(), ollama_rs::error::OllamaError> {
    let mut stream = ollama.send_chat_messages_stream(request).await?;
    while let Some(res) = stream.next().await {
        match res {
            Ok(resp) => {
                let token = resp.message.content;
                if !token.is_empty() {
                    if tx.send(AppEvent::Token(token)).is_err() {
                        return Ok(());
                    }
                }
                if resp.done {
                    let _ = tx.send(AppEvent::GenerationDone);
                    return Ok(());
                }
            }
            Err(_) => {
                let _ = tx.send(AppEvent::GenerationError("Stream error".to_string()));
                return Ok(());
            }
        }
    }
    let _ = tx.send(AppEvent::GenerationDone);
    Ok(())
}

/// Starts an Ollama streaming chat generation in a background task.
///
/// If `user_message` is `Some`, builds a response request; otherwise builds an
/// autonomous thought request. Uses the chat API with proper role-tagged
/// messages so the model sees its own previous responses as assistant messages.
fn spawn_generation(
    ollama: &Ollama,
    app: &mut App,
    tx: &mpsc::UnboundedSender<AppEvent>,
    user_message: Option<String>,
) {
    if app.is_generating {
        return;
    }

    let history_entries = app.history.last_n(10).to_vec();
    let info = app.system_info.clone();
    let model = app.model.clone();
    let sys_prompt = app.config.system_prompt.clone();
    let stats_vis = app.config.stats.clone();

    let request = match &user_message {
        Some(msg) => crate::ollama::build_response_request(&info, &history_entries, msg, &model, sys_prompt.as_deref(), &stats_vis),
        None => crate::ollama::build_autonomous_request(&info, &history_entries, &model, sys_prompt.as_deref(), &stats_vis),
    };

    app.start_ai_message();
    app.last_user_input_time = std::time::Instant::now();

    let delay_min = app.config.think_delay_min_ms;
    let delay_max = app.config.think_delay_max_ms;

    let ollama = ollama.clone();
    let tx = tx.clone();

    tokio::spawn(async move {
        // Pause before streaming to simulate thinking
        if delay_max > 0 {
            let range = delay_min..=delay_max;
            let ms = if delay_min >= delay_max {
                delay_min
            } else {
                rand::random_range(range)
            };
            tokio::time::sleep(Duration::from_millis(ms)).await;
        }

        if let Err(e) = stream_chat(&ollama, request.clone(), &tx).await {
            let err_str = e.to_string();
            // Auto-pull model if not found, then retry
            if err_str.contains("not found") || err_str.contains("pull") {
                let _ = tx.send(AppEvent::GenerationError(format!(
                    "Model not found, pulling {}...", request.model_name
                )));
                match ollama.pull_model(request.model_name.clone(), false).await {
                    Ok(_) => {
                        let _ = tx.send(AppEvent::GenerationError(format!(
                            "Model {} pulled successfully, retrying...", request.model_name
                        )));
                        if let Err(e) = stream_chat(&ollama, request, &tx).await {
                            let _ = tx.send(AppEvent::GenerationError(format!("Ollama error: {}", e)));
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::GenerationError(format!("Failed to pull model: {}", e)));
                    }
                }
            } else {
                let _ = tx.send(AppEvent::GenerationError(format!("Ollama error: {}", e)));
            }
        }
    });
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
