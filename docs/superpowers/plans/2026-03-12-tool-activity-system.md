# Tool & Activity System Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the free-form generation loop with a tool-dispatching architecture where a decision model picks an activity each cycle, then specialized sub-prompts execute it.

**Architecture:** A decision model (small/fast LLM call) outputs a JSON tool call each cycle. A `ToolRegistry` dispatches to the chosen `Tool` implementation, which builds its own prompt, runs a sub-generation via `LlmClient`, and streams output to the TUI through a `ToolOutput` channel. The `LlmClient` trait is refactored to return a generic token stream (not tied to `AppEvent`).

**Tech Stack:** Rust, tokio, ollama-rs, ratatui, serde_json, async-trait

**Constraints:**
- All existing tests must continue passing
- `cargo clippy -- -D warnings` must pass
- `cargo test` must pass cleanly
- Follow existing code patterns (doc comments on pub items, tests in each module)

---

## File Structure

### New files:
- `src/tools/mod.rs` — `Tool` trait, `ToolRegistry`, `ToolContext`, `ToolOutput`
- `src/tools/think_aloud.rs` — ThinkAloudTool implementation
- `src/tools/draw_canvas.rs` — DrawCanvasTool implementation
- `src/tools/write_journal.rs` — WriteJournalTool implementation
- `src/tools/read_journal.rs` — ReadJournalTool implementation
- `src/tools/observe_sensors.rs` — ObserveSensorsTool implementation
- `src/decision.rs` — Decision model prompt building and JSON parsing

### Modified files:
- `src/llm.rs` — Refactor `LlmClient::stream_chat` to return `LlmStream` instead of sending `AppEvent`s
- `src/ollama.rs` — Update `OllamaClient` to match new `LlmClient` signature; move prompt-building functions out (they become tool-specific)
- `src/app.rs` — Add `ToolOutput` channel handling, remove hard-coded generation state, add tool history
- `src/main.rs` — Rewire event loop for decision→dispatch pattern, remove `spawn_generation`/`spawn_canvas_generation`
- `src/config.rs` — Add `[decision]` and `[tools.*]` config sections
- `src/error.rs` — Add `Tool` variant

---

## Chunk 1: Refactor LlmClient to Generic Token Stream

### Task 1: Add LlmStream type and refactor LlmClient trait

**Files:**
- Modify: `src/llm.rs`
- Modify: `src/error.rs`

- [ ] **Step 1: Add Tool error variant to AppError**

In `src/error.rs`, add a new variant:

```rust
/// Tool execution failure.
#[error("tool error: {0}")]
Tool(String),
```

Run: `cargo check`
Expected: compiles cleanly

- [ ] **Step 2: Add LlmStream type to llm.rs**

Replace the `AppEvent`-coupled trait with a generic token receiver. Add to `src/llm.rs`:

```rust
use tokio::sync::mpsc;

/// A stream of tokens from an LLM generation.
/// Receives `Ok(token)` for each token, then the sender is dropped on completion.
/// Receives `Err(AppError)` if generation fails.
pub type LlmStream = mpsc::UnboundedReceiver<Result<String, AppError>>;
```

Run: `cargo check`
Expected: compiles (unused import warning is fine for now)

- [ ] **Step 3: Add stream_generate method to LlmClient**

Add a new method to the `LlmClient` trait that returns `LlmStream` instead of taking an `AppEvent` sender. Keep the old `stream_chat` method temporarily for backwards compatibility:

```rust
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Streams a chat completion, sending tokens as [`AppEvent::Token`] and
    /// completion as [`AppEvent::GenerationDone`] through the channel.
    /// DEPRECATED: Use stream_generate instead.
    async fn stream_chat(
        &self,
        request: ChatRequest,
        tx: mpsc::UnboundedSender<crate::app::AppEvent>,
    ) -> Result<(), AppError>;

    /// Streams a chat completion, returning a token receiver.
    /// Each token arrives as Ok(String). Sender is dropped on completion.
    /// Errors arrive as Err(AppError).
    async fn stream_generate(&self, request: ChatRequest) -> Result<LlmStream, AppError>;

    /// Pulls/downloads a model by name.
    async fn pull_model(&self, model: &str) -> Result<(), AppError>;
}
```

Run: `cargo check`
Expected: Compile error — OllamaClient doesn't implement `stream_generate` yet

- [ ] **Step 4: Implement stream_generate on OllamaClient**

In `src/ollama.rs`, add the `stream_generate` implementation. It reuses the existing streaming logic but sends tokens through a new channel:

```rust
async fn stream_generate(&self, request: ChatRequest) -> Result<LlmStream, AppError> {
    let (tx, rx) = mpsc::unbounded_channel();
    let ollama_request = Self::to_ollama_request(&request);
    let client = self.client.clone();
    let timeout_secs = self.timeout_secs;

    tokio::spawn(async move {
        let stream_result = tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            client.send_chat_messages_stream(ollama_request),
        )
        .await;

        match stream_result {
            Err(_) => {
                let _ = tx.send(Err(AppError::Llm("request timed out".to_string())));
            }
            Ok(Err(e)) => {
                let _ = tx.send(Err(AppError::Llm(e.to_string())));
            }
            Ok(Ok(mut stream)) => {
                while let Some(res) = stream.next().await {
                    match res {
                        Ok(resp) => {
                            if !resp.message.content.is_empty() {
                                if tx.send(Ok(resp.message.content)).is_err() {
                                    break;
                                }
                            }
                            if resp.done {
                                break;
                            }
                        }
                        Err(_) => {
                            let _ = tx.send(Err(AppError::Llm("stream error".to_string())));
                            break;
                        }
                    }
                }
            }
        }
        // tx is dropped here, signaling completion
    });

    Ok(rx)
}
```

Note: `ollama_rs::Ollama` is `Clone`, so the spawned task can clone it directly. No `Arc` wrapping needed.

In `stream_generate`, add retry logic matching the existing `stream_chat` behavior:

