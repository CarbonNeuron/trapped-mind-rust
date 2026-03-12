using Microsoft.Extensions.AI;

namespace TrappedMindSharp;

public enum CommandResult
{
    NotCommand,
    Handled,
    Exit,
    Retry
}

public static class CommandHandler
{
    public static CommandResult TryHandle(
        string input,
        ChatService chat,
        Action<string> setModel,
        Action<string> renderInfo,
        Action<string> renderError)
    {
        if (!input.StartsWith('/'))
            return CommandResult.NotCommand;

        var parts = input.Split(' ', 2, StringSplitOptions.TrimEntries);
        var command = parts[0].ToLowerInvariant();
        var arg = parts.Length > 1 ? parts[1] : null;

        switch (command)
        {
            case "/exit" or "/quit":
                return CommandResult.Exit;

            case "/clear":
                chat.Clear();
                renderInfo("Conversation cleared.");
                return CommandResult.Handled;

            case "/system":
                if (string.IsNullOrWhiteSpace(arg))
                {
                    renderError("Usage: /system <prompt>");
                    return CommandResult.Handled;
                }
                chat.SetSystemPrompt(arg);
                renderInfo($"System prompt updated.");
                return CommandResult.Handled;

            case "/model":
                if (string.IsNullOrWhiteSpace(arg))
                {
                    renderError("Usage: /model <name>");
                    return CommandResult.Handled;
                }
                setModel(arg);
                renderInfo($"Model changed to [bold]{Spectre.Console.Markup.Escape(arg)}[/].");
                return CommandResult.Handled;

            case "/history":
                RenderHistory(chat, renderInfo);
                return CommandResult.Handled;

            case "/help":
                RenderHelp(renderInfo);
                return CommandResult.Handled;

            case "/retry":
                return CommandResult.Retry;

            default:
                renderError($"Unknown command: {Spectre.Console.Markup.Escape(command)}. Type /help for available commands.");
                return CommandResult.Handled;
        }
    }

    private static void RenderHistory(ChatService chat, Action<string> renderInfo)
    {
        if (chat.History.Count <= 1)
        {
            renderInfo("No conversation history yet.");
            return;
        }

        foreach (var msg in chat.History)
        {
            var role = msg.Role.Value;
            var color = role switch
            {
                "system" => "yellow",
                "user" => "cyan",
                "assistant" => "green",
                _ => "white"
            };
            var text = Spectre.Console.Markup.Escape(msg.Text ?? "");
            renderInfo($"[bold {color}]{role}:[/] {text}");
        }
    }

    private static void RenderHelp(Action<string> renderInfo)
    {
        renderInfo("""
            [bold]Available commands:[/]
              [cyan]/clear[/]            Reset conversation history
              [cyan]/system <prompt>[/]  Set the system prompt
              [cyan]/model <name>[/]     Switch the Ollama model
              [cyan]/history[/]          Show conversation history
              [cyan]/retry[/]            Regenerate the last response
              [cyan]/help[/]             Show this help message
              [cyan]/exit[/]             Exit the application
            """);
    }
}
