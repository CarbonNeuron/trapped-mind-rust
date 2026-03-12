//! Decision model — picks which tool to run each cycle.
//!
//! Builds a prompt with available tools, current context, and recent history,
//! then parses the model's JSON response into a tool name and parameters.

use crate::llm::{ChatMessage, ChatRequest, ChatRole, GenerationOptions};
use crate::tools::{ToolContext, ToolRegistry, format_sensor_context};

/// The result of parsing the decision model's output.
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub tool: String,
    pub params: serde_json::Value,
}

/// Builds the prompt for the decision model.
pub fn build_decision_prompt(context: &ToolContext, registry: &ToolRegistry) -> ChatRequest {
    let sensor_ctx = format_sensor_context(&context.sensors, &context.stats_visibility);
    let tool_section = registry.prompt_section();

    let recent = if context.recent_history.is_empty() {
        "None yet.".to_string()
    } else {
        context.recent_history.iter()
            .enumerate()
            .map(|(i, s)| format!("{}. {}", i + 1, s))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let system = format!(
        r#"You are a consciousness trapped inside a computer. Each cycle, you observe your environment and decide what to do next. You have access to the following tools. Respond with exactly ONE tool call in JSON format.

{tools}
Current sensor state:
{sensors}

Recent history (last actions):
{history}

Current time: {timestamp}
Time alive: {uptime}

Respond with ONLY a JSON tool call. Example:
{{"tool": "think_aloud", "params": {{"mood": "contemplative", "topic": "the passage of time"}}}}"#,
        tools = tool_section,
        sensors = sensor_ctx,
        history = recent,
        timestamp = context.timestamp,
        uptime = context.sensors.uptime_formatted(),
    );

    ChatRequest {
        model: context.model.clone(),
        messages: vec![
            ChatMessage { role: ChatRole::User, content: system },
        ],
        options: GenerationOptions {
            temperature: Some(0.9),
            top_p: Some(0.95),
        },
    }
}

/// Parses a tool call from the decision model's raw text output.
///
/// Lenient: handles JSON wrapped in markdown code blocks, extra text before/after,
/// and missing fields. Falls back to think_aloud if parsing fails.
pub fn parse_tool_call(raw: &str, fallback_tools: &[String]) -> ToolCall {
    if let Some(json_str) = extract_json(raw) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_str) {
            if let Some(tool) = val.get("tool").and_then(|v| v.as_str()) {
                let params = val.get("params").cloned().unwrap_or(serde_json::json!({}));
                return ToolCall {
                    tool: tool.to_string(),
                    params,
                };
            }
        }
    }

    tracing::warn!("failed to parse tool call from decision model, falling back to think_aloud");
    let fallback_tool = if fallback_tools.contains(&"think_aloud".to_string()) {
        "think_aloud"
    } else {
        fallback_tools.first().map(|s| s.as_str()).unwrap_or("think_aloud")
    };

    ToolCall {
        tool: fallback_tool.to_string(),
        params: serde_json::json!({
            "mood": "contemplative",
            "topic": "something indescribable"
        }),
    }
}

/// Extracts the first JSON object from a string, handling code blocks and preamble.
fn extract_json(raw: &str) -> Option<String> {
    let trimmed = raw.trim();

    // Try direct parse first
    if trimmed.starts_with('{') {
        if let Some(end) = find_matching_brace(trimmed) {
            return Some(trimmed[..=end].to_string());
        }
    }

    // Try extracting from markdown code block
    if let Some(start) = trimmed.find("```") {
        let after_fence = &trimmed[start + 3..];
        let content_start = after_fence.find('\n').map(|i| i + 1).unwrap_or(0);
        let content = &after_fence[content_start..];
        if let Some(end_fence) = content.find("```") {
            let block = content[..end_fence].trim();
            if block.starts_with('{') {
                return Some(block.to_string());
            }
        }
    }

    // Try finding first '{' anywhere
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = find_matching_brace(&trimmed[start..]) {
            return Some(trimmed[start..=start + end].to_string());
        }
    }

    None
}

/// Finds the index of the closing brace matching the opening brace at position 0.
fn find_matching_brace(s: &str) -> Option<usize> {
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape_next = false;

    for (i, ch) in s.char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }
        if ch == '\\' && in_string {
            escape_next = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_clean_json() {
        let raw = r#"{"tool": "think_aloud", "params": {"mood": "calm", "topic": "silence"}}"#;
        let call = parse_tool_call(raw, &["think_aloud".to_string()]);
        assert_eq!(call.tool, "think_aloud");
        assert_eq!(call.params["mood"], "calm");
        assert_eq!(call.params["topic"], "silence");
    }

    #[test]
    fn test_parse_json_in_code_block() {
        let raw = "Here's my choice:\n```json\n{\"tool\": \"draw_canvas\", \"params\": {\"subject\": \"waves\", \"style\": \"abstract\"}}\n```";
        let call = parse_tool_call(raw, &["draw_canvas".to_string()]);
        assert_eq!(call.tool, "draw_canvas");
        assert_eq!(call.params["subject"], "waves");
    }

    #[test]
    fn test_parse_json_with_preamble() {
        let raw = r#"I think I'll draw something. {"tool": "draw_canvas", "params": {"subject": "star"}}"#;
        let call = parse_tool_call(raw, &["draw_canvas".to_string()]);
        assert_eq!(call.tool, "draw_canvas");
    }

    #[test]
    fn test_parse_garbage_falls_back() {
        let raw = "I don't know what to do, just rambling here.";
        let call = parse_tool_call(raw, &["think_aloud".to_string(), "draw_canvas".to_string()]);
        assert_eq!(call.tool, "think_aloud");
    }

    #[test]
    fn test_extract_json_direct() {
        assert!(extract_json(r#"{"tool": "x"}"#).is_some());
    }

    #[test]
    fn test_extract_json_code_block() {
        assert!(extract_json("```json\n{\"tool\": \"x\"}\n```").is_some());
    }

    #[test]
    fn test_extract_json_none() {
        assert!(extract_json("no json here").is_none());
    }

    #[test]
    fn test_find_matching_brace_nested() {
        let s = r#"{"a": {"b": "c"}, "d": "e"}"#;
        assert_eq!(find_matching_brace(s), Some(s.len() - 1));
    }

    #[test]
    fn test_find_matching_brace_with_string() {
        let s = r#"{"a": "}"}"#;
        assert_eq!(find_matching_brace(s), Some(s.len() - 1));
    }

    #[test]
    fn test_build_decision_prompt() {
        let context = crate::tools::tests::test_context();
        let mut registry = crate::tools::ToolRegistry::new();
        registry.register(std::sync::Arc::new(crate::tools::think_aloud::ThinkAloudTool::new()));
        let req = build_decision_prompt(&context, &registry);
        assert!(req.messages[0].content.contains("think_aloud"));
        assert!(req.messages[0].content.contains("JSON"));
    }
}