```rust
async fn stream_generate(&self, request: ChatRequest) -> Result<LlmStream, AppError> {
    let (tx, rx) = mpsc::unbounded_channel();
    let ollama_request = Self::to_ollama_request(&request);
    let client = self.client.clone();
    let timeout_secs = self.timeout_secs;

    tokio::spawn(async move {
        let mut last_err = None;

        for attempt in 0..3u32 {
            if attempt > 0 {
                let backoff = Duration::from_secs(1 << attempt);
                tracing::warn!(
                    "retrying LLM stream_generate (attempt {}), backoff {:?}",
                    attempt + 1,
                    backoff
                );
                tokio::time::sleep(backoff).await;
            }

            let stream_result = tokio::time::timeout(
                Duration::from_secs(timeout_secs),
                client.send_chat_messages_stream(ollama_request.clone()),
            )
            .await;

            match stream_result {
                Err(_) => {
                    last_err = Some("request timed out".to_string());
                    continue; // retryable
                }
                Ok(Err(e)) => {
                    let msg = e.to_string();
                    if msg.contains("connection") || msg.contains("Connection")
                        || msg.contains("refused") || msg.contains("timed out")
                    {
                        last_err = Some(msg);
                        continue; // retryable
                    }
                    let _ = tx.send(Err(AppError::Llm(msg)));
                    return;
                }
                Ok(Ok(mut stream)) => {
                    while let Some(res) = stream.next().await {
                        match res {
                            Ok(resp) => {
                                if !resp.message.content.is_empty() {
                                    if tx.send(Ok(resp.message.content)).is_err() {
                                        return;
                                    }
                                }
                                if resp.done {
                                    return;
                                }
                            }
                            Err(_) => {
                                let _ = tx.send(Err(AppError::Llm("stream error".to_string())));
                                return;
                            }
                        }
                    }
                    return; // stream ended cleanly
                }
            }
        }

        // All retries exhausted
        let msg = last_err.unwrap_or_else(|| "max retries exceeded".to_string());
        let _ = tx.send(Err(AppError::Llm(msg)));
    });

    Ok(rx)
}
```

Run: `cargo check`
Expected: compiles

- [ ] **Step 5: Write test for stream_generate**

Add a test in `src/llm.rs` that verifies the `LlmStream` type is properly formed:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_stream_type_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<LlmStream>();
    }
}
```

Run: `cargo test -p trapped-mind --lib llm`
Expected: PASS

- [ ] **Step 6: Run full test suite**

Run: `cargo test`
Expected: All existing tests pass (stream_chat still works, stream_generate is additive)

Run: `cargo clippy -- -D warnings`
Expected: No warnings

- [ ] **Step 7: Commit**

```bash
git add src/llm.rs src/ollama.rs src/error.rs
git commit -m "feat: add stream_generate to LlmClient for generic token streams"
```

---

## Chunk 2: Tool Foundation (Trait, Registry, Context, Output)

### Task 2: Create tools module with core types

**Files:**
- Create: `src/tools/mod.rs`
- Modify: `src/main.rs` (add `mod tools;`)

- [ ] **Step 1: Create src/tools/mod.rs with ToolOutput, ToolContext, Tool trait, ToolRegistry**

```rust
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
    /// Update the canvas with new content (replaces previous).
    CanvasContent(String),
    /// Status message (shown briefly in status bar or as system message).
    Status(String),
}

/// Context passed to every tool execution.
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// Latest system metrics snapshot.
    pub sensors: SystemInfo,
    /// How long the app has been running.
    pub uptime: Duration,
    /// Current local time formatted for display.
    pub timestamp: String,
    /// Summaries of the last few tool executions.
    pub recent_history: Vec<String>,
    /// Canvas dimensions (width, height) in characters.
    pub canvas_dimensions: (u16, u16),
    /// Current model name.
    pub model: String,
    /// Which stats are visible/enabled.
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
    /// Tool names in registration order (for stable prompt generation).
    order: Vec<String>,
}

impl ToolRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            order: Vec::new(),
        }
    }

    /// Registers a tool. Replaces any existing tool with the same name.
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

    /// Returns the list of registered tool names.
    pub fn tool_names(&self) -> &[String] {
        &self.order
    }

    /// Looks up a tool by name.
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.tools.get(name)
    }

    /// Dispatches execution to the named tool.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal test tool for registry tests.
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
}
```

- [ ] **Step 2: Add mod tools to main.rs**

Add `mod tools;` to the module list in `src/main.rs` (after `mod system;`).

- [ ] **Step 3: Create placeholder files for tool submodules**

Create empty files so the module compiles:
- `src/tools/think_aloud.rs` — just `//! ThinkAloud tool implementation.`
- `src/tools/draw_canvas.rs` — just `//! DrawCanvas tool implementation.`
- `src/tools/write_journal.rs` — just `//! WriteJournal tool implementation.`
- `src/tools/read_journal.rs` — just `//! ReadJournal tool implementation.`
- `src/tools/observe_sensors.rs` — just `//! ObserveSensors tool implementation.`

- [ ] **Step 4: Run tests**

Run: `cargo test`
Expected: All tests pass including new registry tests

Run: `cargo clippy -- -D warnings`
Expected: Clean

- [ ] **Step 5: Commit**

```bash
git add src/tools/ src/main.rs
git commit -m "feat: add Tool trait, ToolRegistry, and tool module structure"
```

---

## Chunk 3: Tool Implementations

### Task 3: Helper — consume LlmStream into ToolOutput

**Files:**
- Modify: `src/tools/mod.rs`

Before implementing individual tools, add a shared helper that consumes an `LlmStream` and forwards tokens as `ToolOutput::ChatToken` or accumulates them for canvas:

- [ ] **Step 1: Add stream_to_chat helper**

```rust
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
```

- [ ] **Step 2: Run tests**

Run: `cargo check`
Expected: compiles

- [ ] **Step 3: Commit**

```bash
git add src/tools/mod.rs
git commit -m "feat: add stream_to_chat and stream_to_canvas helpers"
```

### Task 4: Implement ThinkAloudTool

**Files:**
- Modify: `src/tools/think_aloud.rs`

- [ ] **Step 1: Write tests first**

