//! Tokenizer Utilities
//!
//! Multi-model token counting, truncation, and cost estimation.
//! Uses `tiktoken-rs` for OpenAI-compatible models and provides
//! a character-based fallback for unsupported providers.
//!
//! Supported encodings:
//!   - cl100k_base   → GPT-4, GPT-3.5-turbo, text-embedding-ada-002
//!   - p50k_base     → GPT-3 (davinci, curie, babbage, ada)
//!   - r50k_base     → GPT-3 (older)
//!   - o200k_base    → GPT-4o, GPT-4o-mini
//!   - char/4        → Fallback for DeepSeek, Anthropic, Gemini, etc.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tiktoken_rs::{cl100k_base, o200k_base, p50k_base, r50k_base, CoreBPE};

// ── Encoding Selection ────────────────────────────────────────────────

/// Known tokenizer encoding types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TokenizerEncoding {
    /// GPT-4, GPT-3.5-turbo, text-embedding-ada-002
    Cl100kBase,
    /// GPT-4o, GPT-4o-mini
    O200kBase,
    /// GPT-3 davinci, curie, babbage, ada
    P50kBase,
    /// GPT-3 (older models)
    R50kBase,
    /// Character-based fallback (~4 chars per token)
    CharFallback,
}

impl TokenizerEncoding {
    /// Select the encoding for a given model name
    pub fn for_model(model: &str) -> Self {
        let lower = model.to_lowercase();

        if lower.contains("gpt-4o") || lower.contains("o200k") {
            TokenizerEncoding::O200kBase
        } else if lower.contains("gpt-4") || lower.contains("gpt-3.5") {
            TokenizerEncoding::Cl100kBase
        } else if lower.contains("text-davinci") || lower.contains("davinci") {
            TokenizerEncoding::P50kBase
        } else if lower.contains("curie") || lower.contains("babbage") || lower.contains("ada") {
            TokenizerEncoding::R50kBase
        } else {
            // DeepSeek, Anthropic, Gemini, Groq, Llama — use char fallback
            TokenizerEncoding::CharFallback
        }
    }

    /// Human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            TokenizerEncoding::Cl100kBase => "cl100k_base",
            TokenizerEncoding::O200kBase => "o200k_base",
            TokenizerEncoding::P50kBase => "p50k_base",
            TokenizerEncoding::R50kBase => "r50k_base",
            TokenizerEncoding::CharFallback => "char/4 fallback",
        }
    }
}

// ── Tokenizer ──────────────────────────────────────────────────────────

/// Multi-model tokenizer with automatic encoding selection
pub struct Tokenizer {
    /// Primary BPE tokenizer (None for char fallback)
    bpe: Option<CoreBPE>,
    /// Selected encoding
    encoding: TokenizerEncoding,
}

impl Tokenizer {
    /// Create a tokenizer for a specific model
    ///
    /// Automatically selects the correct tiktoken encoding based on model name.
    /// Falls back to character-based estimation (~4 chars/token) for unsupported models.
    pub fn for_model(model: &str) -> Result<Self> {
        let encoding = TokenizerEncoding::for_model(model);

        let bpe = match encoding {
            TokenizerEncoding::Cl100kBase => Some(cl100k_base()?),
            TokenizerEncoding::O200kBase => Some(o200k_base()?),
            TokenizerEncoding::P50kBase => Some(p50k_base()?),
            TokenizerEncoding::R50kBase => Some(r50k_base()?),
            TokenizerEncoding::CharFallback => None,
        };

        Ok(Self { bpe, encoding })
    }

    /// Create a tokenizer with a specific encoding
    pub fn with_encoding(encoding: TokenizerEncoding) -> Result<Self> {
        let bpe = match encoding {
            TokenizerEncoding::Cl100kBase => Some(cl100k_base()?),
            TokenizerEncoding::O200kBase => Some(o200k_base()?),
            TokenizerEncoding::P50kBase => Some(p50k_base()?),
            TokenizerEncoding::R50kBase => Some(r50k_base()?),
            TokenizerEncoding::CharFallback => None,
        };

        Ok(Self { bpe, encoding })
    }

    /// What encoding is being used
    pub fn encoding(&self) -> TokenizerEncoding {
        self.encoding
    }

