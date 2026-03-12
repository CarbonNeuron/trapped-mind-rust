//! Application-wide error types.

/// Errors that can occur in the TrappedMind application.
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum AppError {
    /// Configuration loading or validation failure.
    #[error("config error: {0}")]
    Config(String),

    /// Conversation history I/O failure.
    #[error("history error: {0}")]
    History(String),

    /// LLM client communication failure.
    #[error("llm error: {0}")]
    Llm(String),

    /// System sensor read failure.
    #[error("system error: {0}")]
    System(String),

    /// Generic I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let e = AppError::Config("bad port".to_string());
        assert_eq!(e.to_string(), "config error: bad port");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let app_err: AppError = io_err.into();
        assert!(app_err.to_string().contains("gone"));
    }

    #[test]
    fn test_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AppError>();
    }
}
