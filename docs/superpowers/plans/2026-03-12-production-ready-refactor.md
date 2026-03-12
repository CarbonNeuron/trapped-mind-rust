# Production-Ready Refactor Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor TrappedMind into a production-ready Rust project with proper error handling, structured logging, an extensible LLM client trait, retry/timeout logic, graceful shutdown, and config validation.

**Architecture:** The existing module structure stays intact. We add `error.rs` for a `thiserror` error enum, `llm.rs` for an async `LlmClient` trait with a backend-agnostic `ChatRequest` type, and make `OllamaClient` the concrete implementation with retry/backoff. Logging goes to a file via `tracing`. Graceful shutdown via tokio signals. Config validation at startup.

**Tech Stack:** Rust, thiserror, anyhow, tracing/tracing-appender/tracing-subscriber, async-trait, tokio (signals), ratatui, ollama-rs

**Spec:** `docs/superpowers/specs/2026-03-12-production-ready-refactor-design.md`

**Existing tests must keep passing throughout. Run `cargo test` and `cargo clippy -- -D warnings` after every commit.**

---

## Chunk 1: Error Types & Dependencies

### Task 1: Add new dependencies to Cargo.toml

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Update Cargo.toml with new deps and metadata**

Add to `[dependencies]`:
```toml
thiserror = "2"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-appender = "0.2"
async-trait = "0.1"
```

Add metadata fields to `[package]`:
```toml
authors = ["TrappedMind Contributors"]
description = "AI consciousness trapped inside a laptop - a TUI experience"
license = "MIT"
repository = "https://github.com/user/trapped-mind-rust"
```

Add release profile:
```toml
[profile.release]
strip = true
lto = true
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "refactor: add production dependencies and Cargo.toml metadata"
```

### Task 2: Create error module

**Files:**
- Create: `src/error.rs`
- Modify: `src/main.rs` (add `mod error;`)

- [ ] **Step 1: Create src/error.rs**

```rust
//! Application-wide error types.

/// Errors that can occur in the TrappedMind application.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Configuration loading or validation failure.
    #[error("config error: {0}")]
    Config(String),

    /// Conversation history I/O failure.
    #[error("history error: {0}")]
    History(String),

    /// LLM client communication failure.
    #[error("llm error: {0}")]
    Llm(String),

    /// System sensor read failure.
    #[error("system error: {0}")]
    System(String),

    /// Generic I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
```

- [ ] **Step 2: Add `mod error;` to main.rs**

Add `mod error;` after the existing mod declarations in `src/main.rs`.

- [ ] **Step 3: Verify compilation and tests**

Run: `cargo test`
Expected: all existing tests pass, no warnings

- [ ] **Step 4: Commit**

```bash
git add src/error.rs src/main.rs
git commit -m "refactor: add error types with thiserror"
```

### Task 3: Wire anyhow into main() and propagate errors

**Files:**
- Modify: `src/main.rs`
- Modify: `src/config.rs`
- Modify: `src/history.rs`

- [ ] **Step 1: Change main() to return anyhow::Result**

In `src/main.rs`, change:
- `async fn main() -> std::io::Result<()>` to `async fn main() -> anyhow::Result<()>`
- `async fn run_app(...)  -> std::io::Result<()>` to `async fn run_app(...) -> anyhow::Result<()>`
- Add `use anyhow::Context;` for `.context()` calls where helpful

- [ ] **Step 2: Audit and fix unwrap/expect in non-test code**

Scan all source files for `unwrap()` and `expect()` outside of `#[cfg(test)]` blocks. Key locations:

In `src/config.rs`:
- `AppConfig::default()` line 102-103: `dirs::home_dir().unwrap_or_else(...)` — this is fine, already has fallback
- `AppConfig::load()` lines 128-134: `dirs::home_dir().unwrap_or_else(...)` — fine, already has fallback

In `src/ollama.rs`:
- Line 68-69: `.unwrap_or(&AUTONOMOUS_PROMPTS[0])` — fine, guaranteed non-empty array

In `src/app.rs`:
- Line 369: `.unwrap_or(0)` — fine
- Line 379: `.unwrap_or(0)` — fine

