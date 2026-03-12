# Production-Ready Refactor Design

## Overview

Refactor the TrappedMind TUI application into a production-ready Rust project. The codebase is ~1300 lines across 12 source files with good module separation already in place. This refactor adds proper error handling, structured logging, an LLM client trait for extensibility, retry/timeout logic, graceful shutdown, config validation, and build hardening.

## Constraints

- Do NOT change the core concept, TUI layout, personality/prompt system, or sensor fallback behavior
- All existing tests must continue passing
- `cargo clippy -- -D warnings` must pass with zero warnings
- `cargo test` must pass cleanly

## Section 1: Error Handling & Logging

### Error Types

New `src/error.rs` module with `thiserror`-derived enum:

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("config error: {0}")]
    Config(String),
    #[error("history error: {0}")]
    History(String),
    #[error("ollama error: {0}")]
    Ollama(String),
    #[error("system error: {0}")]
    System(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
```

- `main()` returns `anyhow::Result<()>`
- Internal modules use `AppError` where specific handling is needed, `anyhow` where errors just bubble up
- Replace all non-test `unwrap()`/`expect()` with proper error propagation or fallback values
- Existing `unwrap_or` patterns are fine and stay as-is

### Structured Logging

- Add `tracing` + `tracing-appender` + `tracing-subscriber`
- Log to file at `~/.config/trapped-mind/trapped-mind.log` (no stdout/stderr since TUI owns terminal)
- Replace silent `let _ =` error swallowing with `tracing::warn!` calls
- Log levels: `info` for startup/shutdown, `warn` for recoverable failures, `error` for generation failures

## Section 2: LLM Client Trait & Retry Logic

### Trait Definition

New `src/llm.rs` module:

```rust
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn stream_chat(
        &self,
        request: ChatRequest,
        token_tx: mpsc::UnboundedSender<AppEvent>,
    ) -> Result<(), AppError>;

    async fn pull_model(&self, model: &str) -> Result<(), AppError>;
}
```

### ChatRequest

Backend-agnostic struct wrapping model name, messages (using our own `Role` enum), and generation options (temperature, top_p). The `OllamaClient` impl converts to `ollama-rs` types internally so the trait boundary doesn't leak `ollama-rs` types.

### OllamaClient

Existing Ollama logic becomes `OllamaClient` implementing `LlmClient`. `spawn_generation` takes `Arc<dyn LlmClient>` instead of `&Ollama`.

### Retry with Backoff

Built into `OllamaClient::stream_chat`:
- 3 attempts max, exponential backoff (1s, 2s, 4s)
- Only retries on connection errors, not on model-not-found or content errors
- Model auto-pull stays as-is (pull then single retry)

### Timeout

`tokio::time::timeout(60s)` wrapping the stream. Configurable via `ollama_timeout_secs` in config.toml (default 60).

## Section 3: Graceful Shutdown & Config Validation

### Graceful Shutdown

- `tokio::signal::ctrl_c()` feeds `AppEvent::Shutdown` into the event channel
- SIGTERM handled via `tokio::signal::unix::signal(SignalKind::terminate())` on Linux
- On shutdown: log timestamp to history, save pending config, restore terminal, exit cleanly
- Replaces current Ctrl+C handling in `handle_key` which only works if key event loop is responsive

### Config Validation

`AppConfig::validate() -> Result<(), AppError>` called right after `load()`:
- `ollama_port` is non-zero
- `ollama_host` starts with `http://` or `https://`
- `max_history` > 0
- `auto_think_delay_secs` > 0
- `think_delay_min_ms` <= `think_delay_max_ms`
- `history_path` parent directory is writable or creatable

Fails fast with clear error message before entering TUI.

### Sensor Read Failures

Wrap each real sensor read in `SystemReader::read()` with a catch that:
- Logs a warning via `tracing::warn!`
- Falls back to last known value or simulator
- Handles sensors that were available at startup but fail mid-session

## Section 4: Project Structure & Build

### File Changes

```
src/
  main.rs          - slimmed down, setup + event loop
  app.rs           - unchanged structurally
  config.rs        - add validate(), add ollama_timeout_secs field
  error.rs         - NEW: AppError enum with thiserror
  llm.rs           - NEW: LlmClient trait + ChatRequest types
  ollama.rs        - becomes OllamaClient impl, prompt building stays
  history.rs       - Result propagation instead of silent failures
  system.rs        - add last-known-value fallback on sensor read failure
  pet_states.rs    - unchanged
  ui/              - unchanged
```

### New Dependencies

- `thiserror` - error derive
- `anyhow` - main() error handling
- `tracing` + `tracing-appender` + `tracing-subscriber` - file logging
- `async-trait` - for LlmClient trait

### Cargo.toml Cleanup

- Add `authors`, `description`, `license = "MIT"`, `repository`
- Add `[profile.release]` with `strip = true`, `lto = true`

### Commit Sequence

1. `refactor: add error types with thiserror and anyhow`
2. `refactor: add structured logging with tracing`
3. `refactor: extract LlmClient trait and ChatRequest types`
4. `refactor: implement OllamaClient with retry and timeout`
5. `refactor: add config validation and graceful shutdown`
6. `refactor: add last-known-value fallback for sensor failures`
7. `refactor: clean up Cargo.toml metadata and release profile`
8. `test: add unit tests for error handling, config validation, and prompt building`
9. `docs: update README with build instructions and architecture overview`
10. `refactor: production-ready cleanup` (final clippy/warning sweep)
