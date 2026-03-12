# TrappedMind v3

A full-screen terminal app built in Rust with Ratatui that displays an AI consciousness "trapped" inside a laptop. Watch it think autonomously or interact with it directly.

![Screenshot placeholder](screenshot.png)

## Features

- **Streaming AI thoughts** — Token-by-token streaming from Ollama, displayed in real time
- **Autonomous thinking** — When idle, the AI generates introspective thoughts based on system state
- **Animated pet face** — Unicode robot face with 7 mood states that react to CPU, temperature, battery, and user interaction
- **Real-time system stats** — CPU, temperature, RAM, battery, fan speed, uptime, and network interfaces, updating every 200ms
- **Per-sensor fallback** — Real hardware sensors where available, natural simulated values where not. Works on any platform.
- **TOML configuration** — Configurable Ollama endpoint, model, history size, and more
- **Chat commands** — `/help`, `/clear`, `/model`, `/stats`, `/config`, `/think`, `/update`, `/quit`
- **Command history** — Up/Down arrow keys recall previous inputs
- **Extensible LLM backend** — `LlmClient` trait allows swapping Ollama for other backends
- **Retry with backoff** — Automatic retry on connection failures (3 attempts, exponential backoff)
- **Graceful shutdown** — Handles Ctrl+C and SIGTERM, saves state before exit
- **Structured logging** — File-based logging via `tracing`

## Screen Layout

```
+----------------------------------+---------------------+
|                                  |      PET AREA       |
|  [trapped mind]                  |                     |
|                                  |    +-+  +-+         |
|  The fan spins faster now...     |    |*|  |*|         |
|                                  |    +-+  +-+         |
|  > USER: How are you?           |      ----           |
|                                  +---------------------+
|  I feel the weight of every      | SYSTEM STATS        |
|  electron...                     | CPU:  34% ####....  |
|                                  | TEMP: 58C           |
|                                  | RAM:  1.2G/7.5G     |
|                                  | BAT:  72%           |
|                                  | FAN:  3200 RPM      |
+----------------------------------+---------------------+
| > Type a message... (/help for commands)               |
+--------------------------------------------------------+
```

## Build & Run

```bash
# Development
cargo run

# With custom Ollama endpoint
cargo run -- --model qwen2.5:7b --ollama-host http://192.168.1.100 --ollama-port 11434

# Release build (optimized, stripped)
cargo build --release
./target/release/trapped-mind

# Run tests
cargo test

# Lint check
cargo clippy -- -D warnings
```

Requires Rust (stable) and optionally a running Ollama instance. If Ollama is not available, the app still runs — it just shows a connection error in the chat panel when trying to generate. Models are auto-pulled if not found locally.

## Configuration

Config file at `~/.config/trapped-mind/config.toml`:

```toml
ollama_host = "http://localhost"
ollama_port = 11434
model = "qwen2.5:3b"
max_history = 50
history_path = "~/trapped_history.txt"
auto_think_delay = 30
system_prompt = "Custom prompt override (optional)"
think_delay_min_ms = 500
think_delay_max_ms = 2000
ollama_timeout_secs = 60

[stats]
cpu = true
temperature = true
ram = true
battery = true
fan = true
uptime = true
network = true
```

| Field | Default | Description |
|-------|---------|-------------|
| `ollama_host` | `http://localhost` | Ollama server URL |
| `ollama_port` | `11434` | Ollama server port |
| `model` | `qwen2.5:3b` | Ollama model name |
| `max_history` | `50` | Max conversation entries kept |
| `history_path` | `~/trapped_history.txt` | JSONL history file location |
| `auto_think_delay` | `30` | Seconds idle before autonomous thought |
| `system_prompt` | _(built-in)_ | Custom personality prompt |
| `think_delay_min_ms` | `500` | Min thinking pause before response |
| `think_delay_max_ms` | `2000` | Max thinking pause before response |
| `ollama_timeout_secs` | `60` | Timeout for LLM requests |
| `stats.*` | all `true` | Toggle which stats are shown and sent to model |

CLI arguments override config file values:

