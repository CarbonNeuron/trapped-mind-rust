# Ollama Chat CLI Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a .NET 10 console app that provides an interactive streamed chat with Ollama via IChatClient and Spectre.Console.

**Architecture:** Single console project with 4 files: Program.cs (entry + main loop), ChatService.cs (history + streaming), CommandHandler.cs (slash commands), ConsoleRenderer.cs (Spectre output).

**Tech Stack:** .NET 10, Microsoft.Extensions.AI.Ollama, Spectre.Console

---

## Chunk 1: Core Implementation

### Task 1: ChatService

**Files:**
- Create: `ChatService.cs`

- [ ] **Step 1: Create ChatService.cs**
  - Manages `List<ChatMessage>` history
  - `SetSystemPrompt(string)`, `Clear()`, `RemoveLastExchange()`, `GetHistory()`
  - `StreamResponseAsync(IChatClient, CancellationToken)` — sends history to client, yields `ChatResponseUpdate` tokens, appends full response to history

- [ ] **Step 2: Build and verify it compiles**

### Task 2: CommandHandler

**Files:**
- Create: `CommandHandler.cs`

- [ ] **Step 1: Create CommandHandler.cs**
  - `TryHandle(string input, ChatService, Action<string> setModel)` returns `CommandResult` (Handled, Exit, NotCommand, Retry)
  - Implements: `/clear`, `/system`, `/model`, `/history`, `/help`, `/exit`, `/quit`, `/retry`

- [ ] **Step 2: Build and verify**

### Task 3: ConsoleRenderer

**Files:**
- Create: `ConsoleRenderer.cs`

- [ ] **Step 1: Create ConsoleRenderer.cs**
  - `RenderUserPrompt()` — returns markup prompt string
  - `RenderStreamingResponse(IAsyncEnumerable<ChatResponseUpdate>)` — streams tokens with green assistant label
  - `RenderError(string)`, `RenderInfo(string)`, `RenderHistory(List<ChatMessage>)`, `RenderHelp()`, `RenderWelcome()`

- [ ] **Step 2: Build and verify**

### Task 4: Program.cs — Main Loop

**Files:**
- Modify: `Program.cs`

- [ ] **Step 1: Wire everything together**
  - Create `OllamaChatClient`, `ChatService`, `CommandHandler`
  - Main loop: read input → check command → send to chat → render stream
  - Handle Ctrl+C gracefully

- [ ] **Step 2: Build, run, and test end-to-end**

- [ ] **Step 3: Commit all files**
