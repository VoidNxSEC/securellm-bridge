use crate::{ProviderError, Result};
use async_trait::async_trait;
use secrecy::{ExposeSecret, SecretString};
use securellm_core::{
    Choice, ContentPart, Error, FinishReason, HealthStatus, LLMProvider, Message, MessageContent,
    MessageRole, ModelInfo, ModelPricing, ProviderCapabilities, ProviderHealth, Request, Response,
    ResponseMetadata, TokenUsage,
};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

const DEFAULT_ENDPOINT: &str = "https://api.anthropic.com/v1";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Anthropic provider configuration
#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    /// API key for authentication
    pub api_key: SecretString,

    /// API endpoint (defaults to https://api.anthropic.com/v1)
    pub endpoint: String,

    /// Request timeout
    pub timeout: Duration,

    /// Enable request/response logging
    pub logging_enabled: bool,
}

impl AnthropicConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: SecretString::new(api_key.into().into()),
            endpoint: DEFAULT_ENDPOINT.to_string(),
            timeout: DEFAULT_TIMEOUT,
            logging_enabled: false,
        }
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_logging(mut self, enabled: bool) -> Self {
        self.logging_enabled = enabled;
        self
    }
}

/// Anthropic provider implementation
pub struct AnthropicProvider {
    config: AnthropicConfig,
    client: reqwest::Client,
}

impl AnthropicProvider {
    pub fn new(config: AnthropicConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| ProviderError::Http(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { config, client })
    }

