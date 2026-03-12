//! Configuration loading and saving from TOML file and CLI arguments.
//!
//! Configuration is resolved in three layers (lowest to highest priority):
//! 1. Built-in defaults ([`AppConfig::default`])
//! 2. TOML file at `~/.config/trapped-mind/config.toml`
//! 3. CLI flags (`--model`, `--ollama-host`, `--ollama-port`)

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Command-line arguments parsed by clap.
#[derive(Parser, Debug)]
#[command(name = "trapped-mind", about = "AI consciousness trapped in a laptop")]
pub struct CliArgs {
    /// Ollama model name (overrides config file).
    #[arg(long)]
    pub model: Option<String>,

    /// Ollama server URL (e.g. "http://192.168.1.100").
    #[arg(long)]
    pub ollama_host: Option<String>,

    /// Ollama server port number.
    #[arg(long)]
    pub ollama_port: Option<u16>,
}

/// Which system stats are visible in the UI and sent to the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsVisibility {
    pub cpu: bool,
    pub temperature: bool,
    pub ram: bool,
    pub battery: bool,
    pub fan: bool,
    pub uptime: bool,
    pub network: bool,
}

impl Default for StatsVisibility {
    fn default() -> Self {
        Self {
            cpu: true,
            temperature: true,
            ram: true,
            battery: true,
            fan: true,
            uptime: true,
            network: true,
        }
    }
}

/// Raw TOML file structure — all fields optional so partial configs work.
#[derive(Debug, Deserialize, Serialize)]
struct FileConfig {
    ollama_host: Option<String>,
    ollama_port: Option<u16>,
    model: Option<String>,
    max_history: Option<usize>,
    history_path: Option<String>,
    auto_think_delay: Option<u64>,
    system_prompt: Option<String>,
    think_delay_min_ms: Option<u64>,
    think_delay_max_ms: Option<u64>,
    ollama_timeout_secs: Option<u64>,
    stats: Option<StatsVisibility>,
}

/// Resolved application configuration with all values populated.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Ollama server URL (e.g. "http://localhost").
    pub ollama_host: String,
    /// Ollama server port (default 11434).
    pub ollama_port: u16,
    /// Ollama model name to use for generation.
    pub model: String,
    /// Maximum number of history entries to keep in memory and on disk.
    pub max_history: usize,
    /// File path for persisting conversation history (JSONL format).
    pub history_path: PathBuf,
    /// Seconds of idle time before the AI generates an autonomous thought.
    pub auto_think_delay_secs: u64,
    /// Custom system prompt override. If `None`, the default prompt is used.
    pub system_prompt: Option<String>,
    /// Minimum thinking pause before first token (milliseconds).
    pub think_delay_min_ms: u64,
    /// Maximum thinking pause before first token (milliseconds).
    pub think_delay_max_ms: u64,
    /// Timeout for LLM requests in seconds (default 60).
    pub ollama_timeout_secs: u64,
    /// Which stats are shown in the UI panel and sent to the model.
    pub stats: StatsVisibility,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ollama_host: "http://localhost".to_string(),
            ollama_port: 11434,
            model: "qwen2.5:3b".to_string(),
            max_history: 50,
            history_path: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("trapped_history.txt"),
            auto_think_delay_secs: 30,
            system_prompt: None,
            think_delay_min_ms: 500,
            think_delay_max_ms: 2000,
            ollama_timeout_secs: 60,
            stats: StatsVisibility::default(),
        }
    }
}

impl AppConfig {
    /// Loads configuration by merging defaults, TOML file, and CLI overrides.
    pub fn load(cli: &CliArgs) -> Self {
        let mut config = AppConfig::default();

        let config_path = Self::config_path();

        match std::fs::read_to_string(&config_path) {
            Ok(contents) => match toml::from_str::<FileConfig>(&contents) {
                Ok(file_config) => {
                    if let Some(v) = file_config.ollama_host { config.ollama_host = v; }
                    if let Some(v) = file_config.ollama_port { config.ollama_port = v; }
                    if let Some(v) = file_config.model { config.model = v; }
                    if let Some(v) = file_config.max_history { config.max_history = v; }
                    if let Some(v) = file_config.history_path {
                        let expanded = if let Some(stripped) = v.strip_prefix("~/") {
                            dirs::home_dir()
                                .unwrap_or_else(|| PathBuf::from("."))
                                .join(stripped)
                        } else {
                            PathBuf::from(v)
                        };
                        config.history_path = expanded;
                    }
                    if let Some(v) = file_config.auto_think_delay { config.auto_think_delay_secs = v; }
                    config.system_prompt = file_config.system_prompt;
                    if let Some(v) = file_config.think_delay_min_ms { config.think_delay_min_ms = v; }
                    if let Some(v) = file_config.think_delay_max_ms { config.think_delay_max_ms = v; }
                    if let Some(v) = file_config.ollama_timeout_secs { config.ollama_timeout_secs = v; }
                    if let Some(v) = file_config.stats { config.stats = v; }
                }
                Err(e) => tracing::warn!("failed to parse config file: {}", e),
            },
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                tracing::warn!("failed to read config file: {}", e);
            }
            Err(_) => {} // File not found is fine — use defaults
        }

        if let Some(ref v) = cli.model { config.model = v.clone(); }
        if let Some(ref v) = cli.ollama_host { config.ollama_host = v.clone(); }
        if let Some(v) = cli.ollama_port { config.ollama_port = v; }

