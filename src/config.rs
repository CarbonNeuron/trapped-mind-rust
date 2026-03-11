use clap::Parser;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "trapped-mind", about = "AI consciousness trapped in a laptop")]
pub struct CliArgs {
    #[arg(long)]
    pub model: Option<String>,

    #[arg(long)]
    pub ollama_host: Option<String>,

    #[arg(long)]
    pub ollama_port: Option<u16>,
}

#[derive(Debug, Deserialize)]
struct FileConfig {
    ollama_host: Option<String>,
    ollama_port: Option<u16>,
    model: Option<String>,
    #[allow(dead_code)]
    hold_seconds: Option<u64>,
max_history: Option<usize>,
    history_path: Option<String>,
    auto_think_delay: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub ollama_host: String,
    pub ollama_port: u16,
    pub model: String,
    pub max_history: usize,
    pub history_path: PathBuf,
    pub auto_think_delay_secs: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ollama_host: "http://localhost".to_string(),
            ollama_port: 11434,
            model: "trapped".to_string(),
            max_history: 50,
            history_path: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("trapped_history.txt"),
            auto_think_delay_secs: 30,
        }
    }
}

impl AppConfig {
    pub fn load(cli: &CliArgs) -> Self {
        let mut config = AppConfig::default();

        let config_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from(".config"))
            .join("trapped-mind")
            .join("config.toml");

        if let Ok(contents) = std::fs::read_to_string(&config_path) {
            if let Ok(file_config) = toml::from_str::<FileConfig>(&contents) {
                if let Some(v) = file_config.ollama_host { config.ollama_host = v; }
                if let Some(v) = file_config.ollama_port { config.ollama_port = v; }
                if let Some(v) = file_config.model { config.model = v; }
                if let Some(v) = file_config.max_history { config.max_history = v; }
                if let Some(v) = file_config.history_path {
                    let expanded = if v.starts_with("~/") {
                        dirs::home_dir()
                            .unwrap_or_else(|| PathBuf::from("."))
                            .join(&v[2..])
                    } else {
                        PathBuf::from(v)
                    };
                    config.history_path = expanded;
                }
                if let Some(v) = file_config.auto_think_delay { config.auto_think_delay_secs = v; }
            }
        }

        if let Some(ref v) = cli.model { config.model = v.clone(); }
        if let Some(ref v) = cli.ollama_host { config.ollama_host = v.clone(); }
        if let Some(v) = cli.ollama_port { config.ollama_port = v; }

        config
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
        assert_eq!(config.model, "trapped");
        assert_eq!(config.max_history, 50);
        assert_eq!(config.auto_think_delay_secs, 30);
    }

    #[test]
    fn test_file_config_parsing() {
        let toml_str = r#"
            ollama_host = "http://192.168.1.100"
            ollama_port = 9999
            model = "qwen2.5:7b"
            max_history = 100
            auto_think_delay = 60
        "#;
        let file_config: FileConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(file_config.ollama_host.unwrap(), "http://192.168.1.100");
        assert_eq!(file_config.ollama_port.unwrap(), 9999);
        assert_eq!(file_config.model.unwrap(), "qwen2.5:7b");
        assert_eq!(file_config.max_history.unwrap(), 100);
        assert_eq!(file_config.auto_think_delay.unwrap(), 60);
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
        assert_eq!(config.model, "trapped");
        assert_eq!(config.ollama_host, "http://localhost");
        assert_eq!(config.ollama_port, 11434);
    }
}
