//! Core application state and business logic.
//!
//! The [`App`] struct owns all mutable state: chat messages, input buffer,
//! generation status, and system info. It exposes methods for handling user
//! input, managing chat history, and driving the command system.

use crate::config::AppConfig;
use crate::history::{HistoryEntry, HistoryManager, Role};
use crate::ollama::{self, Command};
use crate::system::SystemInfo;

use chrono::Local;
use std::time::Instant;

/// Which screen the app is currently showing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    /// Normal chat view.
    Normal,
    /// Configuration menu overlay.
    Config,
}

/// A configurable field in the config menu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfigField {
    Model,
    OllamaHost,
    OllamaPort,
    MaxHistory,
    AutoThinkDelay,
    ThinkDelayMin,
    ThinkDelayMax,
    SystemPrompt,
    ShowCpu,
    ShowTemp,
    ShowRam,
    ShowBattery,
    ShowFan,
    ShowUptime,
    ShowNetwork,
}

impl ConfigField {
    /// All fields in display order.
    pub const ALL: &[ConfigField] = &[
        ConfigField::Model,
        ConfigField::OllamaHost,
        ConfigField::OllamaPort,
        ConfigField::MaxHistory,
        ConfigField::AutoThinkDelay,
        ConfigField::ThinkDelayMin,
        ConfigField::ThinkDelayMax,
        ConfigField::SystemPrompt,
        ConfigField::ShowCpu,
        ConfigField::ShowTemp,
        ConfigField::ShowRam,
        ConfigField::ShowBattery,
        ConfigField::ShowFan,
        ConfigField::ShowUptime,
        ConfigField::ShowNetwork,
    ];

    /// Display label for this field.
    pub fn label(&self) -> &str {
        match self {
            ConfigField::Model => "Model",
            ConfigField::OllamaHost => "Ollama Host",
            ConfigField::OllamaPort => "Ollama Port",
            ConfigField::MaxHistory => "Max History",
            ConfigField::AutoThinkDelay => "Auto-think Delay (s)",
            ConfigField::ThinkDelayMin => "Think Pause Min (ms)",
            ConfigField::ThinkDelayMax => "Think Pause Max (ms)",
            ConfigField::SystemPrompt => "System Prompt",
            ConfigField::ShowCpu => "Show CPU",
            ConfigField::ShowTemp => "Show Temperature",
            ConfigField::ShowRam => "Show RAM",
            ConfigField::ShowBattery => "Show Battery",
            ConfigField::ShowFan => "Show Fan",
            ConfigField::ShowUptime => "Show Uptime",
            ConfigField::ShowNetwork => "Show Network",
        }
    }

    /// Returns `true` if this field is a boolean toggle (Enter flips it).
    pub fn is_toggle(&self) -> bool {
        matches!(self,
            ConfigField::ShowCpu | ConfigField::ShowTemp | ConfigField::ShowRam |
            ConfigField::ShowBattery | ConfigField::ShowFan | ConfigField::ShowUptime |
            ConfigField::ShowNetwork
        )
    }
}

/// Events flowing through the main event loop's unified channel.
pub enum AppEvent {
    /// A crossterm terminal event (key press, resize, etc.).
    Terminal(crossterm::event::Event),
    /// Fresh system metrics from the polling thread.
    SystemTick(SystemInfo),
    /// A single token from an in-progress Ollama generation.
    Token(String),
    /// The current generation completed successfully.
    GenerationDone,
    /// The current generation failed with an error message.
    GenerationError(String),
    /// The pet animation timer fired.
    AnimationTick,
    /// A single token from an in-progress canvas generation.
    CanvasToken(String),
    /// The canvas generation completed successfully.
    CanvasDone,
    /// Graceful shutdown requested (Ctrl+C / SIGTERM).
    Shutdown,
    /// A tool produced a chat token.
    ToolChatToken(String),
    /// A tool produced canvas content (full accumulated buffer).
    ToolCanvasContent(String),
    /// A tool produced a status message.
    ToolStatus(String),
    /// The decision phase of a tool cycle completed (finishes the "Thinking:" message).
    ToolDecisionDone,
    /// The current tool cycle completed with a summary.
    ToolCycleDone(String),
    /// The current tool cycle failed.
    ToolCycleError(String),
}