    /// Count tokens in a text string
    pub fn count_tokens(&self, text: &str) -> usize {
        match &self.bpe {
            Some(bpe) => bpe.encode_with_special_tokens(text).len(),
            None => {
                // Character-based fallback: ~4 chars per token (conservative)
                (text.chars().count() / 4).max(1)
            }
        }
    }

    /// Count tokens across multiple messages
    pub fn count_message_tokens(&self, messages: &[super::Message]) -> usize {
        messages
            .iter()
            .map(|msg| {
                msg.tokens
                    .unwrap_or_else(|| self.count_tokens(&msg.content))
            })
            .sum()
    }

    /// Truncate text to fit within a token budget, preserving meaning.
    ///
    /// If the text exceeds `max_tokens`, it is truncated from the **beginning**
    /// (keeping the most recent/relevant tail end). A truncation notice is prepended.
    pub fn truncate_text(&self, text: &str, max_tokens: usize) -> String {
        let current = self.count_tokens(text);

        if current <= max_tokens {
            return text.to_string();
        }

        // Estimate how many chars to keep (conservative)
        let ratio = max_tokens as f64 / current as f64;
        let keep_chars = (text.chars().count() as f64 * ratio * 0.9) as usize; // 90% for safety

        // Find a natural break point
        let truncation_notice = format!("[...truncated {} tokens] ", current - max_tokens);
        let notice_tokens = self.count_tokens(&truncation_notice);
        let available = max_tokens.saturating_sub(notice_tokens);

        // Keep the tail end of the text
        let chars: Vec<char> = text.chars().collect();
        let start = chars.len().saturating_sub(keep_chars.min(available * 4));

        // Try to start at a sentence boundary
        let start = if start > 0 {
            find_sentence_boundary(&chars, start)
        } else {
            0
        };

        let truncated: String = chars[start..].iter().collect();
        format!("{}{}", truncation_notice, truncated)
    }

    /// Truncate messages to fit within a token budget.
    ///
    /// - System messages are always preserved (they carry critical instructions).
    /// - Messages are removed from the **beginning** (oldest first).
    /// - A truncation notice is inserted after the system message.
    pub fn truncate_messages(
        &self,
        messages: &[super::Message],
        max_tokens: usize,
    ) -> Vec<super::Message> {
        let total = self.count_message_tokens(messages);
        if total <= max_tokens {
            return messages.to_vec();
        }

        let mut result = Vec::new();
        let mut used = 0usize;

        // Always keep system messages
        let system_msgs: Vec<_> = messages
            .iter()
            .filter(|m| m.role == "system")
            .cloned()
            .collect();
        let non_system: Vec<_> = messages
            .iter()
            .filter(|m| m.role != "system")
            .cloned()
            .collect();

        for msg in &system_msgs {
            let tokens = msg
                .tokens
                .unwrap_or_else(|| self.count_tokens(&msg.content));
            used += tokens;
            result.push(msg.clone());
        }

        let remaining = max_tokens.saturating_sub(used);

        if remaining == 0 {
            return result;
        }

        // Add truncation notice if we lost messages
        let lost_count = non_system.len().saturating_sub(
            non_system
                .iter()
                .filter(|_| {
                    let fits = used < max_tokens;
                    if fits {
                        used += 1; // placeholder
                    }
                    fits
                })
                .count(),
        );

        if lost_count > 0 {
            let notice = super::Message::new(
                "system",
                format!(
                    "[Context truncated: {} messages removed to fit {} token budget]",
                    lost_count, max_tokens
                ),
            );
            let notice_tokens = self.count_tokens(&notice.content);
            used += notice_tokens;
            result.push(notice);
        }

        // Add messages from newest to oldest until budget exhausted
        let remaining_budget = max_tokens.saturating_sub(used);
        let mut added = 0usize;
        let mut msg_tokens = 0usize;

        for msg in non_system.iter().rev() {
            let tokens = msg
                .tokens
                .unwrap_or_else(|| self.count_tokens(&msg.content));
            if msg_tokens + tokens > remaining_budget {
                break;
            }
            msg_tokens += tokens;
            added += 1;
        }

        // Add the messages we kept (newest ones, in original order)
        let keep_start = non_system.len().saturating_sub(added);
        for msg in &non_system[keep_start..] {
            result.push(msg.clone());
        }

        result
    }

