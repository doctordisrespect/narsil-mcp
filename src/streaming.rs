//! Streaming responses for large result sets
//!
//! This module provides infrastructure for streaming large results to avoid timeouts.
//! Phase C3: Activated and integrated with EngineOptions.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

/// Configuration for streaming responses
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    /// Default chunk size for streaming
    pub default_chunk_size: usize,
    /// Automatically stream results larger than this threshold
    pub auto_stream_threshold: usize,
    /// Enable streaming by default
    pub enabled: bool,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            default_chunk_size: 50,
            auto_stream_threshold: 100,
            enabled: true,
        }
    }
}

/// Progress token for tracking streaming operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressToken {
    /// Unique token ID for this streaming operation
    pub token: String,
    /// Total number of items to stream
    pub total: usize,
    /// Current progress (items sent so far)
    pub current: usize,
}

impl ProgressToken {
    pub fn new(operation: &str, total: usize) -> Self {
        Self {
            token: format!("{}_{}", operation, uuid::Uuid::new_v4()),
            total,
            current: 0,
        }
    }

    pub fn percentage(&self) -> u32 {
        if self.total == 0 {
            100
        } else {
            ((self.current as f64 / self.total as f64) * 100.0) as u32
        }
    }

    pub fn is_complete(&self) -> bool {
        self.current >= self.total
    }
}

/// A notification sent during streaming
#[derive(Debug, Serialize)]
pub struct ProgressNotification {
    jsonrpc: String,
    method: String,
    params: ProgressParams,
}

#[derive(Debug, Serialize)]
struct ProgressParams {
    token: String,
    value: ProgressValue,
}

#[derive(Debug, Serialize)]
struct ProgressValue {
    kind: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    percentage: Option<u32>,
}

impl ProgressNotification {
    pub fn report(token: &str, message: String, percentage: Option<u32>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: "notifications/progress".to_string(),
            params: ProgressParams {
                token: token.to_string(),
                value: ProgressValue {
                    kind: "report".to_string(),
                    message,
                    percentage,
                },
            },
        }
    }

    pub fn complete(token: &str, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: "notifications/progress".to_string(),
            params: ProgressParams {
                token: token.to_string(),
                value: ProgressValue {
                    kind: "end".to_string(),
                    message,
                    percentage: Some(100),
                },
            },
        }
    }
}

/// Trait for types that can be streamed
pub trait Streamable {
    /// Get the total number of items
    fn total_items(&self) -> usize;

    /// Split into chunks of the specified size
    fn split_into_chunks(&self, chunk_size: usize) -> Vec<Vec<String>>;

    /// Format a chunk for display
    fn format_chunk(&self, chunk: &[String], chunk_index: usize, total_chunks: usize) -> String;
}

/// A streaming response handler
pub struct StreamingResponse {
    config: StreamingConfig,
    stdout: Arc<Mutex<tokio::io::Stdout>>,
}

impl StreamingResponse {
    pub fn new(config: StreamingConfig) -> Self {
        Self {
            config,
            stdout: Arc::new(Mutex::new(tokio::io::stdout())),
        }
    }

    /// Check if streaming should be enabled for the given number of items
    pub fn should_stream(&self, total_items: usize) -> bool {
        self.config.enabled && total_items > self.config.auto_stream_threshold
    }

    /// Send a progress notification
    pub async fn send_notification(&self, notification: &ProgressNotification) -> Result<()> {
        let mut stdout = self.stdout.lock().await;
        let notification_str = serde_json::to_string(notification)? + "\n";
        stdout.write_all(notification_str.as_bytes()).await?;
        stdout.flush().await?;
        Ok(())
    }

    /// Stream results in chunks
    pub async fn stream_chunks<T: Streamable>(
        &self,
        operation: &str,
        data: &T,
        chunk_size: usize,
    ) -> Result<(Vec<String>, ProgressToken)> {
        let total = data.total_items();
        let mut progress = ProgressToken::new(operation, total);

        // Split data into chunks
        let chunks = data.split_into_chunks(chunk_size);
        let total_chunks = chunks.len();

        // Send initial notification
        self.send_notification(&ProgressNotification::report(
            &progress.token,
            format!("Starting {} - {} items to process", operation, total),
            Some(0),
        ))
            .await?;

        let mut all_results = Vec::new();

        // Stream each chunk
        for (i, chunk) in chunks.iter().enumerate() {
            progress.current += chunk.len();

            // Format and send the chunk notification
            let _chunk_content = data.format_chunk(chunk, i, total_chunks);
            all_results.extend(chunk.clone());

            let message = format!(
                "Processed {} of {} items (chunk {} of {})",
                progress.current,
                total,
                i + 1,
                total_chunks
            );

            self.send_notification(&ProgressNotification::report(
                &progress.token,
                message,
                Some(progress.percentage()),
            ))
                .await?;

            // Small delay to avoid overwhelming the client
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        // Send completion notification
        self.send_notification(&ProgressNotification::complete(
            &progress.token,
            format!("Completed {} - {} items processed", operation, total),
        ))
            .await?;

        Ok((all_results, progress))
    }
}

/// Wrapper for symbols that implements Streamable
pub struct StreamableSymbols {
    pub symbols: Vec<String>,
}

impl Streamable for StreamableSymbols {
    fn total_items(&self) -> usize {
        self.symbols.len()
    }