In `src/system.rs`:
- Line 281: `.unwrap_or("")` — fine

In `src/ui/input.rs`:
- Line 32: `.unwrap_or(after_cursor.len())` — fine

All existing unwrap_or patterns are safe. No bare `unwrap()` or `expect()` found in non-test code. No changes needed here.

- [ ] **Step 3: Run tests**

Run: `cargo test && cargo clippy -- -D warnings`
Expected: all pass

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "refactor: use anyhow for main error handling"
```

## Chunk 2: Structured Logging

### Task 4: Add tracing initialization

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add tracing setup to main()**

At the top of `main()`, before any other logic, add log file initialization:

```rust
use tracing_appender::rolling;
use tracing_subscriber::{fmt, EnvFilter};

// Set up file logging (TUI owns stdout/stderr)
let log_dir = dirs::config_dir()
    .unwrap_or_else(|| std::path::PathBuf::from(".config"))
    .join("trapped-mind");
let file_appender = rolling::daily(&log_dir, "trapped-mind.log");
let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
tracing_subscriber::fmt()
    .with_writer(non_blocking)
    .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
    .with_target(false)
    .init();

tracing::info!("trapped-mind starting");
```

Keep `_guard` alive for the duration of main (it flushes on drop).

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: compiles

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "refactor: initialize tracing file logger"
```

### Task 5: Replace silent error swallowing with tracing calls

**Files:**
- Modify: `src/config.rs`
- Modify: `src/history.rs`
- Modify: `src/system.rs`

- [ ] **Step 1: Add tracing to config.rs**

In `AppConfig::load()`, replace silent error ignoring:
- Line 121-122: When `read_to_string` fails, add `tracing::warn!("failed to read config file: {}", e);`
- Line 122: When `toml::from_str` fails, add `tracing::warn!("failed to parse config file: {}", e);`

In `AppConfig::save()`:
- Line 169: When `create_dir_all` fails, change `let _ =` to log: `if let Err(e) = ... { tracing::warn!("failed to create config directory: {}", e); }`
- Line 171-172: When `toml::to_string_pretty` or `fs::write` fails, log similarly

- [ ] **Step 2: Add tracing to history.rs**

In `HistoryManager::load_from_file()`:
- The `Err(_) => return Vec::new()` on file open is fine (file may not exist yet), but add `tracing::debug!` for it

In `HistoryManager::save()`:
- Line 100: `let _ = fs::create_dir_all` → log on error
- Line 103-104: `Err(_) => return` → `Err(e) => { tracing::warn!("failed to save history: {}", e); return; }`
- Line 108-109: `let _ = writeln!` → log on error

- [ ] **Step 3: Add tracing to system.rs**

Add `tracing::warn!` for sensor read failures in `SystemReader::read()` where values fall back to simulator.

- [ ] **Step 4: Run tests**

Run: `cargo test && cargo clippy -- -D warnings`
Expected: all pass

- [ ] **Step 5: Commit**

```bash
git add src/config.rs src/history.rs src/system.rs
git commit -m "refactor: add structured logging with tracing"
```

## Chunk 3: LLM Client Trait

### Task 6: Create the LlmClient trait and ChatRequest types

**Files:**
- Create: `src/llm.rs`
- Modify: `src/main.rs` (add `mod llm;`)

- [ ] **Step 1: Create src/llm.rs with trait and types**

```rust
//! Backend-agnostic LLM client trait and request types.

use crate::app::AppEvent;
use crate::error::AppError;
use async_trait::async_trait;
use tokio::sync::mpsc;

/// Generation options for LLM requests.
#[derive(Debug, Clone)]
pub struct GenerationOptions {
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
}

impl Default for GenerationOptions {
    fn default() -> Self {
        Self { temperature: None, top_p: None }
    }
}

/// A single message in a chat conversation.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

/// Role of a message sender in a chat conversation.
#[derive(Debug, Clone, PartialEq)]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

/// A backend-agnostic chat request.
#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub options: GenerationOptions,
}

/// Trait for LLM backends that support streaming chat.
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Streams a chat completion, sending tokens through the channel.
    async fn stream_chat(
        &self,
        request: ChatRequest,
        tx: mpsc::UnboundedSender<AppEvent>,
    ) -> Result<(), AppError>;

    /// Pulls/downloads a model by name.
    async fn pull_model(&self, model: &str) -> Result<(), AppError>;
}
```

