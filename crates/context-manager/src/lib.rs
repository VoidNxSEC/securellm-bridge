//! Context Management and Compression System
//!
//! Provides token-aware context compression using zstd,
//! LRU caching, and sliding window optimization.

use anyhow::{Context, Result};
use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::num::NonZeroUsize;
use std::sync::Mutex;
use tiktoken_rs::{cl100k_base, CoreBPE};

mod compression;
mod sliding_window;
mod tokenizer;

pub use compression::Compressor;
pub use tokenizer::{CostEstimate, Tokenizer, TokenizerEncoding};
pub use sliding_window::SlidingWindow;

/// Represents a chat message
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub tokens: Option<usize>,
}

impl Message {
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            tokens: None,
        }
    }

    pub fn with_tokens(mut self, tokens: usize) -> Self {
        self.tokens = Some(tokens);
        self
    }
}

/// Compressed context representation
#[derive(Debug, Clone)]
pub struct CompressedContext {
    /// Zstd-compressed data
    pub data: Vec<u8>,

    /// Original token count
    pub original_tokens: usize,

    /// Compressed byte size
    pub compressed_bytes: usize,

    /// Original byte size (before compression)
    pub original_bytes: usize,
}

impl CompressedContext {
    /// Calculate compression ratio
    pub fn ratio(&self) -> f32 {
        if self.compressed_bytes == 0 {
            return 0.0;
        }
        self.original_bytes as f32 / self.compressed_bytes as f32
    }

    /// Space savings percentage
    pub fn savings_percent(&self) -> f32 {
        if self.original_bytes == 0 {
            return 0.0;
        }
        ((self.original_bytes - self.compressed_bytes) as f32 / self.original_bytes as f32) * 100.0
    }
}

/// Context manager with compression and caching
pub struct ContextManager {
    cache: Mutex<LruCache<String, CompressedContext>>,
    tokenizer: CoreBPE,
    compression_level: i32,
    compressor: Compressor,
}

impl ContextManager {
    /// Create new context manager
    pub fn new() -> Result<Self> {
        let tokenizer = cl100k_base().context("Failed to initialize tokenizer")?;

        Ok(Self {
            cache: Mutex::new(LruCache::new(NonZeroUsize::new(100).unwrap())),
            tokenizer,
            compression_level: 10, // zstd default
            compressor: Compressor::new(10),
        })
    }

    /// Create with custom compression level (1-22)
    pub fn with_compression_level(level: i32) -> Result<Self> {
        let tokenizer = cl100k_base().context("Failed to initialize tokenizer")?;

        Ok(Self {
            cache: Mutex::new(LruCache::new(NonZeroUsize::new(100).unwrap())),
            tokenizer,
            compression_level: level,
            compressor: Compressor::new(level),
        })
    }

    /// Count tokens in text
    pub fn count_tokens(&self, text: &str) -> usize {
        self.tokenizer.encode_with_special_tokens(text).len()
    }

    /// Count tokens in messages
    pub fn count_message_tokens(&self, messages: &[Message]) -> usize {
        messages
            .iter()
            .map(|msg| {
                msg.tokens
                    .unwrap_or_else(|| self.count_tokens(&msg.content))
            })
            .sum()
    }

    /// Compress messages
    pub fn compress(&self, messages: &[Message]) -> Result<CompressedContext> {
        let json = serde_json::to_string(messages)?;
        let original_bytes = json.as_bytes().len();
        let original_tokens = self.count_message_tokens(messages);

        let compressed = self.compressor.compress(json.as_bytes())?;
        let compressed_bytes = compressed.len();

        Ok(CompressedContext {
            data: compressed,
            original_tokens,
            compressed_bytes,
            original_bytes,
        })
    }

    /// Decompress back to messages
    pub fn decompress(&self, ctx: &CompressedContext) -> Result<Vec<Message>> {
        let decompressed = self.compressor.decompress(&ctx.data)?;
        let json = String::from_utf8(decompressed)?;
        Ok(serde_json::from_str(&json)?)
    }

    /// Optimize context using sliding window
    /// Keeps most recent messages and summarizes older ones
    pub fn optimize_context(&self, messages: &[Message], max_tokens: usize) -> Vec<Message> {
        let window = SlidingWindow::new(max_tokens);
        window.optimize(messages, &self.tokenizer)
    }

    /// Cache compressed context
    pub fn cache_put(&self, key: String, ctx: CompressedContext) {
        let mut cache = self.cache.lock().unwrap();
        cache.put(key, ctx);
    }

    /// Retrieve from cache
    pub fn cache_get(&self, key: &str) -> Option<CompressedContext> {
        let mut cache = self.cache.lock().unwrap();
        cache.get(key).cloned()
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> CacheStats {
        let cache = self.cache.lock().unwrap();
        CacheStats {
            size: cache.len(),
            capacity: cache.cap().get(),
        }
    }
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new().expect("Failed to initialize context manager")
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size: usize,
    pub capacity: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_counting() {
        let manager = ContextManager::new().unwrap();
        let tokens = manager.count_tokens("Hello, world!");
        assert!(tokens > 0);
    }

    #[test]
    fn test_compression_roundtrip() {
        let manager = ContextManager::new().unwrap();
        let messages = vec![
            Message::new("user", "Hello"),
            Message::new("assistant", "Hi there!"),
        ];

        let compressed = manager.compress(&messages).unwrap();
        assert!(compressed.ratio() > 1.0);

        let decompressed = manager.decompress(&compressed).unwrap();
        assert_eq!(messages, decompressed);
    }

    #[test]
    fn test_compression_ratio() {
        let manager = ContextManager::new().unwrap();

        // Long repetitive text compresses well
        let long_messages = (0..100)
            .map(|i| Message::new("user", format!("Test message number {}", i)))
            .collect::<Vec<_>>();

        let compressed = manager.compress(&long_messages).unwrap();

        // Should achieve significant compression
        assert!(compressed.ratio() > 2.0);
        assert!(compressed.savings_percent() > 50.0);
    }
}