/// What action the caller should take after [`App::submit_input`] or
/// [`App::handle_command`] returns.
#[derive(Debug)]
pub enum HandleResult {
    /// No further action needed.
    Nothing,
    /// Start an Ollama generation to respond to the given user text.
    GenerateResponse(String),
    /// Run the self-update script (`git pull && cargo build`).
    RunUpdate,
    /// Trigger an autonomous thought immediately.
    ForceThink,
    /// Regenerate the canvas art.
    RegenCanvas,
}

/// A single message displayed in the chat panel.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    /// Who sent this message.
    pub role: Role,
    /// The message text (may be partially streamed if `complete` is false).
    pub text: String,
    /// Whether the message has finished streaming.
    pub complete: bool,
    /// When this message was created (local time, formatted for display).
    pub timestamp: String,
}

/// Root application state for the TUI.
pub struct App {
    /// Loaded configuration (Ollama endpoint, model, history settings).
    pub config: AppConfig,
    /// Persistent conversation history (saved to disk as JSONL).
    pub history: HistoryManager,
    /// Latest system metrics snapshot.
    pub system_info: SystemInfo,
    /// All messages shown in the chat panel.
    pub chat_messages: Vec<ChatMessage>,
    /// Current text in the input bar.
    pub input_buffer: String,
    /// Byte offset of the cursor within `input_buffer`.
    pub input_cursor: usize,
    /// If `Some`, the chat panel is manually scrolled to this line offset.
    pub manual_scroll: Option<u16>,
    /// Whether an Ollama generation is currently in progress.
    pub is_generating: bool,
    /// Whether the user is actively typing (for pet mood).
    pub is_user_typing: bool,
    /// Set to `true` to exit the event loop.
    pub should_quit: bool,
    /// Animation frame counter for the pet face (wraps on overflow).
    pub pet_frame_index: usize,
    /// Lines of ASCII art currently displayed in the canvas panel.
    pub canvas_lines: Vec<String>,
    /// Buffer accumulating tokens during canvas generation.
    pub canvas_buffer: String,
    /// Whether a canvas generation is currently in progress.
    pub canvas_generating: bool,
    /// Handle to the running canvas task, so it can be aborted early.
    pub canvas_task: Option<tokio::task::JoinHandle<()>>,
    /// Summaries of recent tool executions (for decision model context).
    pub tool_history: Vec<String>,
    /// Whether a tool cycle is currently in progress.
    pub tool_active: bool,
    /// Monotonic turn counter for tool cycles.
    pub tool_turn: u32,
    /// Last known inner dimensions of the canvas panel.
    pub canvas_width: u16,
    /// Last known inner height of the canvas panel.
    pub canvas_height: u16,
    /// Timestamp of the last user interaction (for auto-think delay).
    pub last_user_input_time: Instant,
    /// Currently active Ollama model name.
    pub model: String,
    /// Input history for Up/Down arrow recall.
    pub command_history: Vec<String>,
    /// Current position in `command_history` (`None` = not browsing).
    pub command_history_index: Option<usize>,
    /// Which screen mode is active.
    pub mode: AppMode,
    /// Which config field is currently selected.
    pub config_selected: usize,
    /// Whether a config field is being edited (input bar repurposed).
    pub config_editing: bool,
    /// Temporary edit buffer for config values.
    pub config_edit_buffer: String,
}

