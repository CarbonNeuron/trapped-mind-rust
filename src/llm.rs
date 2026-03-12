//! Backend-agnostic LLM client trait and request types.
//!
//! Defines [`LlmClient`] for streaming chat completions and [`ChatRequest`]
//! as a backend-agnostic message format. Concrete implementations (e.g.
//! [`OllamaClient`](crate::ollama::OllamaClient)) convert these types to
//! their native formats internally.

use crate::error::AppError;
use async_trait::async_trait;
use tokio::sync::mpsc;

/// Generation options for LLM requests.
#[derive(Debug, Clone, Default)]
pub struct GenerationOptions {
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
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

/// A stream of tokens from an LLM generation.
/// Receives `Ok(token)` for each token, then the sender is dropped on completion.
/// Receives `Err(AppError)` if generation fails.
pub type LlmStream = mpsc::UnboundedReceiver<Result<String, AppError>>;

/// Trait for LLM backends that support streaming chat.
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Streams a chat completion, returning a token receiver.
    /// Each token arrives as Ok(String). Sender is dropped on completion.
    /// Errors arrive as Err(AppError).
    async fn stream_generate(&self, request: ChatRequest) -> Result<LlmStream, AppError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_stream_type_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<LlmStream>();
    }
}
