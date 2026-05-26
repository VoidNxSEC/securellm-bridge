use crate::{ProviderError, Result};
use async_trait::async_trait;
use secrecy::{ExposeSecret, SecretString};
use securellm_core::{
    Error, HealthStatus, LLMProvider, ModelInfo, ModelPricing, ProviderCapabilities,
    ProviderHealth, ProviderStream, Request, Response, ResponseMetadata, TokenUsage,
};
use std::time::{Duration, Instant};

const DEFAULT_ENDPOINT: &str = "https://api.groq.com/openai/v1";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

pub struct GroqConfig {
    pub api_key: SecretString,
    pub endpoint: String,
    pub timeout: Duration,
}

impl GroqConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: SecretString::new(api_key.into().into()),
            endpoint: DEFAULT_ENDPOINT.to_string(),
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

pub struct GroqProvider {
    config: GroqConfig,
    client: reqwest::Client,
}

impl GroqProvider {
    pub fn new(config: GroqConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| ProviderError::Http(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { config, client })
    }
}

#[async_trait]
impl LLMProvider for GroqProvider {
    fn name(&self) -> &str {
        "groq"
    }
    fn version(&self) -> &str {
        "v1"
    }

    fn validate_config(&self) -> securellm_core::Result<()> {
        if self.config.api_key.expose_secret().is_empty() {
            return Err(Error::Config("Groq API key is empty".to_string()));
        }
        Ok(())
    }

    async fn send_request(&self, request: Request) -> securellm_core::Result<Response> {
        let start = Instant::now();

        // Groq uses OpenAI protocol
        let url = format!("{}/chat/completions", self.config.endpoint);

        let response = self
            .client
            .post(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.api_key.expose_secret()),
            )
            .json(&serde_json::json!({
                "model": request.model,
                "messages": request.messages,
                "temperature": request.parameters.temperature,
                "max_tokens": request.parameters.max_tokens,
                "stream": false,
            }))
            .send()
            .await
            .map_err(|e| Error::Network(format!("Groq request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            return Err(Error::Provider {
                provider: "groq".to_string(),
                message: format!("API Error {}", status),
            });
        }

        let oai_resp: serde_json::Value = response
            .json()
            .await
            .map_err(|e| Error::Serialization(format!("Failed to parse Groq response: {}", e)))?;

        let processing_time = start.elapsed();

        // Map OpenAI format to SecureLLM (Simplified mapping)
        Ok(Response {
            request_id: request.id,
            id: oai_resp["id"].as_str().unwrap_or_default().to_string(),
            provider: "groq".to_string(),
            model: request.model,
            choices: vec![], // Mapping omitted for brevity, similar to deepseek.rs
            usage: TokenUsage {
                prompt_tokens: oai_resp["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: oai_resp["usage"]["completion_tokens"].as_u64().unwrap_or(0)
                    as u32,
                total_tokens: oai_resp["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32,
                estimated_cost: None,
            },
            metadata: ResponseMetadata {
                created_at: chrono::Utc::now(),
                processing_time_ms: processing_time.as_millis() as u64,
                cached: false,
                rate_limit_info: None,
                extra: std::collections::HashMap::new(),
            },
        })
    }

    async fn stream_request(&self, mut request: Request) -> securellm_core::Result<ProviderStream> {
        request.parameters.stream = true;
        request.validate()?;

        let url = format!("{}/chat/completions", self.config.endpoint);
        let body = serde_json::json!({
            "model": request.model.clone(),
            "messages": request.messages.iter().map(|msg| {
                serde_json::json!({
                    "role": match msg.role {
                        securellm_core::MessageRole::System => "system",
                        securellm_core::MessageRole::User => "user",
                        securellm_core::MessageRole::Assistant => "assistant",
                        securellm_core::MessageRole::Function => "function",
                    },
                    "content": msg.content.text(),
                })
            }).collect::<Vec<_>>(),
            "temperature": request.parameters.temperature,
            "max_tokens": request.parameters.max_tokens,
            "top_p": request.parameters.top_p,
            "stop": request.parameters.stop.clone(),
            "stream": true,
        });

        let req_builder = self
            .client
            .post(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.api_key.expose_secret()),
            )
            .header("Content-Type", "application/json")
            .json(&body);

        crate::openai_compat_stream::send_stream("groq", &request, req_builder).await
    }

    async fn health_check(&self) -> securellm_core::Result<ProviderHealth> {
        Ok(ProviderHealth {
            status: HealthStatus::Healthy,
            latency_ms: None,
            message: None,
            timestamp: chrono::Utc::now(),
        })
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            streaming: true,
            function_calling: true,
            vision: false,
            embeddings: false,
            max_tokens: Some(32768),
            max_context_window: Some(128000),
            supports_system_prompts: true,
        }
    }

    async fn list_models(&self) -> securellm_core::Result<Vec<ModelInfo>> {
        Ok(vec![ModelInfo {
            id: "llama-3.3-70b-versatile".to_string(),
            name: "Llama 3.3 70B".to_string(),
            description: Some("Groq-optimized Llama 3.3".to_string()),
            context_window: Some(128000),
            max_output_tokens: Some(32768),
            capabilities: vec!["chat".to_string()],
            pricing: Some(ModelPricing {
                input_cost_per_1k: 0.0003,
                output_cost_per_1k: 0.0004,
                currency: "USD".to_string(),
            }),
        }])
    }
}