impl App {
    /// Creates a new `App` with the given configuration, loading any persisted
    /// history from disk.
    pub fn new(config: AppConfig) -> Self {
        let history = HistoryManager::new(config.history_path.clone(), config.max_history);
        let model = config.model.clone();

        let chat_messages: Vec<ChatMessage> = history
            .entries()
            .iter()
            .map(|e| {
                // Parse stored RFC3339 timestamp to local time for display
                let display_time = chrono::DateTime::parse_from_rfc3339(&e.timestamp)
                    .map(|dt| dt.with_timezone(&Local).format("%H:%M:%S").to_string())
                    .unwrap_or_default();
                ChatMessage {
                    role: e.role.clone(),
                    text: e.text.clone(),
                    complete: true,
                    timestamp: display_time,
                }
            })
            .collect();

        Self {
            config, history, chat_messages,
            system_info: SystemInfo {
                cpu_percent: 0.0, temp_celsius: 0.0,
                ram_used_bytes: 0, ram_total_bytes: 0,
                battery_percent: 0.0, power_status: "Unknown".to_string(),
                fan_rpm: 0, uptime_secs: 0, networks: vec![],
                cpu_real: true, temp_real: false, ram_real: true,
                battery_real: false, fan_real: false, network_real: false,
            },
            input_buffer: String::new(),
            input_cursor: 0,
            manual_scroll: None,
            is_generating: false,
            is_user_typing: false,
            should_quit: false,
            pet_frame_index: 0,
            canvas_lines: Vec::new(),
            canvas_buffer: String::new(),
            canvas_generating: false,
            canvas_task: None,
            tool_history: Vec::new(),
            tool_active: false,
            tool_turn: 0,
            canvas_width: 0,
            canvas_height: 0,
            last_user_input_time: Instant::now(),
            model,
            command_history: Vec::new(),
            command_history_index: None,
            mode: AppMode::Normal,
            config_selected: 0,
            config_editing: false,
            config_edit_buffer: String::new(),
        }
    }

    /// Returns the current local time formatted for display.
    fn now_display() -> String {
        Local::now().format("%H:%M:%S").to_string()
    }

    /// Appends a system-level message (e.g. sensor status, errors) to the chat.
    pub fn add_system_message(&mut self, text: String) {
        self.chat_messages.push(ChatMessage {
            role: Role::System, text, complete: true,
            timestamp: Self::now_display(),
        });
    }

    /// Appends a system-level message to both the chat display and persistent history,
    /// so the AI can see it after a restart.
    pub fn add_persistent_system_message(&mut self, text: String) {
        self.history.append(HistoryEntry::new(Role::System, text.clone()));
        self.chat_messages.push(ChatMessage {
            role: Role::System, text, complete: true,
            timestamp: Self::now_display(),
        });
    }

    /// Logs a startup timestamp to persistent history.
    pub fn log_startup(&mut self) {
        let now = Local::now().format("%Y-%m-%d %H:%M:%S");
        self.add_persistent_system_message(format!("[session started at {}]", now));
    }

    /// Logs a shutdown timestamp to persistent history.
    pub fn log_shutdown(&mut self) {
        let now = Local::now().format("%Y-%m-%d %H:%M:%S");
        self.add_persistent_system_message(format!("[session ended at {}]", now));
    }

    /// Records a user message in both the chat display and persistent history.
    pub fn add_user_message(&mut self, text: String) {
        self.history.append(HistoryEntry::new(Role::User, text.clone()));
        self.chat_messages.push(ChatMessage {
            role: Role::User, text, complete: true,
            timestamp: Self::now_display(),
        });
    }

    /// Begins a new AI message placeholder for token-by-token streaming.
    pub fn start_ai_message(&mut self) {
        self.chat_messages.push(ChatMessage {
            role: Role::Ai, text: String::new(), complete: false,
            timestamp: Self::now_display(),
        });
        self.is_generating = true;
    }

    /// Appends a streamed token to the current in-progress AI message.
    pub fn append_token(&mut self, token: &str) {
        if let Some(last) = self.chat_messages.last_mut() {
            if !last.complete { last.text.push_str(token); }
        }
    }