- [ ] **Step 2: Add `mod llm;` to main.rs**

- [ ] **Step 3: Verify compilation**

Run: `cargo check`
Expected: compiles

- [ ] **Step 4: Commit**

```bash
git add src/llm.rs src/main.rs
git commit -m "refactor: extract LlmClient trait and ChatRequest types"
```

### Task 7: Implement OllamaClient with retry and timeout

**Files:**
- Modify: `src/ollama.rs`
- Modify: `src/config.rs` (add `ollama_timeout_secs` field)
- Modify: `src/main.rs` (use `Arc<dyn LlmClient>` in spawn_generation)

- [ ] **Step 1: Add `ollama_timeout_secs` to config**

In `src/config.rs`:
- Add `pub ollama_timeout_secs: u64` to `AppConfig` (default 60)
- Add `ollama_timeout_secs: Option<u64>` to `FileConfig`
- Wire it through `load()` and `save()`

- [ ] **Step 2: Add OllamaClient struct to ollama.rs**

```rust
use crate::llm::{LlmClient, ChatRequest, ChatRole, GenerationOptions};
use crate::app::AppEvent;
use crate::error::AppError;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

/// Ollama-backed LLM client with retry and timeout support.
pub struct OllamaClient {
    client: ollama_rs::Ollama,
    timeout_secs: u64,
}

impl OllamaClient {
    pub fn new(host: &str, port: u16, timeout_secs: u64) -> Self {
        Self {
            client: ollama_rs::Ollama::new(host, port),
            timeout_secs,
        }
    }

    /// Converts our ChatRequest into an ollama-rs ChatMessageRequest.
    fn to_ollama_request(request: &ChatRequest) -> ollama_rs::generation::chat::request::ChatMessageRequest {
        let messages: Vec<ollama_rs::generation::chat::ChatMessage> = request.messages.iter().map(|m| {
            match m.role {
                ChatRole::System => ollama_rs::generation::chat::ChatMessage::system(m.content.clone()),
                ChatRole::User => ollama_rs::generation::chat::ChatMessage::user(m.content.clone()),
                ChatRole::Assistant => ollama_rs::generation::chat::ChatMessage::assistant(m.content.clone()),
            }
        }).collect();

        let mut req = ollama_rs::generation::chat::request::ChatMessageRequest::new(
            request.model.clone(), messages,
        );
        if request.options.temperature.is_some() || request.options.top_p.is_some() {
            let mut opts = ollama_rs::models::ModelOptions::default();
            if let Some(t) = request.options.temperature { opts = opts.temperature(t); }
            if let Some(p) = request.options.top_p { opts = opts.top_p(p); }
            req.options = Some(opts);
        }
        req
    }

    /// Streams tokens from a single attempt (no retry).
    async fn stream_once(
        &self,
        ollama_request: &ollama_rs::generation::chat::request::ChatMessageRequest,
        tx: &mpsc::UnboundedSender<AppEvent>,
    ) -> Result<(), AppError> {
        let stream_future = self.client.send_chat_messages_stream(ollama_request.clone());
        let mut stream = tokio::time::timeout(
            Duration::from_secs(self.timeout_secs),
            stream_future,
        )
        .await
        .map_err(|_| AppError::Llm("request timed out".to_string()))?
        .map_err(|e| AppError::Llm(e.to_string()))?;

        while let Some(res) = stream.next().await {
            match res {
                Ok(resp) => {
                    let token = resp.message.content;
                    if !token.is_empty() {
                        if tx.send(AppEvent::Token(token)).is_err() {
                            return Ok(());
                        }
                    }
                    if resp.done {
                        let _ = tx.send(AppEvent::GenerationDone);
                        return Ok(());
                    }
                }
                Err(_) => {
                    return Err(AppError::Llm("stream error".to_string()));
                }
            }
        }
        let _ = tx.send(AppEvent::GenerationDone);
        Ok(())
    }

    /// Returns true if the error is a connection error worth retrying.
    fn is_retryable(err: &AppError) -> bool {
        match err {
            AppError::Llm(msg) => {
                msg.contains("connection") || msg.contains("Connection")
                    || msg.contains("timed out") || msg.contains("timeout")
                    || msg.contains("refused")
            }
            _ => false,
        }
    }
}

#[async_trait]
impl LlmClient for OllamaClient {
    async fn stream_chat(
        &self,
        request: ChatRequest,
        tx: mpsc::UnboundedSender<AppEvent>,
    ) -> Result<(), AppError> {
        let ollama_request = Self::to_ollama_request(&request);
        let mut last_err = None;

        for attempt in 0..3u32 {
            if attempt > 0 {
                let backoff = Duration::from_secs(1 << attempt); // 2s, 4s
                tracing::warn!("retrying LLM request (attempt {}), backoff {:?}", attempt + 1, backoff);
                tokio::time::sleep(backoff).await;
            }

            match self.stream_once(&ollama_request, &tx).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    if Self::is_retryable(&e) {
                        tracing::warn!("retryable LLM error: {}", e);
                        last_err = Some(e);
                        continue;
                    }
                    // Non-retryable: check if model needs pulling
                    let err_str = e.to_string();
                    if err_str.contains("not found") || err_str.contains("pull") {
                        return Err(e); // Caller handles auto-pull
                    }
                    return Err(e);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| AppError::Llm("max retries exceeded".to_string())))
    }

    async fn pull_model(&self, model: &str) -> Result<(), AppError> {
        self.client
            .pull_model(model.to_string(), false)
            .await
            .map_err(|e| AppError::Llm(format!("failed to pull model: {}", e)))?;
        Ok(())
    }
}
```

