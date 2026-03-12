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

    #[allow(dead_code)]
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
/// Stops early once `target_lines` complete lines have been received.
/// Lines exceeding `max_line_width` are force-wrapped with a newline.
/// Returns the full concatenated text.
#[allow(dead_code)]
pub async fn stream_to_canvas(
    mut stream: crate::llm::LlmStream,
    tx: &mpsc::UnboundedSender<ToolOutput>,
    target_lines: usize,
    max_line_width: usize,
) -> Result<String, AppError> {
    let mut full_text = String::new();
    while let Some(result) = stream.recv().await {
        match result {
            Ok(token) => {
                full_text.push_str(&token);

                // Force-wrap any line that exceeds max width
                if max_line_width > 0 {
                    full_text = force_wrap(&full_text, max_line_width);
                }

                if tx.send(ToolOutput::CanvasContent(full_text.clone())).is_err() {
                    break;
                }

                // Stop once we have enough complete lines
                if target_lines > 0 && count_complete_lines(&full_text) >= target_lines {
                    break;
                }
            }
            Err(e) => return Err(e),
        }
    }
    Ok(full_text)
}

/// Counts complete lines (lines terminated by '\n').
#[allow(dead_code)]
fn count_complete_lines(text: &str) -> usize {
    if text.ends_with('\n') {
        text.lines().count()
    } else {
        text.lines().count().saturating_sub(1)
    }
}

/// Force-wraps any line exceeding `max_width` characters by inserting newlines.
/// Uses char boundaries to avoid panics on multi-byte UTF-8.
#[allow(dead_code)]
fn force_wrap(text: &str, max_width: usize) -> String {
    let mut result = String::with_capacity(text.len() + 16);
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            result.push('\n');
        }
        if line.chars().count() <= max_width {
            result.push_str(line);
        } else {
            let mut chars = line.chars().peekable();
            let mut col = 0;
            while chars.peek().is_some() {
                if col > 0 && col % max_width == 0 {
                    result.push('\n');
                }
                result.push(chars.next().unwrap());
                col += 1;
            }
        }
    }
    result
}