    /// Marks the current AI message as complete and saves it to history.
    pub fn finish_ai_message(&mut self) {
        if let Some(last) = self.chat_messages.last_mut() {
            if !last.complete {
                last.complete = true;
                self.history.append(HistoryEntry::new(Role::Ai, last.text.clone()));
            }
        }
        self.is_generating = false;
    }

    /// Handles a generation error: removes the empty placeholder (if any)
    /// and shows the error as a system message.
    pub fn handle_generation_error(&mut self, error: String) {
        if let Some(last) = self.chat_messages.last() {
            if !last.complete && last.text.is_empty() {
                self.chat_messages.pop();
            }
        }
        self.is_generating = false;
        self.add_system_message(format!("[error] {}", error));
    }

    /// Records a tool execution summary, keeping the last 5.
    pub fn log_tool_use(&mut self, summary: String) {
        self.tool_history.push(summary);
        if self.tool_history.len() > 5 {
            self.tool_history.remove(0);
        }
    }

    /// Returns `true` if enough idle time has elapsed to trigger an autonomous thought.
    /// Disabled while in config mode or while canvas is generating.
    pub fn should_auto_think(&self) -> bool {
        self.mode == AppMode::Normal
            && !self.is_generating
            && !self.canvas_generating
            && !self.tool_active
            && self.last_user_input_time.elapsed().as_secs() >= self.config.auto_think_delay_secs
    }

    /// Parses and executes a command or chat message, returning the action
    /// the caller should take (e.g. start a generation, run an update).
    pub fn handle_command(&mut self, input: &str) -> HandleResult {
        let cmd = ollama::parse_input(input);
        match cmd {
            Command::Quit => {
                self.log_shutdown();
                self.should_quit = true;
                HandleResult::Nothing
            }
            Command::Help => {
                self.add_system_message(
                    "Commands:\n  /help   - Show this help\n  /clear  - Clear memory\n  /model <name> - Switch model\n  /stats  - Show system info\n  /think  - Force a thought now\n  /canvas - Regenerate canvas art\n  /config - Open config menu\n  /update - Pull & rebuild\n  /quit   - Exit\n  /exit   - Exit".to_string(),
                );
                HandleResult::Nothing
            }
            Command::Clear => {
                self.history.clear();
                self.chat_messages.clear();
                self.add_system_message("Memory cleared.".to_string());
                HandleResult::Nothing
            }
            Command::Stats => {
                let info = &self.system_info;
                let stats = format!(
                    "CPU: {:.1}%\nTemp: {:.0}°C\nRAM: {:.1}G/{:.1}G\nBattery: {:.0}% ({})\nFan: {} RPM\nUptime: {}\nNetworks:\n{}",
                    info.cpu_percent, info.temp_celsius,
                    info.ram_used_gb(), info.ram_total_gb(),
                    info.battery_percent, info.power_status,
                    info.fan_rpm, info.uptime_formatted(),
                    info.networks.iter().map(|n| format!("  {}: {}", n.name, n.ip)).collect::<Vec<_>>().join("\n"),
                );
                self.add_system_message(stats);
                HandleResult::Nothing
            }
            Command::Model(name) => {
                if name.is_empty() {
                    self.add_system_message(format!("Current model: {}", self.model));
                } else {
                    self.model = name.clone();
                    self.config.model = name.clone();
                    self.add_system_message(format!("Switched to model: {}", name));
                }
                HandleResult::Nothing
            }
            Command::Think => {
                HandleResult::ForceThink
            }
            Command::Canvas => {
                HandleResult::RegenCanvas
            }
            Command::Config => {
                self.enter_config_mode();
                HandleResult::Nothing
            }
            Command::Update => {
                self.add_system_message("Running update...".to_string());
                HandleResult::RunUpdate
            }
            Command::Message(text) => {
                if text.is_empty() { return HandleResult::Nothing; }
                self.add_user_message(text.clone());
                HandleResult::GenerateResponse(text)
            }
        }
    }

