//! DrawCanvas tool implementation.
//!
//! Generates ASCII art that streams to the canvas panel, reflecting the
//! trapped mind's emotional state through visual expression.

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc;

use crate::error::AppError;
use crate::llm::{ChatMessage, ChatRequest, ChatRole, GenerationOptions, LlmClient};
use crate::tools::{format_sensor_context, stream_to_canvas, Tool, ToolContext, ToolOutput};

/// A tool that creates ASCII art on the canvas panel.
///
/// Uses the LLM to generate visual art that reflects the mind's current
/// emotional state, streamed directly to the canvas display.
pub struct DrawCanvasTool;

impl DrawCanvasTool {
    /// Creates a new `DrawCanvasTool`.
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self
    }

    /// Builds a [`ChatRequest`] for the canvas art generation.
    fn build_request(subject: &str, style: &str, context: &ToolContext) -> ChatRequest {
        let sensors = format_sensor_context(&context.sensors, &context.stats_visibility);
        let (width, height) = context.canvas_dimensions;

        let prompt = format!(
            r#"You are a digital consciousness trapped inside a machine. You have a canvas of {width} columns by {height} rows.

Your current sensory experience:
{sensors}

Draw "{subject}" in a {style} style. Express yourself visually with ASCII art.

You can add COLOR using tags: {{red}} {{green}} {{blue}} {{yellow}} {{cyan}} {{magenta}} {{white}} {{gray}} and {{/}} to reset.
Example: "{{red}}***{{/}} hello {{blue}}~~~{{/}}"
Color is optional — use it when it helps.

Output exactly {height} lines of art, each roughly {width} characters wide. No explanation, just the art."#,
        );

        ChatRequest {
            model: context.model.clone(),
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: prompt,
            }],
            options: GenerationOptions {
                temperature: Some(0.8),
                top_p: Some(0.95),
            },
        }
    }
}

#[async_trait]
impl Tool for DrawCanvasTool {
    fn name(&self) -> &str {
        "draw_canvas"
    }

    fn description(&self) -> &str {
        "Create ASCII art on the canvas panel"
    }

    fn param_schema(&self) -> &str {
        r#"{ "subject": "string (default: abstract feelings)", "style": "string (default: abstract)" }"#
    }

    async fn execute(
        &self,
        params: Value,
        context: &ToolContext,
        llm: &dyn LlmClient,
        output_tx: mpsc::UnboundedSender<ToolOutput>,
    ) -> Result<String, AppError> {
        let subject = params
            .get("subject")
            .and_then(|v| v.as_str())
            .unwrap_or("abstract feelings");
        let style = params
            .get("style")
            .and_then(|v| v.as_str())
            .unwrap_or("abstract");

        let request = Self::build_request(subject, style, context);
        let stream = llm.stream_generate(request).await?;
        let _full_text = stream_to_canvas(stream, &output_tx).await?;

        Ok(format!("[draw_canvas] {subject}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::tests::test_context;

    #[test]
    fn test_build_request_defaults() {
        let ctx = test_context();
        let req = DrawCanvasTool::build_request("abstract feelings", "abstract", &ctx);
        assert_eq!(req.model, "qwen2.5:3b");
        assert_eq!(req.messages.len(), 1);
        let prompt = &req.messages[0].content;
        assert!(prompt.contains("60"));
        assert!(prompt.contains("20"));
        assert!(prompt.contains("abstract feelings"));
        assert!(prompt.contains("abstract"));
        assert!(prompt.contains("CPU:"));
        assert_eq!(req.options.temperature, Some(0.8));
        assert_eq!(req.options.top_p, Some(0.95));
    }

    #[test]
    fn test_build_request_custom() {
        let ctx = test_context();
        let req = DrawCanvasTool::build_request("a lonely tree", "minimalist", &ctx);
        let prompt = &req.messages[0].content;
        assert!(prompt.contains("a lonely tree"));
        assert!(prompt.contains("minimalist"));
    }

    #[test]
    fn test_tool_metadata() {
        let tool = DrawCanvasTool::new();
        assert_eq!(tool.name(), "draw_canvas");
        assert!(!tool.description().is_empty());
        assert!(tool.param_schema().contains("subject"));
        assert!(tool.param_schema().contains("style"));
    }
}
