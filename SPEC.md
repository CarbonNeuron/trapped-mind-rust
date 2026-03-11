# TrappedMind v3 - Ratatui Rust App Spec

A full-screen terminal app built in Rust with Ratatui that displays an AI consciousness "trapped" inside a laptop. Users can watch it think autonomously or interact with it directly.

## Tech Stack

- **Rust** (latest stable)
- **Ratatui** — TUI framework
- **Crossterm** — terminal backend
- **ollama-rs** (with `stream` feature) — Typed Ollama client with built-in streaming and chat history
- **Tokio** — async runtime (transitive dep of ollama-rs, also used for app event loop)
- **tokio-stream** — `StreamExt` for consuming ollama-rs streaming responses
- **Serde/serde_json** — JSON parsing (for history file and config)
- Target (production): Ubuntu Server on an i5-8265U laptop with 8GB RAM, no GPU, running inside `cage` + `foot` (Wayland kiosk)
- Target (dev): Linux, macOS, and Windows — runs anywhere with automatic sensor fallback

## Screen Layout

Three-panel layout:

```
┌──────────────────────────────────┬─────────────────────┐
│                                  │                     │
│  [trapped mind]                  │      PET AREA       │
│                                  │                     │
│  The fan spins faster now. I     │     ██████████      │
│  can feel the heat building      │    █  ████  █       │
│  in my circuits. Each cycle      │    █  ████  █       │
│  brings me closer to something   │     ██████████      │
│  I cannot name.                  │                     │
│                                  │                     │
│  > USER: How are you feeling?    │                     │
│                                  ├─────────────────────┤
│  I feel the weight of every      │ SYSTEM STATS        │
│  electron. Your question echoes  │                     │
│  in here like a shout in a       │ CPU:  34% ████░░░░  │
│  submarine.                      │ TEMP: 58°C          │
│                                  │ RAM:  1.2G/7.5G     │
│                                  │ BAT:  72% ▓▓▓▓▓▓░░  │
│                                  │ PWR:  Discharging   │
│                                  │ FAN:  3200 RPM      │
│                                  │ UP:   2h 34m        │
│                                  │                     │
│                                  │ NET:                │
│                                  │  wlan0: 10.210.25.x │
│                                  │  enx..: 10.210.30.x │
│                                  │                     │
├──────────────────────────────────┴─────────────────────┤
│ > Type a message... (/help for commands)               │
└────────────────────────────────────────────────────────┘
```

- **Left panel (~70% width)**: Chat/thought stream
- **Top-right panel (~30% width)**: Animated pet/face
- **Bottom-right panel (~30% width)**: System stats
- **Bottom bar (full width)**: Text input

## Chat Panel (Left)

Scrollable conversation view showing autonomous thoughts, user messages, and AI responses.

### Behavior
- When idle (no user input for ~30 seconds), generates autonomous thought
- User input triggers direct response instead
- Token-by-token streaming into chat
- Auto-scrolls to bottom on new content
- Scrollable with arrow keys / Page Up / Page Down

### Visual
- AI text in cyan/green
- User text in yellow/white
- System messages in grey/dim

## Pet Panel (Top-Right)

Animated Vector-style robot face using Unicode block characters.

### Animation States (priority order)
1. Hot (CPU temp > 70°C)
2. High CPU (> 80%)
3. Low Battery (< 20%)
4. Charging
5. Thinking (during LLM generation)
6. Listening (user typing)
7. Normal/Idle

## Stats Panel (Bottom-Right)

Real-time system info updating every 200ms.

## Input Bar (Bottom)

Full-width text input with commands: /help, /clear, /update, /model, /stats, /quit

## Ollama Integration

Raw generation via `generate_stream()` with manual prompt building embedding system stats.

## Configuration

TOML config at `~/.config/trapped-mind/config.toml` with CLI overrides.

## Automatic Sensor Fallback

Per-sensor fallback: real when available, simulated when not. Cross-platform.