    /// Convert SecureLLM request to Anthropic API format
    fn convert_request(&self, request: &Request) -> Result<AnthropicRequest> {
        // Extract system message if present
        let system = request.system.clone();

        // Convert messages (Anthropic doesn't allow system role in messages array)
        let messages = request
            .messages
            .iter()
            .filter(|msg| msg.role != MessageRole::System)
            .map(|msg| {
                AnthropicMessage {
                    role: match msg.role {
                        MessageRole::User => "user".to_string(),
                        MessageRole::Assistant => "assistant".to_string(),
                        _ => "user".to_string(), // Fallback to user
                    },
                    content: match &msg.content {
                        MessageContent::Text(text) => text.clone(),
                        MessageContent::Parts(parts) => {
                            // Combine text parts
                            parts
                                .iter()
                                .filter_map(|part| {
                                    if let ContentPart::Text { text } = part {
                                        Some(text.as_str())
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join(" ")
                        }
                    },
                }
            })
            .collect();

        Ok(AnthropicRequest {
            model: request.model.clone(),
            messages,
            max_tokens: request.parameters.max_tokens.unwrap_or(4096),
            temperature: request.parameters.temperature,
            top_p: request.parameters.top_p,
            system,
            stop_sequences: request.parameters.stop.clone(),
        })
    }

    /// Convert Anthropic API response to SecureLLM format
    fn convert_response(
        &self,
        request_id: uuid::Uuid,
        anthropic_response: AnthropicResponse,
        processing_time: Duration,
    ) -> Result<Response> {
        let choices = vec![Choice {
            index: 0,
            message: Message {
                role: MessageRole::Assistant,
                content: MessageContent::Text(
                    anthropic_response
                        .content
                        .first()
                        .map(|c| c.text.clone())
                        .unwrap_or_default(),
                ),
                name: None,
                metadata: None,
            },
            finish_reason: match anthropic_response.stop_reason.as_deref() {
                Some("end_turn") => FinishReason::Stop,
                Some("max_tokens") => FinishReason::Length,
                Some("stop_sequence") => FinishReason::Stop,
                _ => FinishReason::Unknown,
            },
            logprobs: None,
        }];

        let usage = TokenUsage {
            prompt_tokens: anthropic_response.usage.input_tokens,
            completion_tokens: anthropic_response.usage.output_tokens,
            total_tokens: anthropic_response.usage.input_tokens
                + anthropic_response.usage.output_tokens,
            estimated_cost: None,
        };

        let mut metadata = ResponseMetadata {
            created_at: chrono::Utc::now(),
            processing_time_ms: processing_time.as_millis() as u64,
            cached: false,
            rate_limit_info: None,
            extra: std::collections::HashMap::new(),
        };

        metadata.extra.insert(
            "anthropic_id".to_string(),
            serde_json::Value::String(anthropic_response.id.clone()),
        );
        metadata.extra.insert(
            "anthropic_type".to_string(),
            serde_json::Value::String(anthropic_response.type_.clone()),
        );

        Ok(Response {
            request_id,
            id: anthropic_response.id,
            provider: "anthropic".to_string(),
            model: anthropic_response.model,
            choices,
            usage,
            metadata,
        })
    }
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn version(&self) -> &str {
        ANTHROPIC_VERSION
    }

    fn validate_config(&self) -> securellm_core::Result<()> {
        if self.config.api_key.expose_secret().is_empty() {
            return Err(Error::Config("Anthropic API key is empty".to_string()));
        }

        if self.config.endpoint.is_empty() {
            return Err(Error::Config("Anthropic endpoint is empty".to_string()));
        }

        Ok(())
    }

    async fn send_request(&self, request: Request) -> securellm_core::Result<Response> {
        // Validate request
        request.validate()?;

        // Log request if enabled
        if self.config.logging_enabled {
            tracing::info!(
                request_id = %request.id,
                model = %request.model,
                "Sending request to Anthropic"
            );
        }

        // Convert to Anthropic format
        let anthropic_request = self
            .convert_request(&request)
            .map_err(|e| Error::Provider {
                provider: "anthropic".to_string(),
                message: format!("Request conversion failed: {}", e),
            })?;

        // Build HTTP request
        let url = format!("{}/messages", self.config.endpoint);
        let start = Instant::now();

        let req_builder = self
            .client
            .post(&url)
            .header("x-api-key", self.config.api_key.expose_secret())
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("Content-Type", "application/json")
            .json(&anthropic_request);

        // Send request
        let response = req_builder
            .send()
            .await
            .map_err(|e| Error::Network(format!("HTTP request failed: {}", e)))?;

        let status = response.status();
        let processing_time = start.elapsed();

        // Handle errors
        if !status.is_success() {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            return Err(Error::Provider {
                provider: "anthropic".to_string(),
                message: format!("API error ({}): {}", status, error_body),
            });
        }

        // Parse response
        let anthropic_response: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| Error::Serialization(format!("Failed to parse response: {}", e)))?;

        // Convert to SecureLLM format
        let securellm_response = self
            .convert_response(request.id, anthropic_response, processing_time)
            .map_err(|e| Error::Provider {
                provider: "anthropic".to_string(),
                message: format!("Response conversion failed: {}", e),
            })?;

        // Log response if enabled
        if self.config.logging_enabled {
            tracing::info!(
                request_id = %request.id,
                tokens = securellm_response.usage.total_tokens,
                duration_ms = processing_time.as_millis(),
                "Received response from Anthropic"
            );
        }

        Ok(securellm_response)
    }

    async fn health_check(&self) -> securellm_core::Result<ProviderHealth> {
        let start = Instant::now();

        // Simple ping check (Anthropic doesn't have a models endpoint)
        let url = format!("{}/messages", self.config.endpoint);

        // Send a minimal test request
        let test_request = AnthropicRequest {
            model: "claude-3-haiku-20240307".to_string(),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: "Hi".to_string(),
            }],
            max_tokens: 1,
            temperature: None,
            top_p: None,
            system: None,
            stop_sequences: None,
        };

        let response = self
            .client
            .post(&url)
            .header("x-api-key", self.config.api_key.expose_secret())
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("Content-Type", "application/json")
            .json(&test_request)
            .send()
            .await;

        let latency = start.elapsed();

        let status = match response {
            Ok(resp) if resp.status().is_success() => HealthStatus::Healthy,
            Ok(resp) if resp.status().is_server_error() => HealthStatus::Degraded,
            _ => HealthStatus::Unhealthy,
        };

        Ok(ProviderHealth {
            status,
            latency_ms: Some(latency.as_millis() as u64),
            message: None,
            timestamp: chrono::Utc::now(),
        })
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            streaming: true,
            function_calling: true,
            vision: true,
            embeddings: false,
            max_tokens: Some(4096),
            max_context_window: Some(200000),
            supports_system_prompts: true,
        }
    }

    async fn list_models(&self) -> securellm_core::Result<Vec<ModelInfo>> {
        // Anthropic's main models
        Ok(vec![
            ModelInfo {
                id: "claude-3-opus-20240229".to_string(),
                name: "Claude 3 Opus".to_string(),
                description: Some(
                    "Most capable Claude model with superior performance".to_string(),
                ),
                context_window: Some(200000),
                max_output_tokens: Some(4096),
                capabilities: vec!["chat".to_string(), "vision".to_string()],
                pricing: Some(ModelPricing {
                    input_cost_per_1k: 0.015,
                    output_cost_per_1k: 0.075,
                    currency: "USD".to_string(),
                }),
            },
            ModelInfo {
                id: "claude-3-sonnet-20240229".to_string(),
                name: "Claude 3 Sonnet".to_string(),
                description: Some(
                    "Balanced intelligence and speed for enterprise workloads".to_string(),
                ),
                context_window: Some(200000),
                max_output_tokens: Some(4096),
                capabilities: vec!["chat".to_string(), "vision".to_string()],
                pricing: Some(ModelPricing {
                    input_cost_per_1k: 0.003,
                    output_cost_per_1k: 0.015,
                    currency: "USD".to_string(),
                }),
            },
            ModelInfo {
                id: "claude-3-haiku-20240307".to_string(),
                name: "Claude 3 Haiku".to_string(),
                description: Some(
                    "Fastest and most compact model for near-instant responses".to_string(),
                ),
                context_window: Some(200000),
                max_output_tokens: Some(4096),
                capabilities: vec!["chat".to_string(), "vision".to_string()],
                pricing: Some(ModelPricing {
                    input_cost_per_1k: 0.00025,
                    output_cost_per_1k: 0.00125,
                    currency: "USD".to_string(),
                }),
            },
        ])
    }
}

// Anthropic API types
#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    id: String,
    #[serde(rename = "type")]
    type_: String,
    model: String,
    content: Vec<AnthropicContent>,
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    type_: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = AnthropicConfig::new("test_key")
            .with_timeout(Duration::from_secs(30))
            .with_logging(true);

        assert_eq!(config.api_key.expose_secret(), "test_key");
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert!(config.logging_enabled);
    }

    #[test]
    fn test_provider_capabilities() {
        let config = AnthropicConfig::new("test");
        let provider = AnthropicProvider::new(config).unwrap();

        let caps = provider.capabilities();
        assert!(caps.streaming);
        assert!(caps.function_calling);
        assert!(caps.vision);
        assert_eq!(caps.max_context_window, Some(200000));
    }

    #[tokio::test]
    async fn test_list_models() {
        let config = AnthropicConfig::new("test");
        let provider = AnthropicProvider::new(config).unwrap();

        let models = provider.list_models().await.unwrap();
        assert!(models.len() >= 3);
        assert!(models.iter().any(|m| m.id.contains("claude-3")));
    }
}
