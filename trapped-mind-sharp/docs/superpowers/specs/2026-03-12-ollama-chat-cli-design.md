# Ollama Chat CLI — Design Spec

## Overview

A .NET 10 console app that provides an interactive chat interface to an Ollama LLM. Uses the generic `IChatClient` abstraction from `Microsoft.Extensions.AI` with the Ollama provider. Responses stream to the terminal with Spectre.Console formatting. Conversation history is maintained for context. Slash commands provide runtime control.

## Components

### Program.cs
Entry point. Configures the `IChatClient` (Ollama, default model `qwen2.5:3b`, default endpoint `http://localhost:11434`). Runs the main input loop: read user input, check for commands, otherwise send to ChatService, render streamed response.

### ChatService.cs
Manages conversation state:
- `List<ChatMessage>` for history (system + user + assistant messages)
- `SendAsync(string userMessage)` — appends user message, calls `IChatClient.GetStreamingResponseAsync()`, collects and appends assistant response, yields tokens as they arrive
- `SetSystemPrompt(string prompt)` — replaces system message at index 0
- `Clear()` — resets history (preserves system prompt)
- `RemoveLastAssistantMessage()` — for `/retry` support
- `GetHistory()` — returns conversation history for `/history` display

### CommandHandler.cs
Parses input starting with `/`. Supported commands:
- `/clear` — call `ChatService.Clear()`
- `/system <prompt>` — call `ChatService.SetSystemPrompt()`
- `/model <name>` — swap the underlying model
- `/history` — display conversation history
- `/help` — list commands
- `/exit` or `/quit` — exit the app
- `/retry` — remove last assistant message, re-send last user message

Returns a result indicating whether the command was handled or unrecognized.

### ConsoleRenderer.cs
Spectre.Console rendering:
- User messages: bold cyan label
- Assistant responses: streamed with a green label, tokens written incrementally via `AnsiConsole.Write`/`Markup`
- System messages: dim yellow
- Error messages: red
- Separators between turns using `Rule`

## Data Flow

```
User input
  → starts with "/" → CommandHandler → execute, display result
  → otherwise → ChatService.SendAsync()
    → IChatClient.GetStreamingResponseAsync()
    → ConsoleRenderer streams tokens to terminal
    → full response appended to history
```

## Packages

- `Microsoft.Extensions.AI.Ollama` — IChatClient implementation for Ollama
- `Spectre.Console` — terminal rendering

## Error Handling

- Ollama connection failures: catch, display red error message, continue loop
- Unknown commands: display help text
- Empty input: skip silently

## Defaults

- Model: `qwen2.5:3b`
- Endpoint: `http://localhost:11434`
- System prompt: thematic "trapped mind" prompt