- [ ] **Step 3: Update spawn_generation in main.rs**

Change `spawn_generation` to accept `Arc<dyn LlmClient>` instead of `&Ollama`. Update the function signature and internals:

```rust
fn spawn_generation(
    llm: &Arc<dyn crate::llm::LlmClient>,
    app: &mut App,
    tx: &mpsc::UnboundedSender<AppEvent>,
    user_message: Option<String>,
) {
    // ... existing guard and request building ...

    // Build ChatRequest using our types instead of ollama-rs types
    let chat_request = crate::ollama::build_chat_request(
        &info, &history_entries, user_message.as_deref(), &model,
        sys_prompt.as_deref(), &stats_vis,
    );

    // ... existing delay logic ...

    let llm = Arc::clone(llm);
    let tx = tx.clone();
    tokio::spawn(async move {
        // ... thinking delay ...
        match llm.stream_chat(chat_request.clone(), tx.clone()).await {
            Ok(()) => {}
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("not found") || err_str.contains("pull") {
                    let _ = tx.send(AppEvent::GenerationError(format!(
                        "Model not found, pulling {}...", chat_request.model
                    )));
                    match llm.pull_model(&chat_request.model).await {
                        Ok(()) => {
                            let _ = tx.send(AppEvent::GenerationError(format!(
                                "Model {} pulled, retrying...", chat_request.model
                            )));
                            if let Err(e) = llm.stream_chat(chat_request, tx.clone()).await {
                                let _ = tx.send(AppEvent::GenerationError(format!("LLM error: {}", e)));
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::GenerationError(format!("{}", e)));
                        }
                    }
                } else {
                    let _ = tx.send(AppEvent::GenerationError(format!("LLM error: {}", e)));
                }
            }
        }
    });
}
```

- [ ] **Step 4: Refactor prompt builders to return ChatRequest**

In `src/ollama.rs`, change `build_autonomous_request` and `build_response_request` to return `crate::llm::ChatRequest` instead of `ollama_rs::generation::chat::request::ChatMessageRequest`. Or add a new unified `build_chat_request` function that returns our type. Keep the prompt-building logic identical.

- [ ] **Step 5: Update main() to create Arc<OllamaClient>**

```rust
let llm: Arc<dyn crate::llm::LlmClient> = Arc::new(
    crate::ollama::OllamaClient::new(&config.ollama_host, config.ollama_port, config.ollama_timeout_secs)
);
```

Pass `&llm` to `run_app` and `handle_key` instead of `&ollama`.

- [ ] **Step 6: Remove old stream_chat function from main.rs**

