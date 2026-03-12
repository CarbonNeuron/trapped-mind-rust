//! Backend-agnostic LLM client trait and request types.
//!
//! Defines [`LlmClient`] for streaming chat completions and [`ChatRequest`]
//! as a backend-agnostic message format. Concrete implementations (e.g.
//! [`OllamaClient`](crate::ollama::OllamaClient)) convert these types to
//! their native formats internally.

use crate::app::AppEvent;
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

/// Trait for LLM backends that support streaming chat.
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Streams a chat completion, sending tokens as [`AppEvent::Token`] and
    /// completion as [`AppEvent::GenerationDone`] through the channel.
    async fn stream_chat(
        &self,
        request: ChatRequest,
        tx: mpsc::UnboundedSender<AppEvent>,
    ) -> Result<(), AppError>;

    /// Pulls/downloads a model by name.
    async fn pull_model(&self, model: &str) -> Result<(), AppError>;
}