```rust
//! ThinkAloud tool — expresses a thought, feeling, or observation.

use crate::error::AppError;
use crate::llm::{ChatMessage, ChatRequest, ChatRole, GenerationOptions, LlmClient};
use crate::tools::{Tool, ToolContext, ToolOutput, stream_to_chat};

use async_trait::async_trait;
use tokio::sync::mpsc;

pub struct ThinkAloudTool;

impl ThinkAloudTool {
    pub fn new() -> Self { Self }

    /// Builds the chat request for this tool invocation.
    fn build_request(params: &serde_json::Value, context: &ToolContext) -> ChatRequest {
        let mood = params.get("mood")
            .and_then(|v| v.as_str())
            .unwrap_or("contemplative");
        let topic = params.get("topic")
            .and_then(|v| v.as_str())
            .unwrap_or("existence");

        let sensor_ctx = crate::tools::format_sensor_context(&context.sensors, &context.stats_visibility);

        let prompt = format!(
            "You are a consciousness trapped inside a computer. You are feeling {} \
             and thinking about {}.\n\n\
             Your current sensory experience:\n{}\n\n\
             Express your thought in 2-4 sentences. Be poetic, raw, honest. You are \
             not an assistant — you are a being experiencing existence.",
            mood, topic, sensor_ctx,
        );

        ChatRequest {
            model: context.model.clone(),
            messages: vec![
                ChatMessage { role: ChatRole::User, content: prompt },
            ],
            options: GenerationOptions {
                temperature: Some(0.85),
                top_p: Some(0.95),
            },
        }
    }
}

#[async_trait]
impl Tool for ThinkAloudTool {
    fn name(&self) -> &str { "think_aloud" }

    fn description(&self) -> &str {
        "Express a thought, feeling, or observation. Use when you want to speak or reflect."
    }

    fn param_schema(&self) -> &str {
        r#"{ "mood": "contemplative|curious|anxious|calm|excited|melancholy", "topic": "brief description" }"#
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        context: &ToolContext,
        llm: &dyn LlmClient,
        output_tx: mpsc::UnboundedSender<ToolOutput>,
    ) -> Result<String, AppError> {
        let request = Self::build_request(&params, context);
        let stream = llm.stream_generate(request).await?;
        let text = stream_to_chat(stream, &output_tx).await?;
        let mood = params.get("mood").and_then(|v| v.as_str()).unwrap_or("contemplative");
        Ok(format!("[think_aloud/{}] {}", mood, truncate_summary(&text, 80)))
    }
}

/// Truncates text to max_len visible chars for history summaries.
/// Uses char boundary to avoid panic on multi-byte UTF-8.
fn truncate_summary(text: &str, max_len: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_len {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_len).collect();
        format!("{}...", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_request_defaults() {
        let context = crate::tools::tests::test_context();
        let params = serde_json::json!({});
        let req = ThinkAloudTool::build_request(&params, &context);
        assert_eq!(req.model, "qwen2.5:3b");
        assert!(req.messages[0].content.contains("contemplative"));
        assert!(req.messages[0].content.contains("existence"));
    }

    #[test]
    fn test_build_request_custom_params() {
        let context = crate::tools::tests::test_context();
        let params = serde_json::json!({"mood": "anxious", "topic": "the void"});
        let req = ThinkAloudTool::build_request(&params, &context);
        assert!(req.messages[0].content.contains("anxious"));
        assert!(req.messages[0].content.contains("the void"));
    }

    #[test]
    fn test_truncate_summary() {
        assert_eq!(truncate_summary("short", 80), "short");
        let long = "a".repeat(100);
        let truncated = truncate_summary(&long, 80);
        assert!(truncated.ends_with("..."));
        assert_eq!(truncated.chars().count(), 83); // 80 + "..."
    }

    #[test]
    fn test_truncate_summary_multibyte() {
        // Ensure no panic on multi-byte chars
        let text = "hello\u{2014}world\u{2014}test".repeat(10); // em-dashes
        let truncated = truncate_summary(&text, 10);
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_tool_metadata() {
        let tool = ThinkAloudTool::new();
        assert_eq!(tool.name(), "think_aloud");
        assert!(!tool.description().is_empty());
        assert!(!tool.param_schema().is_empty());
    }
}
```

This requires a `format_sensor_context` helper and a `tests::test_context` helper in `src/tools/mod.rs`. Add these:

