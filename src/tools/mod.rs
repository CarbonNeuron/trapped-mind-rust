//! Tool trait, registry, and supporting types for the activity system.
//!
//! Each tool represents an activity the trapped mind can perform. The decision
//! model picks a tool each cycle, and the registry dispatches execution to the
//! chosen tool's handler.

pub mod think_aloud;
pub mod draw_canvas;
pub mod write_journal;
pub mod read_journal;
pub mod observe_sensors;

use crate::config::StatsVisibility;
use crate::error::AppError;
use crate::llm::LlmClient;
use crate::system::SystemInfo;

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

/// Output from a tool, routed to the appropriate TUI panel.
#[derive(Debug, Clone)]
pub enum ToolOutput {
    /// Stream text to the chat/thought panel.
    ChatToken(String),
    /// Update the canvas with new content (full accumulated buffer).
    CanvasContent(String),
    /// Status message (shown briefly in status bar or as system message).
    Status(String),
}

/// Context passed to every tool execution.
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub sensors: SystemInfo,
    pub uptime: Duration,
    pub timestamp: String,
    pub recent_history: Vec<String>,
    pub canvas_dimensions: (u16, u16),
    pub model: String,
    pub stats_visibility: StatsVisibility,
}

/// Trait for tool implementations.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool identifier (used in decision model output).
    fn name(&self) -> &str;
    /// Brief description for the decision model's system prompt.
    fn description(&self) -> &str;
    /// Parameter schema as a string (for the decision model prompt).
    fn param_schema(&self) -> &str;
    /// Execute the tool with parsed parameters.
    /// Returns a summary string for the history log.
    async fn execute(
        &self,
        params: serde_json::Value,
        context: &ToolContext,
        llm: &dyn LlmClient,
        output_tx: mpsc::UnboundedSender<ToolOutput>,
    ) -> Result<String, AppError>;
}

/// Registry that holds all available tools and dispatches execution.
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    order: Vec<String>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            order: Vec::new(),
        }
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_string();
        self.tools.insert(name.clone(), tool);
        if !self.order.contains(&name) {
            self.order.push(name);
        }
    }

    /// Generates the tool descriptions section for the decision model prompt.
    pub fn prompt_section(&self) -> String {
        let mut section = String::from("Available tools:\n");
        for name in &self.order {
            if let Some(tool) = self.tools.get(name) {
                section.push_str(&format!(
                    "- {}: {}\n  Parameters: {}\n",
                    tool.name(),
                    tool.description(),
                    tool.param_schema(),
                ));
            }
        }
        section
    }

    pub fn tool_names(&self) -> &[String] {
        &self.order
    }

    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.tools.get(name)
    }

    pub async fn dispatch(
        &self,
        tool_name: &str,
        params: serde_json::Value,
        context: &ToolContext,
        llm: &dyn LlmClient,
        output_tx: mpsc::UnboundedSender<ToolOutput>,
    ) -> Result<String, AppError> {
        let tool = self.tools.get(tool_name).ok_or_else(|| {
            AppError::Tool(format!("unknown tool: {}", tool_name))
        })?;
        tool.execute(params, context, llm, output_tx).await
    }
}

/// Formats sensor data as context text for tool prompts.
pub fn format_sensor_context(info: &SystemInfo, vis: &StatsVisibility) -> String {
    let mut parts = Vec::new();
    if vis.cpu { parts.push(format!("CPU: {:.0}%", info.cpu_percent)); }
    if vis.temperature { parts.push(format!("Temperature: {:.0}C", info.temp_celsius)); }
    if vis.ram { parts.push(format!("RAM: {:.1}G / {:.1}G", info.ram_used_gb(), info.ram_total_gb())); }
    if vis.battery { parts.push(format!("Battery: {:.0}% ({})", info.battery_percent, info.power_status)); }
    if vis.fan { parts.push(format!("Fan: {} RPM", info.fan_rpm)); }
    if vis.uptime { parts.push(format!("Uptime: {}", info.uptime_formatted())); }
    parts.join("\n")
}