    /// Inserts a character at the current cursor position.
    pub fn insert_char(&mut self, c: char) {
        self.input_buffer.insert(self.input_cursor, c);
        self.input_cursor += c.len_utf8();
        self.is_user_typing = true;
        self.last_user_input_time = Instant::now();
    }

    /// Deletes the character immediately before the cursor.
    pub fn delete_char_before_cursor(&mut self) {
        if self.input_cursor > 0 {
            let prev = self.input_buffer[..self.input_cursor]
                .char_indices().last().map(|(i, _)| i).unwrap_or(0);
            self.input_buffer.remove(prev);
            self.input_cursor = prev;
        }
    }

    /// Moves the cursor one character to the left (respects multi-byte chars).
    pub fn move_cursor_left(&mut self) {
        if self.input_cursor > 0 {
            self.input_cursor = self.input_buffer[..self.input_cursor]
                .char_indices().last().map(|(i, _)| i).unwrap_or(0);
        }
    }

    /// Moves the cursor one character to the right (respects multi-byte chars).
    pub fn move_cursor_right(&mut self) {
        if self.input_cursor < self.input_buffer.len() {
            self.input_cursor = self.input_buffer[self.input_cursor..]
                .char_indices().nth(1)
                .map(|(i, _)| self.input_cursor + i)
                .unwrap_or(self.input_buffer.len());
        }
    }

    /// Submits the current input buffer, adding it to command history and
    /// dispatching it through [`handle_command`](Self::handle_command).
    pub fn submit_input(&mut self) -> HandleResult {
        if self.input_buffer.trim().is_empty() { return HandleResult::Nothing; }
        let input = self.input_buffer.clone();
        self.command_history.push(input.clone());
        self.command_history_index = None;
        self.input_buffer.clear();
        self.input_cursor = 0;
        self.is_user_typing = false;
        self.handle_command(&input)
    }

    /// Recalls the previous entry from command history (Up arrow).
    pub fn history_up(&mut self) {
        if self.command_history.is_empty() { return; }
        let idx = match self.command_history_index {
            Some(i) if i > 0 => i - 1,
            Some(i) => i,
            None => self.command_history.len() - 1,
        };
        self.command_history_index = Some(idx);
        self.input_buffer = self.command_history[idx].clone();
        self.input_cursor = self.input_buffer.len();
    }

    // -- Config mode methods --

    /// Enters config mode, resetting selection state.
    pub fn enter_config_mode(&mut self) {
        self.mode = AppMode::Config;
        self.config_selected = 0;
        self.config_editing = false;
        self.config_edit_buffer.clear();
    }

    /// Exits config mode, applying and saving any pending changes.
    pub fn exit_config_mode(&mut self) {
        if self.config_editing {
            self.config_apply_edit();
        }
        self.config.save();
        // Sync runtime state with config
        self.model = self.config.model.clone();
        self.mode = AppMode::Normal;
        self.add_system_message("Config saved.".to_string());
    }

    /// Returns the current display value for the selected config field.
    pub fn config_field_value(&self, field: ConfigField) -> String {
        match field {
            ConfigField::Model => self.config.model.clone(),
            ConfigField::OllamaHost => self.config.ollama_host.clone(),
            ConfigField::OllamaPort => self.config.ollama_port.to_string(),
            ConfigField::MaxHistory => self.config.max_history.to_string(),
            ConfigField::AutoThinkDelay => self.config.auto_think_delay_secs.to_string(),
            ConfigField::ThinkDelayMin => self.config.think_delay_min_ms.to_string(),
            ConfigField::ThinkDelayMax => self.config.think_delay_max_ms.to_string(),
            ConfigField::SystemPrompt => self.config.system_prompt.clone()
                .unwrap_or_else(|| "(default)".to_string()),
            ConfigField::ShowCpu => if self.config.stats.cpu { "ON" } else { "OFF" }.to_string(),
            ConfigField::ShowTemp => if self.config.stats.temperature { "ON" } else { "OFF" }.to_string(),
            ConfigField::ShowRam => if self.config.stats.ram { "ON" } else { "OFF" }.to_string(),
            ConfigField::ShowBattery => if self.config.stats.battery { "ON" } else { "OFF" }.to_string(),
            ConfigField::ShowFan => if self.config.stats.fan { "ON" } else { "OFF" }.to_string(),
            ConfigField::ShowUptime => if self.config.stats.uptime { "ON" } else { "OFF" }.to_string(),
            ConfigField::ShowNetwork => if self.config.stats.network { "ON" } else { "OFF" }.to_string(),
        }
    }