    fn split_into_chunks(&self, chunk_size: usize) -> Vec<Vec<String>> {
        self.symbols
            .chunks(chunk_size)
            .map(|chunk| chunk.to_vec())
            .collect()
    }

    fn format_chunk(&self, chunk: &[String], chunk_index: usize, _total_chunks: usize) -> String {
        format!(
            "Chunk {} ({} symbols):\n{}",
            chunk_index + 1,
            chunk.len(),
            chunk.join("\n")
        )
    }
}

/// Wrapper for code search results that implements Streamable
pub struct StreamableSearchResults {
    pub results: Vec<String>,
}

impl Streamable for StreamableSearchResults {
    fn total_items(&self) -> usize {
        self.results.len()
    }

    fn split_into_chunks(&self, chunk_size: usize) -> Vec<Vec<String>> {
        self.results
            .chunks(chunk_size)
            .map(|chunk| chunk.to_vec())
            .collect()
    }

    fn format_chunk(&self, chunk: &[String], chunk_index: usize, _total_chunks: usize) -> String {
        format!(
            "Results chunk {} ({} results):\n{}",
            chunk_index + 1,
            chunk.len(),
            chunk.join("\n\n---\n\n")
        )
    }
}

/// Wrapper for references that implements Streamable
pub struct StreamableReferences {
    pub references: Vec<String>,
}

impl Streamable for StreamableReferences {
    fn total_items(&self) -> usize {
        self.references.len()
    }

    fn split_into_chunks(&self, chunk_size: usize) -> Vec<Vec<String>> {
        self.references
            .chunks(chunk_size)
            .map(|chunk| chunk.to_vec())
            .collect()
    }

    fn format_chunk(&self, chunk: &[String], chunk_index: usize, _total_chunks: usize) -> String {
        format!(
            "References chunk {} ({} references):\n{}",
            chunk_index + 1,
            chunk.len(),
            chunk.join("\n")
        )
    }
}

/// Build initial response with streaming metadata
pub fn build_streaming_response(
    initial_results: Vec<String>,
    progress_token: &ProgressToken,
    operation: &str,
) -> Value {
    json!({
        "streaming": true,
        "progress_token": progress_token.token,
        "total_items": progress_token.total,
        "operation": operation,
        "message": format!(
            "Streaming {} items. Monitor progress via token: {}",
            progress_token.total,
            progress_token.token
        ),
        "initial_batch": initial_results.first().cloned().unwrap_or_default(),
        "note": "Results are being streamed via progress notifications. Full results will be sent progressively."
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_token() {
        let mut token = ProgressToken::new("test_operation", 100);
        assert_eq!(token.percentage(), 0);
        assert!(!token.is_complete());

        token.current = 50;
        assert_eq!(token.percentage(), 50);
        assert!(!token.is_complete());

        token.current = 100;
        assert_eq!(token.percentage(), 100);
        assert!(token.is_complete());
    }

    #[test]
    fn test_streamable_symbols() {
        let symbols = StreamableSymbols {
            symbols: (0..150).map(|i| format!("symbol_{}", i)).collect(),
        };

        assert_eq!(symbols.total_items(), 150);

        let chunks = symbols.split_into_chunks(50);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].len(), 50);
        assert_eq!(chunks[1].len(), 50);
        assert_eq!(chunks[2].len(), 50);
    }

    #[test]
    fn test_streaming_config() {
        let config = StreamingConfig::default();
        assert_eq!(config.default_chunk_size, 50);
        assert_eq!(config.auto_stream_threshold, 100);
        assert!(config.enabled);
    }

    // Phase C3: Test streaming threshold configurability
    #[test]
    fn test_streaming_threshold_configurable() {
        let config = StreamingConfig {
            auto_stream_threshold: 50,
            ..Default::default()
        };
        let response = StreamingResponse::new(config);

        // Should stream when above threshold
        assert!(response.should_stream(51), "51 items should trigger streaming with threshold 50");
        assert!(response.should_stream(100), "100 items should trigger streaming with threshold 50");

        // Should not stream when at or below threshold
        assert!(!response.should_stream(50), "50 items should not trigger streaming with threshold 50");
        assert!(!response.should_stream(25), "25 items should not trigger streaming with threshold 50");
    }

    #[test]
    fn test_streaming_disabled() {
        let config = StreamingConfig {
            enabled: false,
            ..Default::default()
        };
        let response = StreamingResponse::new(config);

        // Should never stream when disabled
        assert!(!response.should_stream(1000), "Should not stream when disabled, even with 1000 items");
    }
}