// ml-ops-api provider — routes ml-ops/{model} to the remote GPU inference bridge.
//
// ml-ops-api speaks the OpenAI-compatible /v1/chat/completions API, so this
// provider is a thin HTTP proxy with no API-key requirement (internal service).

use async_trait::async_trait;
use reqwest::Client;
use securellm_core::*;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120); // GPU inference can be slow

pub struct MlOpsProvider {
    base_url: String,
    client: Client,
}

// ── OpenAI-compatible wire types ──────────────────────────────────────────────

#[derive(Serialize)]
struct OaiRequest {
    model: String,
    messages: Vec<OaiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Serialize, Deserialize)]
struct OaiMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OaiResponse {
    id: String,
    choices: Vec<OaiChoice>,
    #[serde(default)]
    usage: OaiUsage,
    model: String,
}

#[derive(Deserialize)]
struct OaiChoice {
    message: OaiMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize, Default)]
struct OaiUsage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
    #[serde(default)]
    total_tokens: u32,
}

#[derive(Deserialize)]
struct OaiModelsResponse {
    data: Vec<OaiModelData>,
}

#[derive(Deserialize)]
struct OaiModelData {
    id: String,
}

// ── Provider impl ─────────────────────────────────────────────────────────────

impl MlOpsProvider {
    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        let client = Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .build()
            .map_err(|e| Error::Network(e.to_string()))?;
        Ok(Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client,
        })
    }
}

#[async_trait]
impl LLMProvider for MlOpsProvider {
    fn name(&self) -> &str {
        "ml-ops"
    }

    fn version(&self) -> &str {
        "v1"
    }

    fn validate_config(&self) -> Result<()> {
        Ok(())
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            streaming: false,
            function_calling: false,
            vision: false,
            embeddings: false,
            supports_system_prompts: true,
            max_tokens: Some(8192),
            max_context_window: Some(32768),
        }
    }

    async fn send_request(&self, request: Request) -> Result<Response> {
        let start = Instant::now();

        // Build messages list
        let mut messages: Vec<OaiMessage> = Vec::new();
        if let Some(system) = &request.system {
            messages.push(OaiMessage {
                role: "system".to_string(),
                content: system.clone(),
            });
        }
        for msg in &request.messages {
            messages.push(OaiMessage {
                role: match msg.role {
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::System => "system",
                    MessageRole::Function => "function",
                }
                .to_string(),
                content: msg.content.text().to_string(),
            });
        }

        let oai_req = OaiRequest {
            model: request.model.clone(),
            messages,
            temperature: request.parameters.temperature,
            max_tokens: request.parameters.max_tokens,
        };

        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .json(&oai_req)
            .send()
            .await
            .map_err(|e| Error::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Provider {
                provider: "ml-ops".to_string(),
                message: format!("HTTP {status}: {body}"),
            });
        }

        let oai_resp: OaiResponse = response
            .json()
            .await
            .map_err(|e| Error::Serialization(e.to_string()))?;

        let processing_time = start.elapsed().as_millis() as u64;

        let content = oai_resp
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_default();

        Ok(Response {
            request_id: request.id,
            id: oai_resp.id,
            provider: "ml-ops".to_string(),
            model: oai_resp.model,
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: MessageRole::Assistant,
                    content: MessageContent::Text(content),
                    name: None,
                    metadata: None,
                },
                finish_reason: FinishReason::Stop,
                logprobs: None,
            }],
            usage: TokenUsage {
                prompt_tokens: oai_resp.usage.prompt_tokens,
                completion_tokens: oai_resp.usage.completion_tokens,
                total_tokens: oai_resp.usage.total_tokens,
                estimated_cost: Some(0.0), // local GPU, no external cost
            },
            metadata: ResponseMetadata {
                created_at: chrono::Utc::now(),
                processing_time_ms: processing_time,
                cached: false,
                rate_limit_info: None,
                extra: std::collections::HashMap::new(),
            },
        })
    }

    async fn health_check(&self) -> Result<ProviderHealth> {
        let start = Instant::now();
        let response = self
            .client
            .get(format!("{}/health", self.base_url))
            .send()
            .await;
        let latency = start.elapsed().as_millis() as u64;
        match response {
            Ok(resp) if resp.status().is_success() => Ok(ProviderHealth {
                status: HealthStatus::Healthy,
                latency_ms: Some(latency),
                message: Some(format!("ml-ops-api at {}", self.base_url)),
                timestamp: chrono::Utc::now(),
            }),
            _ => Ok(ProviderHealth {
                status: HealthStatus::Degraded,
                latency_ms: Some(latency),
                message: Some("ml-ops-api not responding".to_string()),
                timestamp: chrono::Utc::now(),
            }),
        }
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let response = self
            .client
            .get(format!("{}/v1/models", self.base_url))
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let models: OaiModelsResponse = resp
                    .json()
                    .await
                    .map_err(|e| Error::Serialization(e.to_string()))?;
                Ok(models
                    .data
                    .into_iter()
                    .map(|m| ModelInfo {
                        id: m.id.clone(),
                        name: m.id,
                        description: Some("ml-ops-api GPU model".to_string()),
                        context_window: Some(32768),
                        max_output_tokens: Some(8192),
                        capabilities: vec!["completion".to_string()],
                        pricing: Some(ModelPricing {
                            input_cost_per_1k: 0.0,
                            output_cost_per_1k: 0.0,
                            currency: "USD".to_string(),
                        }),
                    })
                    .collect())
            }
            _ => Ok(vec![ModelInfo {
                id: "ml-ops/default".to_string(),
                name: "ml-ops/default".to_string(),
                description: Some("ml-ops-api GPU inference (offline)".to_string()),
                context_window: Some(32768),
                max_output_tokens: Some(8192),
                capabilities: vec!["completion".to_string()],
                pricing: Some(ModelPricing {
                    input_cost_per_1k: 0.0,
                    output_cost_per_1k: 0.0,
                    currency: "USD".to_string(),
                }),
            }]),
        }
    }
}