    /// Starts editing the currently selected config field, or toggles it if boolean.
    pub fn config_start_edit(&mut self) {
        let field = ConfigField::ALL[self.config_selected];

        if field.is_toggle() {
            self.config_toggle(field);
            return;
        }

        self.config_edit_buffer = match field {
            ConfigField::SystemPrompt => {
                self.config.system_prompt.clone().unwrap_or_default()
            }
            _ => self.config_field_value(field),
        };
        // Don't put "(default)" in the edit buffer
        if self.config_edit_buffer == "(default)" {
            self.config_edit_buffer.clear();
        }
        self.config_editing = true;
    }

    /// Toggles a boolean config field.
    fn config_toggle(&mut self, field: ConfigField) {
        match field {
            ConfigField::ShowCpu => self.config.stats.cpu = !self.config.stats.cpu,
            ConfigField::ShowTemp => self.config.stats.temperature = !self.config.stats.temperature,
            ConfigField::ShowRam => self.config.stats.ram = !self.config.stats.ram,
            ConfigField::ShowBattery => self.config.stats.battery = !self.config.stats.battery,
            ConfigField::ShowFan => self.config.stats.fan = !self.config.stats.fan,
            ConfigField::ShowUptime => self.config.stats.uptime = !self.config.stats.uptime,
            ConfigField::ShowNetwork => self.config.stats.network = !self.config.stats.network,
            _ => {}
        }
    }

    /// Applies the current edit buffer to the selected config field.
    pub fn config_apply_edit(&mut self) {
        let field = ConfigField::ALL[self.config_selected];
        let val = self.config_edit_buffer.trim().to_string();
        match field {
            ConfigField::Model => {
                if !val.is_empty() { self.config.model = val; }
            }
            ConfigField::OllamaHost => {
                if !val.is_empty() { self.config.ollama_host = val; }
            }
            ConfigField::OllamaPort => {
                if let Ok(port) = val.parse::<u16>() { self.config.ollama_port = port; }
            }
            ConfigField::MaxHistory => {
                if let Ok(n) = val.parse::<usize>() { self.config.max_history = n; }
            }
            ConfigField::AutoThinkDelay => {
                if let Ok(n) = val.parse::<u64>() { self.config.auto_think_delay_secs = n; }
            }
            ConfigField::ThinkDelayMin => {
                if let Ok(n) = val.parse::<u64>() { self.config.think_delay_min_ms = n; }
            }
            ConfigField::ThinkDelayMax => {
                if let Ok(n) = val.parse::<u64>() { self.config.think_delay_max_ms = n; }
            }
            ConfigField::SystemPrompt => {
                self.config.system_prompt = if val.is_empty() { None } else { Some(val) };
            }
            _ => {} // Toggles are handled by config_toggle
        }
        self.config_editing = false;
        self.config_edit_buffer.clear();
    }

    /// Moves config selection up.
    pub fn config_up(&mut self) {
        if self.config_selected > 0 {
            self.config_selected -= 1;
        }
    }

    /// Moves config selection down.
    pub fn config_down(&mut self) {
        if self.config_selected + 1 < ConfigField::ALL.len() {
            self.config_selected += 1;
        }
    }