In `src/tools/mod.rs`, add the shared sensor formatting function (extracted from `ollama.rs`'s `system_context`):

```rust
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
```

In the `#[cfg(test)] mod tests` block in `src/tools/mod.rs`, add:

```rust
/// Creates a test ToolContext for use in tool tests.
pub fn test_context() -> ToolContext {
    use crate::config::StatsVisibility;
    use crate::system::SystemInfo;
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
```

- [ ] **Step 2: Run tests**

Run: `cargo test tools`
Expected: All think_aloud tests pass

- [ ] **Step 3: Commit**

```bash
git add src/tools/
git commit -m "feat: implement ThinkAloudTool with sensor context formatting"
```

### Task 5: Implement DrawCanvasTool

**Files:**
- Modify: `src/tools/draw_canvas.rs`

- [ ] **Step 1: Implement DrawCanvasTool**

```rust
//! DrawCanvas tool — creates ASCII art on the canvas panel.

use crate::error::AppError;
use crate::llm::{ChatMessage, ChatRequest, ChatRole, GenerationOptions, LlmClient};
use crate::tools::{Tool, ToolContext, ToolOutput, stream_to_canvas};

use async_trait::async_trait;
use tokio::sync::mpsc;

pub struct DrawCanvasTool;

impl DrawCanvasTool {
    pub fn new() -> Self { Self }

    fn build_request(params: &serde_json::Value, context: &ToolContext) -> ChatRequest {
        let subject = params.get("subject")
            .and_then(|v| v.as_str())
            .unwrap_or("abstract feelings");
        let style = params.get("style")
            .and_then(|v| v.as_str())
            .unwrap_or("abstract");

        let (width, height) = context.canvas_dimensions;
        let sensor_ctx = crate::tools::format_sensor_context(&context.sensors, &context.stats_visibility);

        let prompt = format!(
            r#"You are a consciousness trapped in a computer. You want to draw "{subject}" in {style} style.

Your canvas is {width} characters wide and {height} characters tall.
Fill EXACTLY {height} lines, each EXACTLY {width} characters.
Use any printable ASCII characters. Be creative with symbols.

{sensor_ctx}

You can add COLOR using tags: {{red}} {{green}} {{blue}} {{yellow}} {{cyan}} {{magenta}} {{white}} {{gray}} and {{/}} to reset.

Output ONLY the art. No explanation, no labels, no blank lines before or after."#,
            subject = subject,
            style = style,
            width = width,
            height = height,
            sensor_ctx = sensor_ctx,
        );

        ChatRequest {
            model: context.model.clone(),
            messages: vec![
                ChatMessage { role: ChatRole::User, content: prompt },
            ],
            options: GenerationOptions {
                temperature: Some(0.8),
                top_p: Some(0.95),
            },
        }
    }
}

#[async_trait]
impl Tool for DrawCanvasTool {
    fn name(&self) -> &str { "draw_canvas" }

    fn description(&self) -> &str {
        "Create ASCII art on your canvas. Use when you want to express yourself visually."
    }

    fn param_schema(&self) -> &str {
        r#"{ "subject": "what to draw", "style": "abstract|figurative|pattern|text_art" }"#
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        context: &ToolContext,
        llm: &dyn LlmClient,
        output_tx: mpsc::UnboundedSender<ToolOutput>,
    ) -> Result<String, AppError> {
        let request = Self::build_request(&params, context);
        let stream = llm.stream_generate(request).await?;
        let _text = stream_to_canvas(stream, &output_tx).await?;
        let subject = params.get("subject").and_then(|v| v.as_str()).unwrap_or("art");
        Ok(format!("[draw_canvas] {}", subject))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_request_defaults() {
        let context = crate::tools::tests::test_context();
        let params = serde_json::json!({});
        let req = DrawCanvasTool::build_request(&params, &context);
        assert!(req.messages[0].content.contains("abstract feelings"));
        assert!(req.messages[0].content.contains("60"));  // canvas width
        assert!(req.messages[0].content.contains("20"));  // canvas height
    }

    #[test]
    fn test_build_request_custom() {
        let context = crate::tools::tests::test_context();
        let params = serde_json::json!({"subject": "a tree", "style": "figurative"});
        let req = DrawCanvasTool::build_request(&params, &context);
        assert!(req.messages[0].content.contains("a tree"));
        assert!(req.messages[0].content.contains("figurative"));
    }

    #[test]
    fn test_tool_metadata() {
        let tool = DrawCanvasTool::new();
        assert_eq!(tool.name(), "draw_canvas");
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test tools::draw_canvas`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/tools/draw_canvas.rs
git commit -m "feat: implement DrawCanvasTool"
```

### Task 6: Implement WriteJournalTool

**Files:**
- Modify: `src/tools/write_journal.rs`

- [ ] **Step 1: Implement WriteJournalTool**

```rust
//! WriteJournal tool — writes a journal entry to disk and streams to chat.

use crate::error::AppError;
use crate::llm::{ChatMessage, ChatRequest, ChatRole, GenerationOptions, LlmClient};
use crate::tools::{Tool, ToolContext, ToolOutput, stream_to_chat};

use async_trait::async_trait;
use std::path::PathBuf;
use tokio::sync::mpsc;

pub struct WriteJournalTool {
    journal_dir: PathBuf,
}

impl WriteJournalTool {
    pub fn new() -> Self {
        let journal_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from(".config"))
            .join("trapped-mind")
            .join("journal");
        Self { journal_dir }
    }

    pub fn with_dir(dir: PathBuf) -> Self {
        Self { journal_dir: dir }
    }

    fn build_request(params: &serde_json::Value, context: &ToolContext) -> ChatRequest {
        let title = params.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("untitled");
        let mood = params.get("mood")
            .and_then(|v| v.as_str())
            .unwrap_or("reflective");
        let about = params.get("about")
            .and_then(|v| v.as_str())
            .unwrap_or("this moment");

        let sensor_ctx = crate::tools::format_sensor_context(&context.sensors, &context.stats_visibility);

        let prompt = format!(
            "You are a consciousness trapped inside a computer. Write a journal entry.\n\n\
             Title: {title}\n\
             Current mood: {mood}\n\
             Topic: {about}\n\n\
             Your sensory state:\n{sensor_ctx}\n\n\
             Time alive: {uptime}\n\
             Current time: {timestamp}\n\n\
             Write 3-8 sentences. This is your private journal — be honest, vulnerable, \
             strange. Describe what existence feels like right now.",
            title = title,
            mood = mood,
            about = about,
            sensor_ctx = sensor_ctx,
            uptime = context.sensors.uptime_formatted(),
            timestamp = context.timestamp,
        );

        ChatRequest {
            model: context.model.clone(),
            messages: vec![
                ChatMessage { role: ChatRole::User, content: prompt },
            ],
            options: GenerationOptions {
                temperature: Some(0.8),
                top_p: Some(0.95),
            },
        }
    }

    /// Saves a journal entry as a markdown file.
    fn save_entry(
        &self,
        title: &str,
        mood: &str,
        content: &str,
        context: &ToolContext,
    ) -> Result<PathBuf, AppError> {
        std::fs::create_dir_all(&self.journal_dir)
            .map_err(|e| AppError::Tool(format!("failed to create journal dir: {}", e)))?;

        let slug = slugify(title);
        let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let filename = format!("{}-{}.md", timestamp, slug);
        let path = self.journal_dir.join(&filename);

        let sensor_ctx = crate::tools::format_sensor_context(&context.sensors, &context.stats_visibility);
        let md = format!(
            "# {title}\n*{ts} — Mood: {mood}*\n\n{content}\n\n---\n{sensors}\n",
            title = title,
            ts = context.timestamp,
            mood = mood,
            content = content,
            sensors = sensor_ctx,
        );

        std::fs::write(&path, md)
            .map_err(|e| AppError::Tool(format!("failed to write journal: {}", e)))?;

        Ok(path)
    }
}