    /// Estimate cost for a set of messages using a given pricing model.
    ///
    /// Returns (input_tokens, estimated_output_tokens, estimated_cost_usd).
    pub fn estimate_cost(
        &self,
        messages: &[super::Message],
        input_price_per_1k: f64,
        output_price_per_1k: f64,
        estimated_output_tokens: usize,
    ) -> CostEstimate {
        let input_tokens = self.count_message_tokens(messages);
        let input_cost = (input_tokens as f64 / 1000.0) * input_price_per_1k;
        let output_cost = (estimated_output_tokens as f64 / 1000.0) * output_price_per_1k;

        CostEstimate {
            input_tokens,
            estimated_output_tokens,
            input_cost_usd: input_cost,
            output_cost_usd: output_cost,
            total_cost_usd: input_cost + output_cost,
        }
    }
}

// ── Cost Estimate ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    pub input_tokens: usize,
    pub estimated_output_tokens: usize,
    pub input_cost_usd: f64,
    pub output_cost_usd: f64,
    pub total_cost_usd: f64,
}

// ── Helper ─────────────────────────────────────────────────────────────

/// Find a reasonable sentence boundary near `pos` in the char array
fn find_sentence_boundary(chars: &[char], pos: usize) -> usize {
    if pos == 0 {
        return 0;
    }

    // Look for period, newline, or other sentence-ending punctuation
    let search_start = pos.saturating_sub(100); // Look back up to 100 chars
    for i in (search_start..pos).rev() {
        match chars[i] {
            '.' | '!' | '?' | '\n' => return (i + 1).min(chars.len()),
            _ => continue,
        }
    }

    pos // Fallback: just cut at the estimated position
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Model Selection ──────────────────────────────────────────

    #[test]
    fn test_encoding_for_gpt4() {
        assert_eq!(
            TokenizerEncoding::for_model("gpt-4"),
            TokenizerEncoding::Cl100kBase
        );
    }

    #[test]
    fn test_encoding_for_gpt4o() {
        assert_eq!(
            TokenizerEncoding::for_model("gpt-4o-mini"),
            TokenizerEncoding::O200kBase
        );
    }

    #[test]
    fn test_encoding_for_gpt35() {
        assert_eq!(
            TokenizerEncoding::for_model("gpt-3.5-turbo"),
            TokenizerEncoding::Cl100kBase
        );
    }

    #[test]
    fn test_encoding_for_deepseek() {
        // DeepSeek uses its own tokenizer → char fallback
        assert_eq!(
            TokenizerEncoding::for_model("deepseek-chat"),
            TokenizerEncoding::CharFallback
        );
    }

    #[test]
    fn test_encoding_for_claude() {
        assert_eq!(
            TokenizerEncoding::for_model("claude-sonnet-4-20250514"),
            TokenizerEncoding::CharFallback
        );
    }

    #[test]
    fn test_encoding_for_gemini() {
        assert_eq!(
            TokenizerEncoding::for_model("gemini-2.0-flash"),
            TokenizerEncoding::CharFallback
        );
    }

    // ── Token Counting ───────────────────────────────────────────

    #[test]
    fn test_count_tokens_cl100k() {
        let tokenizer = Tokenizer::with_encoding(TokenizerEncoding::Cl100kBase).unwrap();
        let tokens = tokenizer.count_tokens("Hello, world!");
        // "Hello, world!" should be ~4 tokens with cl100k
        assert!(tokens >= 2 && tokens <= 6, "Got {} tokens", tokens);
    }

    #[test]
    fn test_count_tokens_o200k() {
        let tokenizer = Tokenizer::with_encoding(TokenizerEncoding::O200kBase).unwrap();
        let tokens = tokenizer.count_tokens("Hello, world!");
        assert!(tokens > 0, "Should count at least 1 token");
    }

    #[test]
    fn test_count_tokens_char_fallback() {
        let tokenizer = Tokenizer::with_encoding(TokenizerEncoding::CharFallback).unwrap();
        let tokens = tokenizer.count_tokens("Hello, world! This is a longer text.");
        // 42 chars / 4 = ~10 tokens
        assert!(tokens >= 8 && tokens <= 12, "Got {} tokens", tokens);
    }

    #[test]
    fn test_count_empty_string() {
        let tokenizer = Tokenizer::with_encoding(TokenizerEncoding::Cl100kBase).unwrap();
        let tokens = tokenizer.count_tokens("");
        assert_eq!(
            tokens, 0,
            "Empty string should count as 0 tokens with BPE"
        );
    }

    #[test]
    fn test_count_message_tokens() {
        let tokenizer = Tokenizer::with_encoding(TokenizerEncoding::Cl100kBase).unwrap();
        let messages = vec![
            super::super::Message::new("user", "Hello"),
            super::super::Message::new("assistant", "Hi there! How can I help?"),
        ];

        let total = tokenizer.count_message_tokens(&messages);
        assert!(total > 4, "Multiple messages should have several tokens");
    }

    #[test]
    fn test_respects_precomputed_tokens() {
        let tokenizer = Tokenizer::with_encoding(TokenizerEncoding::Cl100kBase).unwrap();
        let messages = vec![
            super::super::Message::new(
                "user",
                "A very long message that would normally be many tokens",
            )
            .with_tokens(5), // Pre-computed: only 5 tokens
        ];

        let total = tokenizer.count_message_tokens(&messages);
        assert_eq!(total, 5, "Should respect pre-computed token count");
    }

    // ── Truncation ───────────────────────────────────────────────

    #[test]
    fn test_truncate_text_under_budget() {
        let tokenizer = Tokenizer::with_encoding(TokenizerEncoding::CharFallback).unwrap();
        let text = "Short text";
        let result = tokenizer.truncate_text(text, 100);
        assert_eq!(result, text, "Text under budget should not change");
    }

    #[test]
    fn test_truncate_text_over_budget() {
        let tokenizer = Tokenizer::with_encoding(TokenizerEncoding::CharFallback).unwrap();
        let text = "A".repeat(1000); // ~250 tokens with char/4
        let result = tokenizer.truncate_text(&text, 10);
        assert!(result.len() < text.len(), "Should be shorter");
        assert!(
            result.starts_with("[...truncated"),
            "Should have truncation notice, got: {}",
            &result[..50]
        );
    }

    #[test]
    fn test_truncate_messages_preserves_system() {
        let tokenizer = Tokenizer::with_encoding(TokenizerEncoding::Cl100kBase).unwrap();
        let messages = vec![
            super::super::Message::new("system", "You are a helpful assistant."),
            super::super::Message::new("user", "Hello"),
            super::super::Message::new("assistant", "Hi there!"),
            super::super::Message::new("user", "Tell me a very long story about dragons..."),
        ];

        let result = tokenizer.truncate_messages(&messages, 20);
        assert!(!result.is_empty());
        // System message should always be first
        assert_eq!(result[0].role, "system");
    }

    // ── Cost Estimation ──────────────────────────────────────────

    #[test]
    fn test_estimate_cost() {
        let tokenizer = Tokenizer::with_encoding(TokenizerEncoding::Cl100kBase).unwrap();
        let messages = vec![super::super::Message::new(
            "user",
            "What is the capital of France?",
        )];

        let cost = tokenizer.estimate_cost(&messages, 0.01, 0.03, 50);
        assert!(cost.input_tokens > 0);
        assert_eq!(cost.estimated_output_tokens, 50);
        assert!(cost.total_cost_usd > 0.0);
    }

    #[test]
    fn test_cost_free_model() {
        let tokenizer = Tokenizer::with_encoding(TokenizerEncoding::CharFallback).unwrap();
        let messages = vec![super::super::Message::new("user", "Hello")];

        let cost = tokenizer.estimate_cost(&messages, 0.0, 0.0, 10);
        assert_eq!(cost.total_cost_usd, 0.0);
    }

    // ── Sentence Boundary Detection ──────────────────────────────

    #[test]
    fn test_find_sentence_boundary_at_period() {
        let chars: Vec<char> = "First sentence. Second sentence. Third.".chars().collect();
        let pos = "First sentence. Second sent".len(); // Middle of "Second sentence"
        let boundary = find_sentence_boundary(&chars, pos);
        assert!(
            boundary <= pos + 5 && boundary >= pos.saturating_sub(20),
            "Boundary should be near a sentence break"
        );
    }

    #[test]
    fn test_find_sentence_boundary_at_start() {
        let chars: Vec<char> = "Hello world".chars().collect();
        let boundary = find_sentence_boundary(&chars, 0);
        assert_eq!(boundary, 0);
    }
}