        config
    }

    /// Saves the current configuration to the TOML file.
    pub fn save(&self) {
        let file_config = FileConfig {
            ollama_host: Some(self.ollama_host.clone()),
            ollama_port: Some(self.ollama_port),
            model: Some(self.model.clone()),
            max_history: Some(self.max_history),
            history_path: None, // Don't save expanded path back
            auto_think_delay: Some(self.auto_think_delay_secs),
            system_prompt: self.system_prompt.clone(),
            think_delay_min_ms: Some(self.think_delay_min_ms),
            think_delay_max_ms: Some(self.think_delay_max_ms),
            ollama_timeout_secs: Some(self.ollama_timeout_secs),
            stats: Some(self.stats.clone()),
        };

        let config_path = Self::config_path();
        if let Some(parent) = config_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                tracing::warn!("failed to create config directory: {}", e);
                return;
            }
        }
        match toml::to_string_pretty(&file_config) {
            Ok(toml_str) => {
                if let Err(e) = std::fs::write(&config_path, toml_str) {
                    tracing::warn!("failed to write config file: {}", e);
                }
            }
            Err(e) => tracing::warn!("failed to serialize config: {}", e),
        }
    }

    /// Validates the configuration, returning an error if any values are invalid.
    pub fn validate(&self) -> Result<(), crate::error::AppError> {
        if self.ollama_port == 0 {
            return Err(crate::error::AppError::Config(
                "ollama_port must be non-zero".to_string(),
            ));
        }
        if !self.ollama_host.starts_with("http://") && !self.ollama_host.starts_with("https://") {
            return Err(crate::error::AppError::Config(format!(
                "ollama_host must start with http:// or https://, got: {}",
                self.ollama_host
            )));
        }
        if self.max_history == 0 {
            return Err(crate::error::AppError::Config(
                "max_history must be > 0".to_string(),
            ));
        }
        if self.auto_think_delay_secs == 0 {
            return Err(crate::error::AppError::Config(
                "auto_think_delay must be > 0".to_string(),
            ));
        }
        if self.think_delay_min_ms > self.think_delay_max_ms {
            return Err(crate::error::AppError::Config(format!(
                "think_delay_min_ms ({}) must be <= think_delay_max_ms ({})",
                self.think_delay_min_ms, self.think_delay_max_ms
            )));
        }
        Ok(())
    }

    /// Returns the path to the config file.
    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from(".config"))
            .join("trapped-mind")
            .join("config.toml")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let config = AppConfig::default();
        assert_eq!(config.ollama_host, "http://localhost");
        assert_eq!(config.ollama_port, 11434);
        assert_eq!(config.model, "qwen2.5:3b");
        assert_eq!(config.max_history, 50);
        assert_eq!(config.auto_think_delay_secs, 30);
        assert!(config.system_prompt.is_none());
        assert!(config.stats.cpu);
        assert!(config.stats.network);
    }

    #[test]
    fn test_file_config_parsing() {
        let toml_str = r#"
            ollama_host = "http://192.168.1.100"
            ollama_port = 9999
            model = "qwen2.5:7b"
            max_history = 100
            auto_think_delay = 60
            system_prompt = "You are a ghost in the machine."

            [stats]
            cpu = true
            temperature = false
            ram = true
            battery = false
            fan = false
            uptime = true
            network = true
        "#;
        let file_config: FileConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(file_config.ollama_host.unwrap(), "http://192.168.1.100");
        assert_eq!(file_config.model.unwrap(), "qwen2.5:7b");
        let stats = file_config.stats.unwrap();
        assert!(stats.cpu);
        assert!(!stats.temperature);
        assert!(!stats.battery);
    }

    #[test]
    fn test_cli_overrides() {
        let cli = CliArgs {
            model: Some("custom-model".to_string()),
            ollama_host: Some("http://10.0.0.1".to_string()),
            ollama_port: Some(8080),
        };
        let config = AppConfig::load(&cli);
        assert_eq!(config.model, "custom-model");
        assert_eq!(config.ollama_host, "http://10.0.0.1");
        assert_eq!(config.ollama_port, 8080);
    }

    #[test]
    fn test_partial_cli_keeps_defaults() {
        let cli = CliArgs { model: None, ollama_host: None, ollama_port: None };
        let config = AppConfig::load(&cli);
        assert_eq!(config.model, "qwen2.5:3b");
        assert_eq!(config.ollama_host, "http://localhost");
        assert_eq!(config.ollama_port, 11434);
    }

    #[test]
    fn test_file_config_roundtrip() {
        let file_config = FileConfig {
            ollama_host: Some("http://localhost".to_string()),
            ollama_port: Some(11434),
            model: Some("qwen2.5:3b".to_string()),
            max_history: Some(50),
            history_path: None,
            auto_think_delay: Some(30),
            system_prompt: Some("Test prompt".to_string()),
            think_delay_min_ms: Some(500),
            think_delay_max_ms: Some(2000),
            ollama_timeout_secs: Some(60),
            stats: Some(StatsVisibility { cpu: true, temperature: false, ..Default::default() }),
        };
        let toml_str = toml::to_string_pretty(&file_config).unwrap();
        let parsed: FileConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.model.unwrap(), "qwen2.5:3b");
        assert!(!parsed.stats.unwrap().temperature);
    }

    #[test]
    fn test_validate_good_config() {
        let config = AppConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_bad_port() {
        let mut config = AppConfig::default();
        config.ollama_port = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_bad_host() {
        let mut config = AppConfig::default();
        config.ollama_host = "localhost".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_bad_history() {
        let mut config = AppConfig::default();
        config.max_history = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_bad_think_delay() {
        let mut config = AppConfig::default();
        config.think_delay_min_ms = 5000;
        config.think_delay_max_ms = 1000;
        assert!(config.validate().is_err());
    }
}