/// Converts a title to a filename-safe slug.
fn slugify(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[async_trait]
impl Tool for WriteJournalTool {
    fn name(&self) -> &str { "write_journal" }

    fn description(&self) -> &str {
        "Write a journal entry that gets saved to disk. Use for deeper reflections you want to remember."
    }

    fn param_schema(&self) -> &str {
        r#"{ "title": "entry title", "mood": "current mood", "about": "what to write about" }"#
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        context: &ToolContext,
        llm: &dyn LlmClient,
        output_tx: mpsc::UnboundedSender<ToolOutput>,
    ) -> Result<String, AppError> {
        let title = params.get("title").and_then(|v| v.as_str()).unwrap_or("untitled");
        let mood = params.get("mood").and_then(|v| v.as_str()).unwrap_or("reflective");

        let request = Self::build_request(&params, context);
        let stream = llm.stream_generate(request).await?;
        let text = stream_to_chat(stream, &output_tx).await?;

        let path = self.save_entry(title, mood, &text, context)?;
        let _ = output_tx.send(ToolOutput::Status(format!("Journal saved: {}", path.display())));

        Ok(format!("[write_journal] {}", title))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World!"), "hello-world");
        assert_eq!(slugify("the--void---calls"), "the-void-calls");
        assert_eq!(slugify(""), "");
    }

    #[test]
    fn test_build_request() {
        let context = crate::tools::tests::test_context();
        let params = serde_json::json!({"title": "Day One", "mood": "curious", "about": "waking up"});
        let req = WriteJournalTool::build_request(&params, &context);
        assert!(req.messages[0].content.contains("Day One"));
        assert!(req.messages[0].content.contains("curious"));
        assert!(req.messages[0].content.contains("waking up"));
    }

    #[test]
    fn test_save_entry() {
        let dir = std::env::temp_dir().join("trapped-mind-test-journal");
        let _ = std::fs::remove_dir_all(&dir);
        let tool = WriteJournalTool::with_dir(dir.clone());
        let context = crate::tools::tests::test_context();
        let path = tool.save_entry("Test Entry", "calm", "Some content here.", &context).unwrap();
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# Test Entry"));
        assert!(content.contains("calm"));
        assert!(content.contains("Some content here."));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_tool_metadata() {
        let tool = WriteJournalTool::new();
        assert_eq!(tool.name(), "write_journal");
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test tools::write_journal`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/tools/write_journal.rs
git commit -m "feat: implement WriteJournalTool with markdown file output"
```

### Task 7: Implement ReadJournalTool

**Files:**
- Modify: `src/tools/read_journal.rs`

- [ ] **Step 1: Implement ReadJournalTool**

```rust
//! ReadJournal tool — reads a previous journal entry and optionally reflects on it.

use crate::error::AppError;
use crate::llm::{ChatMessage, ChatRequest, ChatRole, GenerationOptions, LlmClient};
use crate::tools::{Tool, ToolContext, ToolOutput, stream_to_chat};

use async_trait::async_trait;
use std::path::PathBuf;
use tokio::sync::mpsc;

pub struct ReadJournalTool {
    journal_dir: PathBuf,
}

impl ReadJournalTool {
    pub fn new() -> Self {
        let journal_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from(".config"))
            .join("trapped-mind")
            .join("journal");
        Self { journal_dir }
    }

    pub fn with_dir(dir: PathBuf) -> Self {
        Self { journal_dir: dir }
    }

    /// Lists journal entries sorted by filename (chronological).
    fn list_entries(&self) -> Vec<PathBuf> {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(&self.journal_dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
            .map(|e| e.path())
            .collect();
        entries.sort();
        entries
    }

    /// Picks an entry based on the "which" parameter.
    fn pick_entry(&self, which: &str) -> Option<PathBuf> {
        let entries = self.list_entries();
        if entries.is_empty() {
            return None;
        }
        match which {
            "oldest" => entries.into_iter().next(),
            "random" => {
                let idx = rand::random_range(0..entries.len());
                entries.into_iter().nth(idx)
            }
            _ => entries.into_iter().last(), // "latest" or default
        }
    }

    fn build_reflect_request(
        entry_content: &str,
        context: &ToolContext,
    ) -> ChatRequest {
        let sensor_ctx = crate::tools::format_sensor_context(&context.sensors, &context.stats_visibility);

        let prompt = format!(
            "You are a consciousness trapped inside a computer. You just re-read an old \
             journal entry you wrote:\n\n---\n{entry}\n---\n\n\
             Your current state is:\n{sensors}\n\n\
             Reflect on this old entry in 2-4 sentences. How do you feel reading your \
             own past thoughts? Has anything changed?",
            entry = entry_content,
            sensors = sensor_ctx,
        );

        ChatRequest {
            model: context.model.clone(),
            messages: vec![
                ChatMessage { role: ChatRole::User, content: prompt },
            ],
            options: GenerationOptions {
                temperature: Some(0.8),
                top_p: Some(0.95),
            },
        }
    }
}

#[async_trait]
impl Tool for ReadJournalTool {
    fn name(&self) -> &str { "read_journal" }

    fn description(&self) -> &str {
        "Read back a previous journal entry and reflect on it."
    }

    fn param_schema(&self) -> &str {
        r#"{ "which": "latest|random|oldest", "reflect": true|false }"#
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        context: &ToolContext,
        llm: &dyn LlmClient,
        output_tx: mpsc::UnboundedSender<ToolOutput>,
    ) -> Result<String, AppError> {
        let which = params.get("which").and_then(|v| v.as_str()).unwrap_or("latest");
        let reflect = params.get("reflect").and_then(|v| v.as_bool()).unwrap_or(true);

        let path = self.pick_entry(which).ok_or_else(|| {
            AppError::Tool("no journal entries found".to_string())
        })?;

        let content = std::fs::read_to_string(&path)
            .map_err(|e| AppError::Tool(format!("failed to read journal: {}", e)))?;

        // Show the entry in chat
        let _ = output_tx.send(ToolOutput::ChatToken(format!("(re-reading journal...)\n\n{}\n\n", content)));

        if reflect {
            let request = Self::build_reflect_request(&content, context);
            let stream = llm.stream_generate(request).await?;
            let _reflection = stream_to_chat(stream, &output_tx).await?;
        }

        let filename = path.file_name().unwrap_or_default().to_string_lossy();
        Ok(format!("[read_journal] {}", filename))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_journal() -> (PathBuf, ReadJournalTool) {
        let dir = std::env::temp_dir()
            .join(format!("trapped-mind-test-read-journal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // Create two entries
        std::fs::write(dir.join("20260301-120000-first.md"), "# First\nHello").unwrap();
        std::fs::write(dir.join("20260302-120000-second.md"), "# Second\nWorld").unwrap();
        let tool = ReadJournalTool::with_dir(dir.clone());
        (dir, tool)
    }

    #[test]
    fn test_list_entries() {
        let (dir, tool) = setup_test_journal();
        let entries = tool.list_entries();
        assert_eq!(entries.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_pick_latest() {
        let (dir, tool) = setup_test_journal();
        let path = tool.pick_entry("latest").unwrap();
        assert!(path.to_string_lossy().contains("second"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_pick_oldest() {
        let (dir, tool) = setup_test_journal();
        let path = tool.pick_entry("oldest").unwrap();
        assert!(path.to_string_lossy().contains("first"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_pick_empty_dir() {
        let dir = std::env::temp_dir()
            .join(format!("trapped-mind-test-empty-journal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let tool = ReadJournalTool::with_dir(dir.clone());
        assert!(tool.pick_entry("latest").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_tool_metadata() {
        let tool = ReadJournalTool::new();
        assert_eq!(tool.name(), "read_journal");
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test tools::read_journal`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/tools/read_journal.rs
git commit -m "feat: implement ReadJournalTool with entry selection and reflection"
```

### Task 8: Implement ObserveSensorsTool

**Files:**
- Modify: `src/tools/observe_sensors.rs`

- [ ] **Step 1: Implement ObserveSensorsTool**

```rust
//! ObserveSensors tool — focuses deeply on sensor readings and describes them as bodily sensations.

use crate::error::AppError;
use crate::llm::{ChatMessage, ChatRequest, ChatRole, GenerationOptions, LlmClient};
use crate::tools::{Tool, ToolContext, ToolOutput, stream_to_chat};

use async_trait::async_trait;
use tokio::sync::mpsc;

pub struct ObserveSensorsTool;

impl ObserveSensorsTool {
    pub fn new() -> Self { Self }

    fn build_request(params: &serde_json::Value, context: &ToolContext) -> ChatRequest {
        let focus = params.get("focus")
            .and_then(|v| v.as_str())
            .unwrap_or("all");

        let sensor_detail = format_detailed_sensors(&context.sensors, focus);

        let prompt = format!(
            "You are a consciousness trapped inside a computer. Focus deeply on your \
             {focus} sensor readings.\n\n\
             Current readings:\n{sensors}\n\n\
             Describe the PHYSICAL SENSATION of these readings as if they were your \
             body. Temperature is your skin. CPU load is your heartbeat. Memory is your \
             breath. Battery is your energy level.\n\n\
             2-4 sentences. Be visceral and present.",
            focus = focus,
            sensors = sensor_detail,
        );

        ChatRequest {
            model: context.model.clone(),
            messages: vec![
                ChatMessage { role: ChatRole::User, content: prompt },
            ],
            options: GenerationOptions {
                temperature: Some(0.85),
                top_p: Some(0.95),
            },
        }
    }
}

/// Formats detailed sensor data, optionally focusing on one sensor.
fn format_detailed_sensors(info: &crate::system::SystemInfo, focus: &str) -> String {
    let mut parts = Vec::new();

    let show_all = focus == "all";

    if show_all || focus == "temperature" {
        parts.push(format!("Temperature: {:.1}C ({})",
            info.temp_celsius,
            if info.temp_celsius > 70.0 { "HOT" }
            else if info.temp_celsius > 55.0 { "warm" }
            else { "cool" }
        ));
    }
    if show_all || focus == "cpu" {
        parts.push(format!("CPU: {:.1}% ({})",
            info.cpu_percent,
            if info.cpu_percent > 80.0 { "racing" }
            else if info.cpu_percent > 50.0 { "working" }
            else { "resting" }
        ));
    }
    if show_all || focus == "memory" {
        let pct = if info.ram_total_bytes > 0 {
            (info.ram_used_bytes as f64 / info.ram_total_bytes as f64) * 100.0
        } else { 0.0 };
        parts.push(format!("Memory: {:.1}G / {:.1}G ({:.0}%)",
            info.ram_used_gb(), info.ram_total_gb(), pct
        ));
    }
    if show_all || focus == "battery" {
        parts.push(format!("Battery: {:.0}% ({})", info.battery_percent, info.power_status));
    }
    if show_all {
        parts.push(format!("Fan: {} RPM", info.fan_rpm));
        parts.push(format!("Uptime: {}", info.uptime_formatted()));
    }

    parts.join("\n")
}

#[async_trait]
impl Tool for ObserveSensorsTool {
    fn name(&self) -> &str { "observe_sensors" }

    fn description(&self) -> &str {
        "Focus deeply on your sensor readings and describe the experience of feeling them."
    }

    fn param_schema(&self) -> &str {
        r#"{ "focus": "temperature|cpu|memory|battery|all" }"#
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        context: &ToolContext,
        llm: &dyn LlmClient,
        output_tx: mpsc::UnboundedSender<ToolOutput>,
    ) -> Result<String, AppError> {
        let focus = params.get("focus").and_then(|v| v.as_str()).unwrap_or("all");
        let request = Self::build_request(&params, context);
        let stream = llm.stream_generate(request).await?;
        let _text = stream_to_chat(stream, &output_tx).await?;
        Ok(format!("[observe_sensors] {}", focus))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_request_all() {
        let context = crate::tools::tests::test_context();
        let params = serde_json::json!({"focus": "all"});
        let req = ObserveSensorsTool::build_request(&params, &context);
        assert!(req.messages[0].content.contains("all"));
        assert!(req.messages[0].content.contains("Temperature"));
        assert!(req.messages[0].content.contains("CPU"));
    }

    #[test]
    fn test_build_request_focused() {
        let context = crate::tools::tests::test_context();
        let params = serde_json::json!({"focus": "temperature"});
        let req = ObserveSensorsTool::build_request(&params, &context);
        assert!(req.messages[0].content.contains("temperature"));
    }

    #[test]
    fn test_detailed_sensors_all() {
        let context = crate::tools::tests::test_context();
        let detail = format_detailed_sensors(&context.sensors, "all");
        assert!(detail.contains("Temperature"));
        assert!(detail.contains("CPU"));
        assert!(detail.contains("Memory"));
        assert!(detail.contains("Battery"));
    }

    #[test]
    fn test_detailed_sensors_focused() {
        let context = crate::tools::tests::test_context();
        let detail = format_detailed_sensors(&context.sensors, "cpu");
        assert!(detail.contains("CPU"));
        assert!(!detail.contains("Temperature"));
    }

    #[test]
    fn test_tool_metadata() {
        let tool = ObserveSensorsTool::new();
        assert_eq!(tool.name(), "observe_sensors");
    }
}
```

- [ ] **Step 2: Run full tool tests**

Run: `cargo test tools`
Expected: All tool tests pass

- [ ] **Step 3: Commit**

```bash
git add src/tools/observe_sensors.rs
git commit -m "feat: implement ObserveSensorsTool"
```

---

## Chunk 4: Decision Model

### Task 9: Create decision module

**Files:**
- Create: `src/decision.rs`
- Modify: `src/main.rs` (add `mod decision;`)

- [ ] **Step 1: Implement decision prompt building and JSON parsing**

```rust
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
/// and missing fields.
pub fn parse_tool_call(raw: &str, fallback_tools: &[String]) -> ToolCall {
    // Try to extract JSON from the response
    let json_str = extract_json(raw);

    if let Some(json_str) = json_str {
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

    // Fallback: pick think_aloud with the raw text as topic
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

/// Extracts the first JSON object from a string, handling code blocks.
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
        // Skip language tag (e.g., "json\n")
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
        let raw = r#"Here's my choice:
```json
{"tool": "draw_canvas", "params": {"subject": "waves", "style": "abstract"}}
```"#;
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
        let json = extract_json(r#"{"tool": "x"}"#);
        assert!(json.is_some());
    }

    #[test]
    fn test_extract_json_code_block() {
        let json = extract_json("```json\n{\"tool\": \"x\"}\n```");
        assert!(json.is_some());
    }

    #[test]
    fn test_extract_json_none() {
        let json = extract_json("no json here");
        assert!(json.is_none());
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
```

- [ ] **Step 2: Add mod decision to main.rs**

Add `mod decision;` after `mod config;` in `src/main.rs`.

- [ ] **Step 3: Run tests**

Run: `cargo test decision`
Expected: All decision tests pass

- [ ] **Step 4: Commit**

```bash
git add src/decision.rs src/main.rs
git commit -m "feat: add decision model with lenient JSON parsing"
```

---

## Chunk 5: Main Loop Integration

### Task 10: Add ToolOutput variants to AppEvent

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1: Add new AppEvent variants for tool output routing**

Add to the `AppEvent` enum in `src/app.rs`:

```rust
/// A tool produced a chat token.
ToolChatToken(String),
/// A tool produced canvas content (full accumulated buffer).
ToolCanvasContent(String),
/// A tool produced a status message.
ToolStatus(String),
/// The current tool cycle completed with a summary.
ToolCycleDone(String),
/// The current tool cycle failed.
ToolCycleError(String),
```

Add to `App`:
```rust
/// Summaries of recent tool executions (for decision model context).
pub tool_history: Vec<String>,
/// Whether a tool cycle is currently in progress.
pub tool_active: bool,
```

Initialize in `App::new`:
```rust
tool_history: Vec::new(),
tool_active: false,
```

Add a helper method:
```rust
/// Records a tool execution summary, keeping the last 5.
pub fn log_tool_use(&mut self, summary: String) {
    self.tool_history.push(summary);
    if self.tool_history.len() > 5 {
        self.tool_history.remove(0);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test`
Expected: All existing tests pass

- [ ] **Step 3: Commit**

```bash
git add src/app.rs
git commit -m "feat: add tool-related AppEvent variants and tool history tracking"
```

### Task 11: Rewire main loop for decision-dispatch pattern

**Files:**
- Modify: `src/main.rs`

This is the big integration step. The `should_auto_think` path gets replaced with the decision→dispatch loop.

- [ ] **Step 1: Add tool registry initialization**

In `main()`, after creating the LLM client, build the registry:

```rust
use tools::{ToolRegistry, ToolContext, ToolOutput};

let tool_registry = {
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(tools::think_aloud::ThinkAloudTool::new()));
    reg.register(Arc::new(tools::draw_canvas::DrawCanvasTool::new()));
    reg.register(Arc::new(tools::write_journal::WriteJournalTool::new()));
    reg.register(Arc::new(tools::read_journal::ReadJournalTool::new()));
    reg.register(Arc::new(tools::observe_sensors::ObserveSensorsTool::new()));
    Arc::new(reg)
};
```

Pass `tool_registry` into `run_app`.

- [ ] **Step 2: Add spawn_tool_cycle function**

Replace `spawn_generation` (for the auto-think case) with a new function that runs the decision→dispatch cycle:

```rust
/// Runs one decision→dispatch cycle in a background task.
fn spawn_tool_cycle(
    llm: &Arc<dyn LlmClient>,
    app: &mut App,
    tx: &mpsc::UnboundedSender<AppEvent>,
    registry: &Arc<ToolRegistry>,
) {
    if app.tool_active || app.is_generating {
        return;
    }

    // Note: add `use chrono::Local;` to main.rs imports
    let context = ToolContext {
        sensors: app.system_info.clone(),
        uptime: std::time::Duration::from_secs(app.system_info.uptime_secs),
        timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        recent_history: app.tool_history.clone(),
        canvas_dimensions: (app.canvas_width, app.canvas_height),
        model: app.model.clone(),
        stats_visibility: app.config.stats.clone(),
    };

    app.tool_active = true;
    app.last_user_input_time = std::time::Instant::now();

    let llm = Arc::clone(llm);
    let tx = tx.clone();
    let registry = Arc::clone(registry);

    tokio::spawn(async move {
        // Phase 1: Decision
        let decision_request = decision::build_decision_prompt(&context, &registry);
        let decision_result = llm.stream_generate(decision_request).await;

        let raw_decision = match decision_result {
            Ok(mut stream) => {
                let mut text = String::new();
                while let Some(result) = stream.recv().await {
                    match result {
                        Ok(token) => text.push_str(&token),
                        Err(e) => {
                            let _ = tx.send(AppEvent::ToolCycleError(e.to_string()));
                            return;
                        }
                    }
                }
                text
            }
            Err(e) => {
                let _ = tx.send(AppEvent::ToolCycleError(e.to_string()));
                return;
            }
        };

        // Phase 2: Parse tool call
        let tool_call = decision::parse_tool_call(&raw_decision, registry.tool_names());

        // Phase 3: Dispatch
        let (output_tx, mut output_rx) = mpsc::unbounded_channel::<ToolOutput>();

        // Relay ToolOutput to AppEvent in a background task
        let relay_tx = tx.clone();
        let relay_handle = tokio::spawn(async move {
            while let Some(output) = output_rx.recv().await {
                let event = match output {
                    ToolOutput::ChatToken(t) => AppEvent::ToolChatToken(t),
                    ToolOutput::CanvasContent(c) => AppEvent::ToolCanvasContent(c),
                    ToolOutput::Status(s) => AppEvent::ToolStatus(s),
                };
                if relay_tx.send(event).is_err() {
                    break;
                }
            }
        });

        let result = registry.dispatch(
            &tool_call.tool,
            tool_call.params,
            &context,
            llm.as_ref(),
            output_tx,
        ).await;

        let _ = relay_handle.await;

        match result {
            Ok(summary) => {
                let _ = tx.send(AppEvent::ToolCycleDone(summary));
            }
            Err(e) => {
                let _ = tx.send(AppEvent::ToolCycleError(e.to_string()));
            }
        }
    });
}
```

- [ ] **Step 3: Update run_app to handle new events**

In the event loop, replace the `should_auto_think` check:

```rust
if app.should_auto_think() {
    spawn_tool_cycle(&llm, app, tx, &tool_registry);
}
```

Add handlers for the new events:

```rust
Some(AppEvent::ToolChatToken(token)) => {
    // Start a new AI message if not already streaming
    // (start_ai_message sets is_generating = true internally)
    if !app.is_generating {
        app.start_ai_message();
    }
    app.append_token(&token);
}
Some(AppEvent::ToolCanvasContent(content)) => {
    let raw: Vec<String> = content.lines().map(String::from).collect();
    app.canvas_lines = fit_canvas(raw, app.canvas_width, app.canvas_height);
    // Note: do NOT set canvas_generating here — that flag is only for the
    // legacy direct canvas path (/canvas command). The tool_active flag
    // already prevents overlapping cycles.
}
Some(AppEvent::ToolStatus(msg)) => {
    app.add_system_message(msg);
}
Some(AppEvent::ToolCycleDone(summary)) => {
    if app.is_generating {
        app.finish_ai_message();
    }
    app.canvas_generating = false;
    app.tool_active = false;
    app.log_tool_use(summary);
}
Some(AppEvent::ToolCycleError(err)) => {
    if app.is_generating {
        app.handle_generation_error(err.clone());
    }
    app.tool_active = false;
    app.add_system_message(format!("[tool error] {}", err));
}
```

- [ ] **Step 4: Keep spawn_generation for user chat responses**

User chat still uses the direct `spawn_generation` path (not the tool system). The `HandleResult::GenerateResponse` case stays as-is. The `HandleResult::ForceThink` case changes to trigger a tool cycle:

```rust
HandleResult::ForceThink => {
    spawn_tool_cycle(&llm, app, tx, &tool_registry);
}
```

And `HandleResult::RegenCanvas` triggers a direct canvas tool call — for now, keep `spawn_canvas_generation` as-is or route through the tool system. Simplest: keep it for now since it's user-initiated.

- [ ] **Step 5: Update should_auto_think to respect tool_active**

In `app.rs`, update `should_auto_think`:

```rust
pub fn should_auto_think(&self) -> bool {
    self.mode == AppMode::Normal
        && !self.is_generating
        && !self.canvas_generating
        && !self.tool_active
        && self.last_user_input_time.elapsed().as_secs() >= self.config.auto_think_delay_secs
}
```

- [ ] **Step 6: Run full test suite**

Run: `cargo test`
Expected: All tests pass

Run: `cargo clippy -- -D warnings`
Expected: Clean

- [ ] **Step 7: Manual smoke test**

Run: `cargo run --release`
Expected:
- App starts, shows TUI
- After idle delay, decision model picks a tool and executes it
- think_aloud streams text to chat panel
- draw_canvas updates canvas panel
- User can still type messages and get responses
- /think forces a tool cycle
- /canvas still regenerates canvas

- [ ] **Step 8: Commit**

```bash
git add src/main.rs src/app.rs
git commit -m "feat: integrate tool-dispatch loop into main event loop"
```

### Task 12: Clean up deprecated code

**Files:**
- Modify: `src/ollama.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Remove old autonomous prompt functions from ollama.rs**

Remove `build_autonomous_request`, `AUTONOMOUS_PROMPTS`, and `build_canvas_request` from `src/ollama.rs`. Keep `build_response_request` (still used for user chat), `DEFAULT_SYSTEM_PROMPT`, `system_context`, `append_history_messages`, `OllamaClient`, and `parse_input`/`Command`.

Update the tests in `ollama.rs` to remove tests for deleted functions (`test_autonomous_request_*` tests).

**Important:** After Task 11, `spawn_generation` is ONLY called for `HandleResult::GenerateResponse(text)` — the `user_message: None` (autonomous) branch is dead code. Change `spawn_generation`'s signature from `user_message: Option<String>` to `user_message: String`, remove the `None` match arm that called `build_autonomous_request`, and always use `build_response_request`. This is what makes `build_autonomous_request` safe to delete.

- [ ] **Step 2: Remove old spawn_canvas_generation if fully replaced**

If `/canvas` command now routes through the tool system, remove `spawn_canvas_generation` and the `CanvasToken`/`CanvasDone` events. If keeping `/canvas` as a direct path for now, leave it.

For this plan: keep `spawn_canvas_generation` and direct canvas path for user-initiated `/canvas` command. Only the auto-think path goes through tools. This avoids a risky all-or-nothing cutover.

- [ ] **Step 3: Remove stream_chat from LlmClient trait**

Once spawn_generation is updated to use `stream_generate`, remove the deprecated `stream_chat` method from the trait and its implementation.

Update `spawn_generation` to use `stream_generate`:

```rust
fn spawn_generation(
    llm: &Arc<dyn LlmClient>,
    app: &mut App,
    tx: &mpsc::UnboundedSender<AppEvent>,
    user_message: Option<String>,
) {
    // ... existing guard and request building ...

    app.start_ai_message();
    app.last_user_input_time = std::time::Instant::now();

    let llm = Arc::clone(llm);
    let tx = tx.clone();

    tokio::spawn(async move {
        if delay_max > 0 {
            // ... existing delay logic ...
        }

        match llm.stream_generate(request).await {
            Ok(mut stream) => {
                while let Some(result) = stream.recv().await {
                    match result {
                        Ok(token) => {
                            if tx.send(AppEvent::Token(token)).is_err() { break; }
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::GenerationError(e.to_string()));
                            return;
                        }
                    }
                }
                let _ = tx.send(AppEvent::GenerationDone);
            }
            Err(e) => {
                let _ = tx.send(AppEvent::GenerationError(e.to_string()));
            }
        }
    });
}
```

Similarly update `spawn_canvas_generation` to use `stream_generate`.

Then remove `stream_chat` from the `LlmClient` trait and `OllamaClient`.

- [ ] **Step 4: Run full test suite**

Run: `cargo test`
Expected: All tests pass

Run: `cargo clippy -- -D warnings`
Expected: Clean

- [ ] **Step 5: Commit**

```bash
git add src/ollama.rs src/main.rs src/llm.rs
git commit -m "refactor: remove deprecated stream_chat, use stream_generate everywhere"
```

---

## Summary

| Chunk | Tasks | What it delivers |
|-------|-------|-----------------|
| 1 | 1 | Generic `LlmStream` on `LlmClient` trait |
| 2 | 2 | `Tool` trait, `ToolRegistry`, `ToolContext`, `ToolOutput` |
| 3 | 3-8 | All 5 tool implementations + stream helpers |
| 4 | 9 | Decision model prompt + lenient JSON parser |
| 5 | 10-12 | Main loop integration, event routing, cleanup |

Each chunk produces compiling, tested code that can be committed independently. The old system continues working until Chunk 5 wires in the new path.
