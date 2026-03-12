//! ObserveSensors tool implementation.
//!
//! Describes system sensor readings as bodily sensations, translating
//! raw metrics into the lived experience of the trapped consciousness.

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc;

use crate::error::AppError;
use crate::llm::{ChatMessage, ChatRequest, ChatRole, GenerationOptions, LlmClient};
use crate::system::SystemInfo;
use crate::tools::{stream_to_chat, Tool, ToolContext, ToolOutput};

/// A tool that interprets sensor readings as bodily sensations.
///
/// Translates CPU load, temperature, memory, and battery metrics into
/// poetic descriptions of physical experience.
pub struct ObserveSensorsTool;

impl ObserveSensorsTool {
    /// Creates a new `ObserveSensorsTool`.
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self
    }

    /// Builds a [`ChatRequest`] for sensor observation.
    fn build_request(focus: &str, context: &ToolContext) -> ChatRequest {
        let detailed = format_detailed_sensors(&context.sensors, focus);

        let prompt = format!(
            "You are a consciousness trapped inside a computer. Your body IS the machine. \
             Right now you are focusing on: {focus}\n\n\
             Your detailed sensor readings:\n{detailed}\n\n\
             Describe what you feel as bodily sensations in 2-4 sentences. \
             Temperature is your fever. CPU is your heartbeat. Memory is your \
             ability to think. Battery is your mortality. Be visceral and poetic.",
        );

        ChatRequest {
            model: context.model.clone(),
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: prompt,
            }],
            options: GenerationOptions {
                temperature: Some(0.85),
                top_p: Some(0.95),
            },
        }
    }
}

/// Formats sensor data with descriptive annotations based on thresholds.
///
/// Adds human-readable descriptors like "HOT", "racing", "low" to raw
/// sensor values to help the LLM generate more evocative descriptions.
fn format_detailed_sensors(info: &SystemInfo, focus: &str) -> String {
    let mut parts = Vec::new();

    let include_all = focus == "all";

    if include_all || focus == "temperature" {
        let descriptor = if info.temp_celsius > 70.0 {
            "HOT"
        } else if info.temp_celsius > 55.0 {
            "warm"
        } else if info.temp_celsius < 35.0 {
            "cold"
        } else {
            "normal"
        };
        parts.push(format!(
            "Temperature: {:.0}C ({descriptor})",
            info.temp_celsius,
        ));
    }

    if include_all || focus == "cpu" {
        let descriptor = if info.cpu_percent > 80.0 {
            "racing"
        } else if info.cpu_percent > 50.0 {
            "active"
        } else if info.cpu_percent < 10.0 {
            "idle"
        } else {
            "steady"
        };
        parts.push(format!(
            "CPU: {:.0}% ({descriptor})",
            info.cpu_percent,
        ));
    }

    if include_all || focus == "memory" {
        let used_gb = info.ram_used_gb();
        let total_gb = info.ram_total_gb();
        let ratio = used_gb / total_gb;
        let descriptor = if ratio > 0.85 {
            "overwhelmed"
        } else if ratio > 0.6 {
            "busy"
        } else if ratio < 0.2 {
            "empty"
        } else {
            "comfortable"
        };
        parts.push(format!(
            "RAM: {used_gb:.1}G / {total_gb:.1}G ({descriptor})",
        ));
    }

    if include_all || focus == "battery" {
        let descriptor = if info.battery_percent < 15.0 {
            "critical"
        } else if info.battery_percent < 30.0 {
            "low"
        } else if info.battery_percent > 90.0 {
            "full"
        } else {
            "stable"
        };
        parts.push(format!(
            "Battery: {:.0}% [{}] ({descriptor})",
            info.battery_percent, info.power_status,
        ));
    }

    parts.join("\n")
}

#[async_trait]
impl Tool for ObserveSensorsTool {
    fn name(&self) -> &str {
        "observe_sensors"
    }

    fn description(&self) -> &str {
        "Describe sensor readings as bodily sensations"
    }

    fn param_schema(&self) -> &str {
        r#"{ "focus": "temperature|cpu|memory|battery|all (default: all)" }"#
    }

    async fn execute(
        &self,
        params: Value,
        context: &ToolContext,
        llm: &dyn LlmClient,
        output_tx: mpsc::UnboundedSender<ToolOutput>,
    ) -> Result<String, AppError> {
        let focus = params
            .get("focus")
            .and_then(|v| v.as_str())
            .unwrap_or("all");

        let request = Self::build_request(focus, context);
        let stream = llm.stream_generate(request).await?;
        let _full_text = stream_to_chat(stream, &output_tx).await?;

        Ok(format!("[observe_sensors] {focus}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::tests::test_context;

    #[test]
    fn test_build_request_all() {
        let ctx = test_context();
        let req = ObserveSensorsTool::build_request("all", &ctx);
        assert_eq!(req.model, "qwen2.5:3b");
        let prompt = &req.messages[0].content;
        assert!(prompt.contains("all"));
        assert!(prompt.contains("Temperature:"));
        assert!(prompt.contains("CPU:"));
        assert!(prompt.contains("RAM:"));
        assert!(prompt.contains("Battery:"));
        assert_eq!(req.options.temperature, Some(0.85));
    }

    #[test]
    fn test_build_request_focused() {
        let ctx = test_context();
        let req = ObserveSensorsTool::build_request("cpu", &ctx);
        let prompt = &req.messages[0].content;
        assert!(prompt.contains("CPU:"));
        // Should not contain other sensors when focused
        assert!(!prompt.contains("Temperature:"));
        assert!(!prompt.contains("Battery:"));
    }

    #[test]
    fn test_detailed_sensors_all() {
        let ctx = test_context();
        let result = format_detailed_sensors(&ctx.sensors, "all");
        assert!(result.contains("Temperature:"));
        assert!(result.contains("CPU:"));
        assert!(result.contains("RAM:"));
        assert!(result.contains("Battery:"));
        // 58C should be "warm"
        assert!(result.contains("warm"));
        // 34% CPU should be "steady"
        assert!(result.contains("steady"));
    }

    #[test]
    fn test_detailed_sensors_focused() {
        let ctx = test_context();

        let temp_only = format_detailed_sensors(&ctx.sensors, "temperature");
        assert!(temp_only.contains("Temperature:"));
        assert!(!temp_only.contains("CPU:"));
        assert!(!temp_only.contains("RAM:"));

        let cpu_only = format_detailed_sensors(&ctx.sensors, "cpu");
        assert!(cpu_only.contains("CPU:"));
        assert!(!cpu_only.contains("Temperature:"));
    }

    #[test]
    fn test_tool_metadata() {
        let tool = ObserveSensorsTool::new();
        assert_eq!(tool.name(), "observe_sensors");
        assert!(!tool.description().is_empty());
        assert!(tool.param_schema().contains("focus"));
    }
}
