//! Multi-level digest generation for efficient context retrieval

use serde::{Deserialize, Serialize};

/// Multi-level digest for a node
///
/// Provides three levels of summarization:
/// - Brief: ~50 tokens - Quick relevance check
/// - Summary: ~500 tokens - Planning and understanding
/// - Full: Original content - Deep reading
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Digest {
    /// Brief summary (~50 tokens)
    pub brief: String,

    /// Medium summary (~500 tokens)
    pub summary: String,

    /// Whether digests have been generated
    pub generated: bool,
}

impl Digest {
    /// Create a new empty digest
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a digest with pre-generated content
    pub fn with_content(brief: String, summary: String) -> Self {
        Self {
            brief,
            summary,
            generated: true,
        }
    }

    /// Check if this digest has been generated
    pub fn is_generated(&self) -> bool {
        self.generated
    }

    /// Get the appropriate level based on token budget
    pub fn get_level(&self, max_tokens: usize) -> DigestLevel {
        if max_tokens < 100 {
            DigestLevel::Brief
        } else if max_tokens < 1000 {
            DigestLevel::Summary
        } else {
            DigestLevel::Full
        }
    }
}

/// Level of digest detail
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigestLevel {
    /// Brief summary only
    Brief,
    /// Medium summary
    Summary,
    /// Full content
    Full,
}

/// Generator for creating digests from content
pub struct DigestGenerator {
    llm_client: Option<LLMClient>,
}

impl DigestGenerator {
    /// Create a new digest generator
    pub fn new(llm_client: Option<LLMClient>) -> Self {
        Self { llm_client }
    }

    /// Generate a digest for the given content
    pub async fn generate(&self, content: &str, kind: crate::core::NodeKind) -> crate::Result<Digest> {
        // If no LLM client, use simple extraction
        if self.llm_client.is_none() {
            return Ok(self.generate_simple(content));
        }

        let llm = self.llm_client.as_ref().unwrap();

        // Generate brief summary
        let brief_prompt = format!(
            "Summarize the following {} in one concise sentence (max 50 tokens):\n\n{}",
            kind_to_str(kind),
            truncate(content, 4000)
        );

        let brief = llm.complete(&brief_prompt).await?;

        // Generate medium summary
        let summary_prompt = format!(
            "Provide a comprehensive summary of the following {} (max 500 tokens). \
             Include key points, main concepts, and important details:\n\n{}",
            kind_to_str(kind),
            truncate(content, 8000)
        );

        let summary = llm.complete(&summary_prompt).await?;

        Ok(Digest::with_content(brief, summary))
    }

    /// Generate a simple digest without LLM
    fn generate_simple(&self, content: &str) -> Digest {
        let brief = extract_first_sentence(content);
        let summary = truncate(content, 2000).to_string();

        Digest::with_content(brief, summary)
    }
}

/// Simple LLM client interface
pub struct LLMClient {
    api_base: String,
    api_key: String,
    model: String,
}

impl LLMClient {
    pub fn new(api_base: String, api_key: String, model: String) -> Self {
        Self {
            api_base,
            api_key,
            model,
        }
    }

    pub async fn complete(&self, prompt: &str) -> crate::Result<String> {
        let client = reqwest::Client::new();

        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.0,
            "max_tokens": 1000,
        });

        let response = client
            .post(&format!("{}/chat/completions", self.api_base))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(crate::A3SError::DigestGeneration(format!(
                "LLM API error: {}",
                response.status()
            )));
        }

        let result: serde_json::Value = response.json().await?;

        let content = result["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| crate::A3SError::DigestGeneration("Invalid response format".to_string()))?;

        Ok(content.to_string())
    }
}

fn kind_to_str(kind: crate::core::NodeKind) -> &'static str {
    match kind {
        crate::core::NodeKind::Document => "document",
        crate::core::NodeKind::Code => "code",
        crate::core::NodeKind::Markdown => "markdown document",
        crate::core::NodeKind::Memory => "memory",
        crate::core::NodeKind::Capability => "capability",
        crate::core::NodeKind::Message => "message",
        crate::core::NodeKind::Data => "data",
        crate::core::NodeKind::Directory => "directory",
    }
}

fn truncate(s: &str, max_chars: usize) -> &str {
    if s.len() <= max_chars {
        s
    } else {
        &s[..max_chars]
    }
}

fn extract_first_sentence(s: &str) -> String {
    let s = s.trim();
    if s.is_empty() {
        return String::new();
    }

    // Find first sentence ending
    let endings = [". ", ".\n", "! ", "!\n", "? ", "?\n"];
    let mut min_pos = s.len();

    for ending in &endings {
        if let Some(pos) = s.find(ending) {
            min_pos = min_pos.min(pos + 1);
        }
    }

    // Limit to 200 chars
    let end = min_pos.min(200);
    s[..end].trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_first_sentence() {
        let text = "This is the first sentence. This is the second.";
        assert_eq!(extract_first_sentence(text), "This is the first sentence.");
    }

    #[test]
    fn test_extract_first_sentence_with_exclamation() {
        let text = "Hello world! This is exciting.";
        assert_eq!(extract_first_sentence(text), "Hello world!");
    }

    #[test]
    fn test_extract_first_sentence_with_question() {
        let text = "What is this? Let me explain.";
        assert_eq!(extract_first_sentence(text), "What is this?");
    }

    #[test]
    fn test_extract_first_sentence_empty() {
        assert_eq!(extract_first_sentence(""), "");
        assert_eq!(extract_first_sentence("   "), "");
    }

    #[test]
    fn test_extract_first_sentence_no_ending() {
        let text = "This has no sentence ending";
        assert_eq!(extract_first_sentence(text), "This has no sentence ending");
    }

    #[test]
    fn test_truncate() {
        let text = "Hello, world!";
        assert_eq!(truncate(text, 5), "Hello");
        assert_eq!(truncate(text, 100), "Hello, world!");
    }

    #[test]
    fn test_truncate_exact() {
        let text = "Hello";
        assert_eq!(truncate(text, 5), "Hello");
    }

    #[test]
    fn test_digest_new() {
        let digest = Digest::new();
        assert!(!digest.is_generated());
        assert!(digest.brief.is_empty());
        assert!(digest.summary.is_empty());
    }

    #[test]
    fn test_digest_with_content() {
        let digest = Digest::with_content(
            "Brief summary".to_string(),
            "Longer summary content".to_string(),
        );
        assert!(digest.is_generated());
        assert_eq!(digest.brief, "Brief summary");
        assert_eq!(digest.summary, "Longer summary content");
    }

    #[test]
    fn test_digest_get_level() {
        let digest = Digest::new();
        assert_eq!(digest.get_level(50), DigestLevel::Brief);
        assert_eq!(digest.get_level(500), DigestLevel::Summary);
        assert_eq!(digest.get_level(2000), DigestLevel::Full);
    }

    #[test]
    fn test_kind_to_str() {
        assert_eq!(kind_to_str(crate::core::NodeKind::Document), "document");
        assert_eq!(kind_to_str(crate::core::NodeKind::Code), "code");
        assert_eq!(kind_to_str(crate::core::NodeKind::Markdown), "markdown document");
        assert_eq!(kind_to_str(crate::core::NodeKind::Memory), "memory");
        assert_eq!(kind_to_str(crate::core::NodeKind::Capability), "capability");
        assert_eq!(kind_to_str(crate::core::NodeKind::Directory), "directory");
    }
}