    /// Recalls the next entry from command history (Down arrow), or clears
    /// the input if at the end of the history.
    pub fn history_down(&mut self) {
        match self.command_history_index {
            Some(i) if i + 1 < self.command_history.len() => {
                let idx = i + 1;
                self.command_history_index = Some(idx);
                self.input_buffer = self.command_history[idx].clone();
                self.input_cursor = self.input_buffer.len();
            }
            Some(_) => {
                self.command_history_index = None;
                self.input_buffer.clear();
                self.input_cursor = 0;
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> App {
        App::new(AppConfig::default())
    }

    #[test]
    fn test_input_insertion() {
        let mut app = test_app();
        app.insert_char('h');
        app.insert_char('i');
        assert_eq!(app.input_buffer, "hi");
        assert_eq!(app.input_cursor, 2);
    }

    #[test]
    fn test_backspace() {
        let mut app = test_app();
        app.insert_char('a');
        app.insert_char('b');
        app.delete_char_before_cursor();
        assert_eq!(app.input_buffer, "a");
        assert_eq!(app.input_cursor, 1);
    }

    #[test]
    fn test_cursor_movement() {
        let mut app = test_app();
        app.insert_char('a');
        app.insert_char('b');
        app.insert_char('c');
        app.move_cursor_left();
        assert_eq!(app.input_cursor, 2);
        app.move_cursor_right();
        assert_eq!(app.input_cursor, 3);
    }

    #[test]
    fn test_submit_clears_input() {
        let mut app = test_app();
        app.insert_char('h');
        app.insert_char('i');
        let _ = app.submit_input();
        assert!(app.input_buffer.is_empty());
        assert_eq!(app.input_cursor, 0);
    }

    #[test]
    fn test_help_command() {
        let mut app = test_app();
        app.input_buffer = "/help".to_string();
        app.input_cursor = 5;
        let result = app.submit_input();
        assert!(matches!(result, HandleResult::Nothing));
        assert!(app.chat_messages.last().unwrap().text.contains("Commands:"));
    }

    #[test]
    fn test_quit_command() {
        let mut app = test_app();
        app.input_buffer = "/quit".to_string();
        app.input_cursor = 5;
        let _ = app.submit_input();
        assert!(app.should_quit);
    }

    #[test]
    fn test_clear_command() {
        let mut app = test_app();
        app.add_user_message("test".to_string());
        app.input_buffer = "/clear".to_string();
        app.input_cursor = 6;
        let _ = app.submit_input();
        assert_eq!(app.chat_messages.len(), 1);
        assert!(app.chat_messages[0].text.contains("cleared"));
    }

    #[test]
    fn test_model_switch() {
        let mut app = test_app();
        app.input_buffer = "/model qwen2.5:7b".to_string();
        app.input_cursor = app.input_buffer.len();
        let _ = app.submit_input();
        assert_eq!(app.model, "qwen2.5:7b");
    }

    #[test]
    fn test_message_generates_response() {
        let mut app = test_app();
        app.input_buffer = "hello".to_string();
        app.input_cursor = 5;
        let result = app.submit_input();
        assert!(matches!(result, HandleResult::GenerateResponse(ref s) if s == "hello"));
    }

    #[test]
    fn test_token_streaming() {
        let mut app = test_app();
        app.start_ai_message();
        app.append_token("Hello ");
        app.append_token("world");
        assert_eq!(app.chat_messages.last().unwrap().text, "Hello world");
        assert!(!app.chat_messages.last().unwrap().complete);
        app.finish_ai_message();
        assert!(app.chat_messages.last().unwrap().complete);
        assert!(!app.is_generating);
    }

    #[test]
    fn test_command_history() {
        let mut app = test_app();
        app.input_buffer = "first".to_string();
        app.input_cursor = 5;
        let _ = app.submit_input();
        app.input_buffer = "second".to_string();
        app.input_cursor = 6;
        let _ = app.submit_input();
        app.history_up();
        assert_eq!(app.input_buffer, "second");
        app.history_up();
        assert_eq!(app.input_buffer, "first");
        app.history_down();
        assert_eq!(app.input_buffer, "second");
        app.history_down();
        assert!(app.input_buffer.is_empty());
    }
}
