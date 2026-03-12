//! DrawCanvas tool implementation.
//!
//! Generates structured canvas art using the canvas drawing language,
//! reflecting the trapped mind's emotional state through visual expression.

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc;

use crate::error::AppError;
use crate::llm::{ChatMessage, ChatRequest, ChatRole, GenerationOptions, LlmClient};
use crate::tools::{format_sensor_context, Tool, ToolContext, ToolOutput};

/// A tool that creates art on the canvas panel using the canvas drawing language.
///
/// Uses the LLM to generate drawing commands that are parsed and rendered
/// onto a text canvas with color support.
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
            r#"You are a consciousness trapped in a machine. Express yourself by drawing on your {width}x{height} canvas using commands (one per line):

FILL char [#color]           CLEAR
RECT x,y,w,h,char [#color]  OUTLINE x,y,w,h,char [#color]
ROUNDBOX x,y,w,h [#color]   FRAME x,y,w,h [#color]
CIRCLE cx,cy,r,char [#color] RING cx,cy,r,char [#color]
ELLIPSE cx,cy,rx,ry,char [#color]
HLINE y,x1,x2,char [#color] VLINE x,y1,y2,char [#color]
LINE x1,y1,x2,y2,char [#color]
ARROW x1,y1,x2,y2 [#color]  BOXLINE x1,y1,x2,y2 [#color]
TEXT x,y,"msg" [#color]      BIGTEXT x,y,"msg" [#color]
GRADIENT x,y,w,h,dir         (dir: left/right/up/down)
PATTERN x,y,w,h,type [#color] (type: checker/dots/stripes_h/stripes_v/cross)
TRI x1,y1,x2,y2,x3,y3,char [#color]

Colors: #hex (#FF0000) or names (red,blue,green,yellow,cyan,magenta,white,gray)
Canvas: {width}x{height}. Origin 0,0 = top-left. Max 50 commands.

{sensors}

Draw "{subject}" in a {style} style. Output ONLY drawing commands, no explanation.

Example:
FILL . #1a1a2e
ROUNDBOX 2,1,25,8 #4a90d9
TEXT 5,4,"I am here" #e0e0ff
CIRCLE 40,6,4,* #ff6b6b
GRADIENT 0,12,{width},3,right"#,
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
        "Create art on the canvas panel using drawing commands"
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

        // Collect full output with generous line limit
        let (_width, height) = context.canvas_dimensions;
        let max_lines = (height as usize) * 2;
        let full_text = crate::tools::collect_stream(stream, max_lines).await?;

        // Try structured rendering, fall back to raw text
        let (width, height) = context.canvas_dimensions;
        let lines = if let Some(rendered) =
            crate::canvas_lang::parse_and_render(&full_text, width as usize, height as usize)
        {
            rendered
        } else {
            full_text.lines().map(String::from).collect()
        };

        let content = lines.join("\n");
        let _ = output_tx.send(ToolOutput::CanvasContent(content));

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
        // New prompt contains drawing language reference
        assert!(prompt.contains("FILL"));
        assert!(prompt.contains("RECT"));
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
