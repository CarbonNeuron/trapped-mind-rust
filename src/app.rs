use crate::config::AppConfig;
use crate::history::{HistoryEntry, HistoryManager, Role};
use crate::ollama::{self, Command};
use crate::system::SystemInfo;

use std::time::Instant;

pub enum AppEvent {
    Terminal(crossterm::event::Event),
    SystemTick(SystemInfo),
    Token(String),
    GenerationDone,
    GenerationError(String),
    AnimationTick,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: Role,
    pub text: String,
    pub complete: bool,
}

pub struct App {
    pub config: AppConfig,
    pub history: HistoryManager,
    pub system_info: SystemInfo,
    pub chat_messages: Vec<ChatMessage>,
    pub input_buffer: String,
    pub input_cursor: usize,
    pub manual_scroll: Option<u16>,
    pub is_generating: bool,
    pub is_user_typing: bool,
    pub should_quit: bool,
    pub pet_frame_index: usize,
    pub last_user_input_time: Instant,
    pub model: String,
    pub command_history: Vec<String>,
    pub command_history_index: Option<usize>,
}

impl App {
    pub fn new(config: AppConfig) -> Self {
        let history = HistoryManager::new(config.history_path.clone(), config.max_history);
        let model = config.model.clone();

        let chat_messages: Vec<ChatMessage> = history
            .entries()
            .iter()
            .map(|e| ChatMessage { role: e.role.clone(), text: e.text.clone(), complete: true })
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
            last_user_input_time: Instant::now(),
            model,
            command_history: Vec::new(),
            command_history_index: None,
        }
    }

    pub fn add_system_message(&mut self, text: String) {
        self.chat_messages.push(ChatMessage { role: Role::System, text, complete: true });
    }

    pub fn add_user_message(&mut self, text: String) {
        self.history.append(HistoryEntry::new(Role::User, text.clone()));
        self.chat_messages.push(ChatMessage { role: Role::User, text, complete: true });
    }

    pub fn start_ai_message(&mut self) {
        self.chat_messages.push(ChatMessage { role: Role::Ai, text: String::new(), complete: false });
        self.is_generating = true;
    }

    pub fn append_token(&mut self, token: &str) {
        if let Some(last) = self.chat_messages.last_mut() {
            if !last.complete { last.text.push_str(token); }
        }
    }

    pub fn finish_ai_message(&mut self) {
        if let Some(last) = self.chat_messages.last_mut() {
            if !last.complete {
                last.complete = true;
                self.history.append(HistoryEntry::new(Role::Ai, last.text.clone()));
            }
        }
        self.is_generating = false;
    }

    pub fn handle_generation_error(&mut self, error: String) {
        if let Some(last) = self.chat_messages.last() {
            if !last.complete && last.text.is_empty() {
                self.chat_messages.pop();
            }
        }
        self.is_generating = false;
        self.add_system_message(format!("[error] {}", error));
    }

    pub fn should_auto_think(&self) -> bool {
        !self.is_generating
            && self.last_user_input_time.elapsed().as_secs() >= self.config.auto_think_delay_secs
    }

    pub fn handle_command(&mut self, input: &str) -> HandleResult {
        let cmd = ollama::parse_input(input);
        match cmd {
            Command::Quit => { self.should_quit = true; HandleResult::Nothing }
            Command::Help => {
                self.add_system_message(
                    "Commands:\n  /help   - Show this help\n  /clear  - Clear memory\n  /model <name> - Switch model\n  /stats  - Show system info\n  /update - Pull & rebuild\n  /quit   - Exit".to_string(),
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
                    self.add_system_message(format!("Model switched to: {}", name));
                }
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

    pub fn insert_char(&mut self, c: char) {
        self.input_buffer.insert(self.input_cursor, c);
        self.input_cursor += c.len_utf8();
        self.is_user_typing = true;
        self.last_user_input_time = Instant::now();
    }

    pub fn delete_char_before_cursor(&mut self) {
        if self.input_cursor > 0 {
            let prev = self.input_buffer[..self.input_cursor]
                .char_indices().last().map(|(i, _)| i).unwrap_or(0);
            self.input_buffer.remove(prev);
            self.input_cursor = prev;
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.input_cursor > 0 {
            self.input_cursor = self.input_buffer[..self.input_cursor]
                .char_indices().last().map(|(i, _)| i).unwrap_or(0);
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.input_cursor < self.input_buffer.len() {
            self.input_cursor = self.input_buffer[self.input_cursor..]
                .char_indices().nth(1)
                .map(|(i, _)| self.input_cursor + i)
                .unwrap_or(self.input_buffer.len());
        }
    }

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

#[derive(Debug)]
pub enum HandleResult {
    Nothing,
    GenerateResponse(String),
    GenerateAutonomous,
    RunUpdate,
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