The standalone `stream_chat` function in `main.rs` is now replaced by `OllamaClient::stream_once`.

- [ ] **Step 7: Run tests**

Run: `cargo test && cargo clippy -- -D warnings`
Expected: all pass. Fix any clippy warnings.

- [ ] **Step 8: Commit**

```bash
git add src/ollama.rs src/llm.rs src/main.rs src/config.rs Cargo.toml Cargo.lock
git commit -m "refactor: implement OllamaClient with retry and timeout"
```

## Chunk 4: Config Validation & Graceful Shutdown

### Task 8: Add config validation

**Files:**
- Modify: `src/config.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write failing tests for validation**

In `src/config.rs` tests module, add:

```rust
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
    config.ollama_host = "localhost".to_string(); // missing http://
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
```

- [ ] **Step 2: Run tests to see them fail**

Run: `cargo test config::tests::test_validate`
Expected: FAIL — `validate` method doesn't exist yet

- [ ] **Step 3: Implement validate()**

In `src/config.rs`, add to `impl AppConfig`:

```rust
/// Validates the configuration, returning an error if any values are invalid.
pub fn validate(&self) -> Result<(), crate::error::AppError> {
    if self.ollama_port == 0 {
        return Err(crate::error::AppError::Config("ollama_port must be non-zero".to_string()));
    }
    if !self.ollama_host.starts_with("http://") && !self.ollama_host.starts_with("https://") {
        return Err(crate::error::AppError::Config(
            format!("ollama_host must start with http:// or https://, got: {}", self.ollama_host)
        ));
    }
    if self.max_history == 0 {
        return Err(crate::error::AppError::Config("max_history must be > 0".to_string()));
    }
    if self.auto_think_delay_secs == 0 {
        return Err(crate::error::AppError::Config("auto_think_delay must be > 0".to_string()));
    }
    if self.think_delay_min_ms > self.think_delay_max_ms {
        return Err(crate::error::AppError::Config(
            format!("think_delay_min_ms ({}) must be <= think_delay_max_ms ({})",
                self.think_delay_min_ms, self.think_delay_max_ms)
        ));
    }
    Ok(())
}
```

- [ ] **Step 4: Call validate() in main()**

After `AppConfig::load(&cli)`, add:
```rust
config.validate().context("invalid configuration")?;
```

- [ ] **Step 5: Run tests**

Run: `cargo test && cargo clippy -- -D warnings`
Expected: all pass

- [ ] **Step 6: Commit**

```bash
git add src/config.rs src/main.rs
git commit -m "refactor: add config validation at startup"
```

### Task 9: Add graceful shutdown

**Files:**
- Modify: `src/main.rs`
- Modify: `src/app.rs` (add `AppEvent::Shutdown`)

- [ ] **Step 1: Add Shutdown variant to AppEvent**

In `src/app.rs`, add to `AppEvent` enum:
```rust
/// Graceful shutdown requested (Ctrl+C / SIGTERM).
Shutdown,
```

- [ ] **Step 2: Spawn signal handler tasks in main()**

After the animation tick spawn, add:

```rust
// Graceful shutdown on Ctrl+C
let tx_ctrlc = tx.clone();
tokio::spawn(async move {
    if tokio::signal::ctrl_c().await.is_ok() {
        let _ = tx_ctrlc.send(AppEvent::Shutdown);
    }
});