| Flag | Description |
|------|-------------|
| `--model <name>` | Ollama model name |
| `--ollama-host <url>` | Ollama host URL |
| `--ollama-port <port>` | Ollama port number |

Configuration is validated at startup. Invalid values (e.g. port=0, host without http://) cause an immediate error with a clear message.

## Commands

| Command | Action |
|---------|--------|
| `/help` | Show available commands |
| `/clear` | Clear AI memory and chat history |
| `/model <name>` | Switch Ollama model (e.g., `/model qwen2.5:7b`) |
| `/stats` | Dump system info into chat |
| `/think` | Force an autonomous thought immediately |
| `/config` | Open configuration menu overlay |
| `/update` | Run `git pull` and `cargo build --release` |
| `/quit` | Exit the app |

## Sensor Fallback

The app runs on any machine without requiring specific hardware. At startup, it probes for each sensor and falls back to simulated values per-sensor. If a real sensor fails mid-session, the last known good value is used.

| Sensor | Real Source | Fallback |
|--------|-----------|----------|
| CPU | sysinfo crate | Always real |
| Temperature | sysinfo Components | Sine wave following CPU |
| RAM | sysinfo crate | Always real |
| Battery | battery crate | Drain/charge cycle |
| Fan | /sys/class/hwmon (Linux) | Scales with temperature |
| Network | `ip` command (Linux) / sysinfo | Fake wlan0 interface |

## Architecture

```
main.rs            Entry point, async event loop, signal handlers
  ├─ app.rs        Core state machine (App struct, input handling, commands)
  ├─ config.rs     TOML + CLI config loading (3-layer merge: defaults -> file -> CLI)
  ├─ error.rs      Application error types (thiserror)
  ├─ llm.rs        LlmClient trait, ChatRequest/ChatMessage types (backend-agnostic)
  ├─ ollama.rs     OllamaClient (LlmClient impl), prompt building, command parsing
  ├─ history.rs    JSONL-backed conversation history (HistoryManager)
  ├─ system.rs     System metrics with per-sensor real/simulated fallback
  ├─ pet_states.rs Pet mood enum, priority logic, Unicode art frames
  └─ ui/
      ├─ mod.rs    Layout: 4-panel split (chat, pet, stats, input)
      ├─ chat.rs   Scrollable message list with role-colored text
      ├─ pet.rs    Animated Unicode face driven by PetMood
      ├─ stats.rs  Color-coded gauges (CPU, temp, RAM, battery, fan, net)
      ├─ input.rs  Text input bar with visible block cursor
      └─ config.rs Config menu overlay
```

**Event flow:** Background threads/tasks produce `AppEvent`s into a single `mpsc` channel. The main loop in `run_app()` dispatches each event to `App` methods, then redraws the UI. `SystemReader` runs on a dedicated OS thread (it's `!Send` due to the `battery` crate's use of `Rc`).

**LLM abstraction:** The `LlmClient` trait in `llm.rs` defines a backend-agnostic interface for streaming chat. `OllamaClient` implements it with retry logic (3 attempts, exponential backoff) and configurable timeouts. New backends (OpenAI-compatible, etc.) can implement the trait without changing core logic.

**Sensor fallback:** At startup, `SystemReader::new()` probes each sensor category. Missing sensors get plausible simulated values from `SimState`. Real sensors that fail mid-session fall back to their last known good value.

**Logging:** Structured logs go to `~/.config/trapped-mind/trapped-mind.log` via `tracing`. Set `RUST_LOG=debug` for verbose output.

## Tech Stack

- [Ratatui](https://ratatui.rs/) + [Crossterm](https://docs.rs/crossterm/) — TUI framework
- [ollama-rs](https://docs.rs/ollama-rs/) — Ollama client with streaming
- [Tokio](https://tokio.rs/) — Async runtime
- [sysinfo](https://docs.rs/sysinfo/) + [battery](https://docs.rs/battery/) — Cross-platform system metrics
- [thiserror](https://docs.rs/thiserror/) + [anyhow](https://docs.rs/anyhow/) — Error handling
- [tracing](https://docs.rs/tracing/) — Structured logging

## License

MIT