/// Consumes an LlmStream, forwarding each token as ChatToken through the output channel.
/// Returns the full concatenated text.
pub async fn stream_to_chat(
    mut stream: crate::llm::LlmStream,
    tx: &mpsc::UnboundedSender<ToolOutput>,
) -> Result<String, AppError> {
    let mut full_text = String::new();
    while let Some(result) = stream.recv().await {
        match result {
            Ok(token) => {
                full_text.push_str(&token);
                if tx.send(ToolOutput::ChatToken(token)).is_err() {
                    break;
                }
            }
            Err(e) => return Err(e),
        }
    }
    Ok(full_text)
}

/// Consumes an LlmStream, accumulating tokens and sending canvas updates.
/// Returns the full concatenated text.
pub async fn stream_to_canvas(
    mut stream: crate::llm::LlmStream,
    tx: &mpsc::UnboundedSender<ToolOutput>,
) -> Result<String, AppError> {
    let mut full_text = String::new();
    while let Some(result) = stream.recv().await {
        match result {
            Ok(token) => {
                full_text.push_str(&token);
                if tx.send(ToolOutput::CanvasContent(full_text.clone())).is_err() {
                    break;
                }
            }
            Err(e) => return Err(e),
        }
    }
    Ok(full_text)
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// Creates a test ToolContext for use in tool tests across submodules.
    pub fn test_context() -> ToolContext {
        ToolContext {
            sensors: SystemInfo {
                cpu_percent: 34.0, temp_celsius: 58.0,
                ram_used_bytes: 4_000_000_000, ram_total_bytes: 8_000_000_000,
                battery_percent: 72.0, power_status: "Discharging".to_string(),
                fan_rpm: 3200, uptime_secs: 9240, networks: vec![],
                cpu_real: true, temp_real: true, ram_real: true,
                battery_real: true, fan_real: true, network_real: true,
            },
            uptime: Duration::from_secs(9240),
            timestamp: "2026-03-12 14:30:00".to_string(),
            recent_history: vec![],
            canvas_dimensions: (60, 20),
            model: "qwen2.5:3b".to_string(),
            stats_visibility: StatsVisibility::default(),
        }
    }

    struct DummyTool;

    #[async_trait]
    impl Tool for DummyTool {
        fn name(&self) -> &str { "dummy" }
        fn description(&self) -> &str { "A test tool" }
        fn param_schema(&self) -> &str { r#"{ "x": "string" }"# }
        async fn execute(
            &self,
            _params: serde_json::Value,
            _context: &ToolContext,
            _llm: &dyn LlmClient,
            tx: mpsc::UnboundedSender<ToolOutput>,
        ) -> Result<String, AppError> {
            let _ = tx.send(ToolOutput::ChatToken("hello".to_string()));
            Ok("dummy executed".to_string())
        }
    }

    #[test]
    fn test_registry_register_and_lookup() {
        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(DummyTool));
        assert!(reg.get("dummy").is_some());
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn test_registry_tool_names() {
        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(DummyTool));
        assert_eq!(reg.tool_names(), &["dummy"]);
    }

    #[test]
    fn test_registry_prompt_section() {
        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(DummyTool));
        let section = reg.prompt_section();
        assert!(section.contains("dummy"));
        assert!(section.contains("A test tool"));
    }

    #[test]
    fn test_format_sensor_context_all() {
        let ctx = test_context();
        let text = format_sensor_context(&ctx.sensors, &ctx.stats_visibility);
        assert!(text.contains("CPU:"));
        assert!(text.contains("Temperature:"));
        assert!(text.contains("Battery:"));
    }

    #[test]
    fn test_format_sensor_context_filtered() {
        let ctx = test_context();
        let vis = StatsVisibility {
            cpu: true, temperature: false, ram: false,
            battery: true, fan: false, uptime: false, network: false,
        };
        let text = format_sensor_context(&ctx.sensors, &vis);
        assert!(text.contains("CPU:"));
        assert!(!text.contains("Temperature:"));
        assert!(text.contains("Battery:"));
    }
}