/// Collects an LlmStream into a String, stopping after max_lines newlines.
/// If max_lines is 0, collects until the stream ends.
pub async fn collect_stream(
    mut stream: crate::llm::LlmStream,
    max_lines: usize,
) -> Result<String, AppError> {
    let mut text = String::new();
    while let Some(result) = stream.recv().await {
        match result {
            Ok(token) => {
                text.push_str(&token);
                if max_lines > 0 && text.matches('\n').count() >= max_lines {
                    break;
                }
            }
            Err(e) => return Err(e),
        }
    }
    Ok(text)
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

    /// Helper: create an LlmStream from a vec of tokens.
    fn mock_stream(tokens: Vec<&str>) -> crate::llm::LlmStream {
        let (tx, rx) = mpsc::unbounded_channel();
        for t in tokens {
            tx.send(Ok(t.to_string())).unwrap();
        }
        drop(tx); // signal completion
        rx
    }

    #[tokio::test]
    async fn test_stream_to_chat_concatenates() {
        let stream = mock_stream(vec!["hello ", "world"]);
        let (tx, mut rx) = mpsc::unbounded_channel();
        let result = stream_to_chat(stream, &tx).await.unwrap();
        assert_eq!(result, "hello world");
        // Verify tokens were forwarded
        let mut tokens = Vec::new();
        while let Ok(output) = rx.try_recv() {
            if let ToolOutput::ChatToken(t) = output {
                tokens.push(t);
            }
        }
        assert_eq!(tokens, vec!["hello ", "world"]);
    }

    #[tokio::test]
    async fn test_stream_to_canvas_stops_at_target_lines() {
        // Send 10 lines but target is 3
        let mut lines = String::new();
        for i in 1..=10 {
            lines.push_str(&format!("line {}\n", i));
        }
        let tokens: Vec<&str> = lines.split_inclusive('\n').collect();
        let stream = mock_stream(tokens);
        let (tx, _rx) = mpsc::unbounded_channel();
        let result = stream_to_canvas(stream, &tx, 3, 0).await.unwrap();
        let line_count = result.lines().count();
        assert!(line_count >= 3, "expected at least 3 lines, got {}", line_count);
        assert!(line_count <= 4, "expected at most 4 lines (3 + partial), got {}", line_count);
    }

    #[tokio::test]
    async fn test_stream_to_canvas_zero_target_no_cutoff() {
        let stream = mock_stream(vec!["line1\n", "line2\n", "line3\n"]);
        let (tx, _rx) = mpsc::unbounded_channel();
        let result = stream_to_canvas(stream, &tx, 0, 0).await.unwrap();
        assert_eq!(result.lines().count(), 3);
    }

    #[tokio::test]
    async fn test_stream_to_canvas_partial_lines_not_counted() {
        let stream = mock_stream(vec!["line1\n", "partial"]);
        let (tx, _rx) = mpsc::unbounded_channel();
        let result = stream_to_canvas(stream, &tx, 3, 0).await.unwrap();
        assert_eq!(result, "line1\npartial");
    }

    #[tokio::test]
    async fn test_stream_to_canvas_sends_content_events() {
        let stream = mock_stream(vec!["a\n", "b\n"]);
        let (tx, mut rx) = mpsc::unbounded_channel();
        let _result = stream_to_canvas(stream, &tx, 5, 0).await.unwrap();
        let mut events = Vec::new();
        while let Ok(output) = rx.try_recv() {
            if let ToolOutput::CanvasContent(c) = output {
                events.push(c);
            }
        }
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], "a\n");
        assert_eq!(events[1], "a\nb\n");
    }

    #[tokio::test]
    async fn test_stream_to_canvas_force_wraps_long_lines() {
        // A single 25-char token with no newline, max width 10
        let stream = mock_stream(vec!["abcdefghijklmnopqrstuvwxy"]);
        let (tx, _rx) = mpsc::unbounded_channel();
        let result = stream_to_canvas(stream, &tx, 0, 10).await.unwrap();
        // Should be force-wrapped into 3 lines: 10 + 10 + 5
        assert_eq!(result, "abcdefghij\nklmnopqrst\nuvwxy");
    }

    #[tokio::test]
    async fn test_stream_to_canvas_force_wrap_triggers_line_cutoff() {
        // A long line that force-wraps into 3+ lines, target is 2
        let stream = mock_stream(vec!["abcdefghijklmnopqrstuvwxyz0123456789"]);
        let (tx, _rx) = mpsc::unbounded_channel();
        let result = stream_to_canvas(stream, &tx, 2, 10).await.unwrap();
        // After wrapping: "abcdefghij\nklmnopqrst\nuvwxyz0123\n456789"
        // 2 complete lines → should stop
        let complete = count_complete_lines(&result);
        assert!(complete >= 2, "expected >= 2 complete lines, got {}", complete);
    }

    #[test]
    fn test_force_wrap_short_lines_unchanged() {
        assert_eq!(force_wrap("hello\nworld\n", 10), "hello\nworld\n");
    }

    #[test]
    fn test_force_wrap_long_line() {
        assert_eq!(force_wrap("abcdefghijklmno", 5), "abcde\nfghij\nklmno");
    }

    #[test]
    fn test_force_wrap_mixed() {
        assert_eq!(
            force_wrap("short\nabcdefghijklmno\nok", 5),
            "short\nabcde\nfghij\nklmno\nok"
        );
    }

    #[test]
    fn test_force_wrap_multibyte_no_panic() {
        // Full-width slashes (3 bytes each) — the exact crash case
        let input = "／／／／／／／／／／";  // 10 full-width chars
        let result = force_wrap(input, 4);
        // Should wrap at 4 chars, not panic on byte boundary
        assert_eq!(result.lines().count(), 3); // 4 + 4 + 2
    }

    #[test]
    fn test_force_wrap_emoji() {
        let input = "😀😀😀😀😀😀";  // 6 emoji, 4 bytes each
        let result = force_wrap(input, 3);
        assert_eq!(result, "😀😀😀\n😀😀😀");
    }

    #[test]
    fn test_count_complete_lines() {
        assert_eq!(count_complete_lines("a\nb\n"), 2);
        assert_eq!(count_complete_lines("a\nb"), 1);
        assert_eq!(count_complete_lines("partial"), 0);
        assert_eq!(count_complete_lines(""), 0);
        assert_eq!(count_complete_lines("\n"), 1);
    }

    #[tokio::test]
    async fn test_collect_stream_with_limit() {
        let stream = mock_stream(vec!["line1\n", "line2\n", "line3\n", "line4\n"]);
        let result = collect_stream(stream, 2).await.unwrap();
        assert_eq!(result.matches('\n').count(), 2);
    }

    #[tokio::test]
    async fn test_collect_stream_no_limit() {
        let stream = mock_stream(vec!["a\n", "b\n"]);
        let result = collect_stream(stream, 0).await.unwrap();
        assert_eq!(result, "a\nb\n");
    }
}
