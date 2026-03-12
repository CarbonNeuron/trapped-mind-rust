using Microsoft.Extensions.AI;

namespace TrappedMindSharp;

public class ChatService
{
    private readonly List<ChatMessage> _history = [];

    public ChatService(string systemPrompt)
    {
        _history.Add(new ChatMessage(ChatRole.System, systemPrompt));
    }

    public IReadOnlyList<ChatMessage> History => _history;

    public void SetSystemPrompt(string prompt)
    {
        if (_history.Count > 0 && _history[0].Role == ChatRole.System)
            _history[0] = new ChatMessage(ChatRole.System, prompt);
        else
            _history.Insert(0, new ChatMessage(ChatRole.System, prompt));
    }

    public void Clear()
    {
        var system = _history.Count > 0 && _history[0].Role == ChatRole.System
            ? _history[0]
            : null;
        _history.Clear();
        if (system is not null)
            _history.Add(system);
    }

    public void AddUserMessage(string content)
    {
        _history.Add(new ChatMessage(ChatRole.User, content));
    }

    public bool RemoveLastAssistantMessage()
    {
        for (int i = _history.Count - 1; i >= 0; i--)
        {
            if (_history[i].Role == ChatRole.Assistant)
            {
                _history.RemoveAt(i);
                return true;
            }
        }
        return false;
    }

    public string? GetLastUserMessage()
    {
        for (int i = _history.Count - 1; i >= 0; i--)
        {
            if (_history[i].Role == ChatRole.User)
                return _history[i].Text;
        }
        return null;
    }

    public async IAsyncEnumerable<string> StreamResponseAsync(
        IChatClient client,
        [System.Runtime.CompilerServices.EnumeratorCancellation] CancellationToken ct = default)
    {
        var fullResponse = new System.Text.StringBuilder();

        await foreach (var update in client.GetStreamingResponseAsync(_history, cancellationToken: ct))
        {
            var text = update.Text;
            if (text is not null)
            {
                fullResponse.Append(text);
                yield return text;
            }
        }

        _history.Add(new ChatMessage(ChatRole.Assistant, fullResponse.ToString()));
    }
}
