// LlamaCpp provider implementation for local models
// Compatible with llama.cpp server API (port 8081)

use async_trait::async_trait;
use reqwest::Client;
use securellm_core::*;
use serde::{Deserialize, Serialize};
use std::time::Instant;

pub struct LlamaCppProvider {
    base_url: String,
    client: Client,
    model_name: String,
}

#[derive(Serialize)]
struct LlamaCppRequest {
    prompt: String,
    temperature: Option<f32>,
    n_predict: Option<i32>,
    stop: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct LlamaCppResponse {
    content: String,
    #[serde(default)]
    tokens_predicted: u32,
    #[serde(default)]
    tokens_evaluated: u32,
}

#[derive(Deserialize)]
struct LlamaCppModelsResponse {
    data: Vec<LlamaCppModelData>,
}

#[derive(Deserialize)]
struct LlamaCppModelData {
    id: String,
    #[serde(default)]
    owned_by: String,
    #[serde(default)]
    meta: Option<LlamaCppModelMeta>,
}

#[derive(Deserialize)]
struct LlamaCppModelMeta {
    #[serde(default)]
    n_ctx_train: Option<u32>,
    #[serde(default)]
    n_params: Option<u64>,
}

impl LlamaCppProvider {
    pub fn new(port: u16, model_name: impl Into<String>) -> Result<Self> {
        Ok(Self {
            base_url: format!("http://localhost:{}", port),
            client: Client::new(),
            model_name: model_name.into(),
        })
    }

    fn build_prompt(&self, request: &Request) -> String {
        let mut prompt = String::new();

        // Add system prompt if present
        if let Some(system) = &request.system {
            prompt.push_str(&format!("System: {}\n\n", system));
        }

        // Add messages
        for msg in &request.messages {
            let role = match msg.role {
                MessageRole::User => "User",
                MessageRole::Assistant => "Assistant",
                MessageRole::System => "System",
                MessageRole::Function => "Function",
            };

            let content = msg.content.text();
            prompt.push_str(&format!("{}: {}\n", role, content));
        }

        prompt.push_str("Assistant: ");
        prompt
    }
}

#[async_trait]
impl LLMProvider for LlamaCppProvider {
    fn name(&self) -> &str {
        "llamacpp"
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
            max_tokens: Some(4096),
            max_context_window: Some(8192),
        }
    }

    async fn send_request(&self, request: Request) -> Result<Response> {
        let start = Instant::now();

        let prompt = self.build_prompt(&request);

        let llama_request = LlamaCppRequest {
            prompt,
            temperature: request.parameters.temperature,
            n_predict: request.parameters.max_tokens.map(|t| t as i32),
            stop: Some(vec!["User:".to_string()]),
        };

        let response = self
            .client
            .post(format!("{}/completion", self.base_url))
            .json(&llama_request)
            .send()
            .await
            .map_err(|e| Error::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(Error::Provider {
                provider: "llamacpp".to_string(),
                message: format!("HTTP {}", response.status()),
            });
        }

        let llama_response: LlamaCppResponse = response
            .json()
            .await
            .map_err(|e| Error::Serialization(e.to_string()))?;

        let processing_time = start.elapsed().as_millis() as u64;

        Ok(Response {
            request_id: request.id,
            id: format!("llamacpp-{}", chrono::Utc::now().timestamp()),
            provider: "llamacpp".to_string(),
            model: self.model_name.clone(),
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: MessageRole::Assistant,
                    content: MessageContent::Text(llama_response.content),
                    name: None,
                    metadata: None,
                },
                finish_reason: FinishReason::Stop,
                logprobs: None,
            }],
            usage: TokenUsage {
                prompt_tokens: llama_response.tokens_evaluated,
                completion_tokens: llama_response.tokens_predicted,
                total_tokens: llama_response.tokens_evaluated + llama_response.tokens_predicted,
                estimated_cost: Some(0.0),
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
                message: Some(format!("LlamaCpp at {}", self.base_url)),
                timestamp: chrono::Utc::now(),
            }),
            _ => Ok(ProviderHealth {
                status: HealthStatus::Degraded,
                latency_ms: Some(latency),
                message: Some("Server not responding".to_string()),
                timestamp: chrono::Utc::now(),
            }),
        }
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        // Try to fetch models from llama.cpp API
        let response = self
            .client
            .get(format!("{}/v1/models", self.base_url))
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let models_response: LlamaCppModelsResponse = resp
                    .json()
                    .await
                    .map_err(|e| Error::Serialization(e.to_string()))?;

                let model_infos: Vec<ModelInfo> = models_response
                    .data
                    .into_iter()
                    .map(|model| {
                        let context_window = model.meta.as_ref().and_then(|m| m.n_ctx_train);

                        ModelInfo {
                            id: model.id.clone(),
                            name: model.id.clone(),
                            description: Some(format!(
                                "LlamaCpp model (owned by: {})",
                                model.owned_by
                            )),
                            context_window: context_window.or(Some(8192)),
                            max_output_tokens: Some(4096),
                            capabilities: vec!["completion".to_string()],
                            pricing: Some(ModelPricing {
                                input_cost_per_1k: 0.0,
                                output_cost_per_1k: 0.0,
                                currency: "USD".to_string(),
                            }),
                        }
                    })
                    .collect();

                if model_infos.is_empty() {
                    // Fallback to default model if API returns empty list
                    Ok(vec![ModelInfo {
                        id: self.model_name.clone(),
                        name: self.model_name.clone(),
                        description: Some("Local LlamaCpp model".to_string()),
                        context_window: Some(8192),
                        max_output_tokens: Some(4096),
                        capabilities: vec!["completion".to_string()],
                        pricing: Some(ModelPricing {
                            input_cost_per_1k: 0.0,
                            output_cost_per_1k: 0.0,
                            currency: "USD".to_string(),
                        }),
                    }])
                } else {
                    Ok(model_infos)
                }
            }
            _ => {
                // Fallback to default model if API is unreachable
                Ok(vec![ModelInfo {
                    id: self.model_name.clone(),
                    name: self.model_name.clone(),
                    description: Some("Local LlamaCpp model (offline)".to_string()),
                    context_window: Some(8192),
                    max_output_tokens: Some(4096),
                    capabilities: vec!["completion".to_string()],
                    pricing: Some(ModelPricing {
                        input_cost_per_1k: 0.0,
                        output_cost_per_1k: 0.0,
                        currency: "USD".to_string(),
                    }),
                }])
            }
        }
    }
}
