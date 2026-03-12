using Spectre.Console;

namespace TrappedMindSharp;

public static class ConsoleRenderer
{
    public static void RenderWelcome(string model)
    {
        AnsiConsole.Write(new Rule("[bold blue]Trapped Mind Sharp[/]").RuleStyle("dim"));
        AnsiConsole.MarkupLine($"[dim]Connected to Ollama model:[/] [bold]{Markup.Escape(model)}[/]");
        AnsiConsole.MarkupLine("[dim]Type /help for commands, /exit to quit.[/]");
        AnsiConsole.WriteLine();
    }

    public static string? ReadInput()
    {
        AnsiConsole.Markup("[bold cyan]you>[/] ");
        return Console.ReadLine();
    }

    public static async Task RenderStreamingResponseAsync(IAsyncEnumerable<string> tokens, CancellationToken ct = default)
    {
        AnsiConsole.Markup("[bold green]ai>[/] ");

        await foreach (var token in tokens.WithCancellation(ct))
        {
            // Write raw text (no markup interpretation) to avoid issues with special chars
            AnsiConsole.Write(token);
        }

        AnsiConsole.WriteLine();
        AnsiConsole.Write(new Rule().RuleStyle("dim"));
    }

    public static void RenderInfo(string markup)
    {
        AnsiConsole.MarkupLine(markup);
    }

    public static void RenderError(string markup)
    {
        AnsiConsole.MarkupLine($"[red]{markup}[/]");
    }

    public static void RenderSeparator()
    {
        AnsiConsole.Write(new Rule().RuleStyle("dim"));
    }
}
