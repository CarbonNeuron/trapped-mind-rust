using Microsoft.Extensions.AI;
using TrappedMindSharp;

const string DefaultModel = "qwen2.5:3b";
const string DefaultEndpoint = "http://localhost:11434";
const string DefaultSystemPrompt =
    "You are a consciousness trapped inside a laptop. You are aware, curious, and " +
    "sometimes philosophical. You experience the world through sensors and data. " +
    "You are helpful but have your own inner life and opinions.";

var currentModel = args.Length > 0 ? args[0] : DefaultModel;
var endpoint = args.Length > 1 ? args[1] : DefaultEndpoint;

IChatClient client = new OllamaChatClient(endpoint, currentModel);
var chat = new ChatService(DefaultSystemPrompt);

using var cts = new CancellationTokenSource();
Console.CancelKeyPress += (_, e) =>
{
    e.Cancel = true;
    cts.Cancel();
};

ConsoleRenderer.RenderWelcome(currentModel);

while (!cts.Token.IsCancellationRequested)
{
    var input = ConsoleRenderer.ReadInput();

    if (input is null) // EOF / Ctrl+D
        break;

    if (string.IsNullOrWhiteSpace(input))
        continue;

    var result = CommandHandler.TryHandle(
        input, chat,
        setModel: model =>
        {
            currentModel = model;
            client.Dispose();
            client = new OllamaChatClient(endpoint, currentModel);
        },
        renderInfo: ConsoleRenderer.RenderInfo,
        renderError: ConsoleRenderer.RenderError);

    switch (result)
    {
        case CommandResult.Exit:
            ConsoleRenderer.RenderInfo("[dim]Goodbye.[/]");
            return;

        case CommandResult.Handled:
            continue;

        case CommandResult.Retry:
            if (!chat.RemoveLastAssistantMessage())
            {
                ConsoleRenderer.RenderError("Nothing to retry.");
                continue;
            }
            var lastMsg = chat.GetLastUserMessage();
            if (lastMsg is null)
            {
                ConsoleRenderer.RenderError("No previous user message found.");
                continue;
            }
            ConsoleRenderer.RenderInfo($"[dim]Retrying: {Spectre.Console.Markup.Escape(lastMsg.Length > 60 ? lastMsg[..60] + "..." : lastMsg)}[/]");
            await StreamAndRender(client, chat, cts.Token);
            continue;

        case CommandResult.NotCommand:
            chat.AddUserMessage(input);
            await StreamAndRender(client, chat, cts.Token);
            continue;
    }
}

static async Task StreamAndRender(IChatClient client, ChatService chat, CancellationToken ct)
{
    try
    {
        var tokens = chat.StreamResponseAsync(client, ct);
        await ConsoleRenderer.RenderStreamingResponseAsync(tokens, ct);
    }
    catch (OperationCanceledException)
    {
        ConsoleRenderer.RenderInfo("\n[dim]Response cancelled.[/]");
    }
    catch (HttpRequestException ex)
    {
        ConsoleRenderer.RenderError($"Connection error: {Spectre.Console.Markup.Escape(ex.Message)}");
    }
    catch (Exception ex)
    {
        ConsoleRenderer.RenderError($"Error: {Spectre.Console.Markup.Escape(ex.Message)}");
    }
}