// Graceful shutdown on SIGTERM (Unix only)
#[cfg(unix)]
{
    let tx_term = tx.clone();
    tokio::spawn(async move {
        use tokio::signal::unix::{signal, SignalKind};
        if let Ok(mut sig) = signal(SignalKind::terminate()) {
            sig.recv().await;
            let _ = tx_term.send(AppEvent::Shutdown);
        }
    });
}
```

- [ ] **Step 3: Handle Shutdown event in run_app()**

In the match block of `run_app()`, add before `None => break`:

```rust
Some(AppEvent::Shutdown) => {
    tracing::info!("shutdown signal received");
    app.log_shutdown();
    app.config.save();
    break;
}
```

- [ ] **Step 4: Remove Ctrl+C from handle_key (it's now handled by signal)**

In `handle_key`, keep the Ctrl+C handler but have it send Shutdown through the channel instead of directly setting should_quit. Actually — keep the Ctrl+C key handler as a fast-path (it's more responsive than the signal handler for interactive use), but make it also call `log_shutdown()` and `config.save()` consistently. The signal handler is a safety net for non-interactive shutdown.

Actually, simplest approach: keep the existing Ctrl+C key handler as-is (it already calls `log_shutdown()`). The signal handler is a backup for when the key event loop isn't responsive.

- [ ] **Step 5: Run tests**

Run: `cargo test && cargo clippy -- -D warnings`
Expected: all pass

- [ ] **Step 6: Commit**

```bash
git add src/main.rs src/app.rs
git commit -m "refactor: add graceful shutdown via signals"
```

## Chunk 5: Sensor Resilience & Final Polish

### Task 10: Add last-known-value fallback for sensor failures

**Files:**
- Modify: `src/system.rs`

- [ ] **Step 1: Add last-known fields to SystemReader**

Add fields to `SystemReader`:
```rust
last_temp: f32,
last_battery: (f32, String),
last_fan: u32,
last_networks: Vec<NetworkInterface>,
```

Initialize them with default values in `new()`.

- [ ] **Step 2: Wrap sensor reads with fallback**

In `read()`, for each real sensor read that could fail mid-session, catch the failure and use the last known value:

For temperature: if `components.refresh()` + iteration yields no value when `has_real_temp` is true, use `self.last_temp` and log a warning.

For battery: `read_real_battery()` already falls back to sim values on error — update it to fall back to `self.last_battery` instead.

For fan: `probe_fan_speed()` already returns `Option` — when it returns `None` on a system that had real fans (`has_real_fan`), use `self.last_fan`.

For networks: `read_real_networks()` already falls back to a fake interface — this is fine as-is.

After successful reads, update the `last_*` fields.

- [ ] **Step 3: Run tests**

Run: `cargo test && cargo clippy -- -D warnings`
Expected: all pass

- [ ] **Step 4: Commit**

```bash
git add src/system.rs
git commit -m "refactor: add last-known-value fallback for sensor failures"
```

### Task 11: Add remaining unit tests

**Files:**
- Modify: `src/error.rs` (add tests)
- Modify: `src/ollama.rs` (add tests for ChatRequest building)
- Modify: `src/config.rs` (validation tests already added in Task 8)

- [ ] **Step 1: Add error type tests**

In `src/error.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let e = AppError::Config("bad port".to_string());
        assert_eq!(e.to_string(), "config error: bad port");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let app_err: AppError = io_err.into();
        assert!(app_err.to_string().contains("gone"));
    }

    #[test]
    fn test_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AppError>();
    }
}
```

- [ ] **Step 2: Add ChatRequest building tests**

In `src/ollama.rs` tests, add tests that verify the new `build_chat_request` function returns correct `ChatRequest` types with proper role mapping.

- [ ] **Step 3: Run full test suite**

Run: `cargo test && cargo clippy -- -D warnings`
Expected: all pass, zero warnings

- [ ] **Step 4: Commit**

```bash
git add src/error.rs src/ollama.rs
git commit -m "test: add unit tests for error types and prompt building"
```

### Task 12: Update README

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update README**

Add/update sections:
- **Build Instructions**: cargo build, cargo build --release, cargo test, cargo clippy
- **Configuration Reference**: document all config.toml fields including new `ollama_timeout_secs`
- **Architecture Overview**: update the module tree to include `error.rs` and `llm.rs`, mention LlmClient trait
- **Logging**: document log file location
- Keep existing content (features, screen layout, commands, sensor fallback)

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: update README with build instructions and architecture overview"
```

### Task 13: Final clippy/warning sweep

**Files:**
- Any files with remaining warnings

- [ ] **Step 1: Run full quality check**

```bash
cargo clippy -- -D warnings 2>&1
cargo test 2>&1
cargo build --release 2>&1
```

- [ ] **Step 2: Fix any remaining issues**

Address any clippy lints, unused imports, dead code warnings.

- [ ] **Step 3: Final commit**

```bash
git add -A
git commit -m "refactor: production-ready cleanup"
```
