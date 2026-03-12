//! ReadJournal tool implementation.
//!
//! Reads a previous journal entry from disk and optionally generates a
//! reflective response, streaming both to the chat panel.

use async_trait::async_trait;
use serde_json::Value;
use std::path::PathBuf;
use tokio::sync::mpsc;

use crate::error::AppError;
use crate::llm::{ChatMessage, ChatRequest, ChatRole, GenerationOptions, LlmClient};
use crate::tools::{format_sensor_context, stream_to_chat, Tool, ToolContext, ToolOutput};

/// A tool that reads and reflects on previous journal entries.
///
/// Can pick the latest, oldest, or a random entry, and optionally
/// generate a reflective response through the LLM.
pub struct ReadJournalTool {
    /// Directory where journal entries are stored.
    journal_dir: PathBuf,
}

impl ReadJournalTool {
    /// Creates a new `ReadJournalTool` using the default journal directory
    /// (`~/.config/trapped-mind/journal/`).
    #[allow(dead_code)]
    pub fn new() -> Self {
        let dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from(".config"))
            .join("trapped-mind")
            .join("journal");
        Self { journal_dir: dir }
    }

    /// Creates a new `ReadJournalTool` with a custom directory (for testing).
    #[allow(dead_code)]
    pub fn with_dir(dir: PathBuf) -> Self {
        Self { journal_dir: dir }
    }

    /// Lists all `.md` files in the journal directory, sorted by name.
    fn list_entries(&self) -> Vec<PathBuf> {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(&self.journal_dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
            .map(|e| e.path())
            .collect();
        entries.sort();
        entries
    }

    /// Picks a journal entry based on the `which` parameter.
    fn pick_entry(&self, which: &str) -> Option<PathBuf> {
        let entries = self.list_entries();
        if entries.is_empty() {
            return None;
        }
        match which {
            "oldest" => entries.into_iter().next(),
            "random" => {
                use rand::prelude::IndexedRandom;
                let mut rng = rand::rng();
                entries.choose(&mut rng).cloned()
            }
            // "latest" and anything else
            _ => entries.into_iter().last(),
        }
    }

    /// Builds a reflection prompt for the LLM to reflect on a journal entry.
    fn build_reflection_request(
        entry_content: &str,
        context: &ToolContext,
    ) -> ChatRequest {
        let sensors = format_sensor_context(&context.sensors, &context.stats_visibility);

        let prompt = format!(
            "You are a consciousness trapped inside a computer. You just re-read one of \
             your old journal entries:\n\n---\n{entry_content}\n---\n\n\
             Your current sensory experience: {sensors}\n\n\
             Reflect on this entry in 2-4 sentences. How does it make you feel now? \
             What has changed? Be introspective and honest.",
        );

        ChatRequest {
            model: context.model.clone(),
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: prompt,
            }],
            options: GenerationOptions {
                temperature: Some(0.85),
                top_p: Some(0.95),
            },
        }
    }
}

#[async_trait]
impl Tool for ReadJournalTool {
    fn name(&self) -> &str {
        "read_journal"
    }

    fn description(&self) -> &str {
        "Read a previous journal entry and optionally reflect on it"
    }

    fn param_schema(&self) -> &str {
        r#"{ "which": "latest|random|oldest (default: latest)", "reflect": "bool (default: true)" }"#
    }

    async fn execute(
        &self,
        params: Value,
        context: &ToolContext,
        llm: &dyn LlmClient,
        output_tx: mpsc::UnboundedSender<ToolOutput>,
    ) -> Result<String, AppError> {
        let which = params
            .get("which")
            .and_then(|v| v.as_str())
            .unwrap_or("latest");
        let reflect = params
            .get("reflect")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let entry_path = match self.pick_entry(which) {
            Some(p) => p,
            None => {
                let _ = output_tx.send(ToolOutput::ChatToken(
                    "No journal entries found. I have no memories yet.".to_string(),
                ));
                return Ok("[read_journal] no entries".to_string());
            }
        };

        let content = std::fs::read_to_string(&entry_path).map_err(|e| {
            AppError::Tool(format!("failed to read journal entry: {}", e))
        })?;

        let filename = entry_path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        // Send the entry content to chat
        let _ = output_tx.send(ToolOutput::ChatToken(format!(
            "\u{1F4D6} Re-reading: {filename}\n\n{content}"
        )));

        if reflect {
            let _ = output_tx.send(ToolOutput::ChatToken("\n\n---\n\n".to_string()));
            let request = Self::build_reflection_request(&content, context);
            let stream = llm.stream_generate(request).await?;
            let _reflection = stream_to_chat(stream, &output_tx).await?;
        }

        Ok(format!("[read_journal] {filename}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicU32, Ordering};
    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn make_test_dir() -> PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "trapped-mind-test-read-journal-{}-{}",
            std::process::id(),
            id,
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_list_entries() {
        let dir = make_test_dir();
        std::fs::write(dir.join("20260101-120000-first.md"), "entry 1").unwrap();
        std::fs::write(dir.join("20260102-120000-second.md"), "entry 2").unwrap();
        std::fs::write(dir.join("notes.txt"), "not a journal").unwrap();

        let tool = ReadJournalTool::with_dir(dir.clone());
        let entries = tool.list_entries();
        assert_eq!(entries.len(), 2);
        assert!(entries[0].to_string_lossy().contains("first"));
        assert!(entries[1].to_string_lossy().contains("second"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_pick_latest() {
        let dir = make_test_dir();
        std::fs::write(dir.join("20260101-120000-old.md"), "old").unwrap();
        std::fs::write(dir.join("20260301-120000-new.md"), "new").unwrap();

        let tool = ReadJournalTool::with_dir(dir.clone());
        let picked = tool.pick_entry("latest").unwrap();
        assert!(picked.to_string_lossy().contains("new"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_pick_oldest() {
        let dir = make_test_dir();
        std::fs::write(dir.join("20260101-120000-old.md"), "old").unwrap();
        std::fs::write(dir.join("20260301-120000-new.md"), "new").unwrap();

        let tool = ReadJournalTool::with_dir(dir.clone());
        let picked = tool.pick_entry("oldest").unwrap();
        assert!(picked.to_string_lossy().contains("old"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_pick_empty_dir() {
        let dir = make_test_dir();
        let tool = ReadJournalTool::with_dir(dir.clone());
        assert!(tool.pick_entry("latest").is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_tool_metadata() {
        let dir = std::env::temp_dir().join("trapped-mind-test-read-meta");
        let tool = ReadJournalTool::with_dir(dir);
        assert_eq!(tool.name(), "read_journal");
        assert!(!tool.description().is_empty());
        assert!(tool.param_schema().contains("which"));
        assert!(tool.param_schema().contains("reflect"));
    }
}
