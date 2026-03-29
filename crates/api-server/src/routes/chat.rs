use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive},
        IntoResponse, Sse,
    },
    Json,
};
use futures::{
    stream::{self, Stream},
    StreamExt,
};
use securellm_core::audit::{AuditEvent, AuditEventType, RequestStatus};
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, sync::Arc, time::Instant};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::{error::ApiResult, state::AppState};

/// OpenAI-compatible chat completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub top_p: Option<f32>,
    #[serde(default)]
    pub n: Option<u32>,
    #[serde(default)]
    pub stream: Option<bool>,
    #[serde(default)]
    pub stop: Option<Vec<String>>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub presence_penalty: Option<f32>,
    #[serde(default)]
    pub frequency_penalty: Option<f32>,
    #[serde(default)]
    pub logit_bias: Option<serde_json::Value>,
    #[serde(default)]
    pub user: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// OpenAI-compatible chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// OpenAI-compatible streaming chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChunk {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChatChunkChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChunkChoice {
    pub index: u32,
    pub delta: ChatDelta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// POST /v1/chat/completions - Chat completion endpoint
///
/// Supports both streaming and non-streaming responses.
/// Routes requests to appropriate providers based on model prefix.
pub async fn chat_completions(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatCompletionRequest>,
) -> ApiResult<axum::response::Response> {
    let request_id = Uuid::new_v4();
    let start = Instant::now();

    info!(
        "Chat completion request: request_id={}, model={}, messages={}, stream={:?}",
        request_id,
        req.model,
        req.messages.len(),
        req.stream
    );

    // Parse provider and model from model string (format: "provider/model")
    let (provider_name, model_name) = parse_model_identifier(&req.model)?;

    debug!(
        "Routing to provider: {}, model: {}",
        provider_name, model_name
    );

    // Log request received
    if let Err(e) = state
        .audit_logger
        .log_request_received(
            request_id,
            &provider_name,
            &model_name,
            req.messages.len(),
            None, // TODO: Extract client IP from headers
        )
        .await
    {
        warn!("Failed to log audit event: {}", e);
    }

    // Check if streaming is requested
    if req.stream.unwrap_or(false) {
        // Return SSE stream
        let stream =
            create_completion_stream(state, req, provider_name, model_name, request_id).await?;
        Ok(Sse::new(stream)
            .keep_alive(KeepAlive::default())
            .into_response())
    } else {
        // Return complete response
        match create_completion(
            state.clone(),
            req.clone(),
            provider_name.clone(),
            model_name.clone(),
        )
        .await
        {
            Ok(response) => {
                let duration = start.elapsed();
                let duration_ms = duration.as_millis() as u64;

                // 1. Calculate cost using Dynamic Pricing Engine
                let prompt_tokens = response.usage.prompt_tokens;
                let completion_tokens = response.usage.completion_tokens;

                let cost = state.pricing_registry.calculate_cost(
                    &provider_name,
                    &model_name,
                    prompt_tokens,
                    completion_tokens,
                );

                // 2. Increment Prometheus metrics
                state
                    .metrics
                    .requests_total
                    .with_label_values(&[&provider_name, "success"])
                    .inc();
                state
                    .metrics
                    .request_duration_seconds
                    .with_label_values(&[&provider_name])
                    .observe(duration.as_secs_f64());
                state
                    .metrics
                    .token_usage_total
                    .with_label_values(&[&provider_name, "prompt"])
                    .inc_by(prompt_tokens as f64);
                state
                    .metrics
                    .token_usage_total
                    .with_label_values(&[&provider_name, "completion"])
                    .inc_by(completion_tokens as f64);
                state
                    .metrics
                    .cost_usd_total
                    .with_label_values(&[&provider_name])
                    .inc_by(cost);

                // 3. Publish NATS events (fire-and-forget)
                {
                    let publisher = state.nats_publisher.clone();
                    let p_name = provider_name.clone();
                    let m_name = model_name.clone();
                    let rid = request_id;
                    let pt = prompt_tokens;
                    let ct = completion_tokens;
                    let dur_ms = duration_ms;
                    tokio::spawn(async move {
                        publisher.publish_llm_request(rid, &m_name, &p_name, pt).await;
                        publisher
                            .publish_llm_response(rid, &m_name, &p_name, ct, dur_ms, "success")
                            .await;
                        publisher.publish_cost_event(&p_name, &m_name, cost, pt + ct).await;
                    });
                }

                // 4. Feed the Anomaly Detector (QoS Observatory)
                state.qos_observatory.observe_request(
                    &provider_name,
                    &model_name,
                    duration,
                    false, // Success
                );

                // 5. Log response sent with precise cost
                let audit_event = AuditEvent {
                    timestamp: chrono::Utc::now(),
                    request_id,
                    event_type: AuditEventType::ResponseSent,
                    user_id: req.user.clone(),
                    provider: provider_name.clone(),
                    model: model_name.clone(),
                    prompt_tokens,
                    completion_tokens,
                    total_tokens: response.usage.total_tokens,
                    estimated_cost_usd: cost,
                    duration_ms,
                    status: RequestStatus::Success,
                    error_message: None,
                    client_ip: None,
                };

                if let Err(e) = state.audit_logger.log_response_sent(&audit_event).await {
                    warn!("Failed to log audit event: {}", e);
                }

                Ok(Json(response).into_response())
            }
            Err(e) => {
                let duration = start.elapsed();
                let duration_ms = duration.as_millis() as u64;

                // Increment error metrics
                state
                    .metrics
                    .requests_total
                    .with_label_values(&[&provider_name, "error"])
                    .inc();
                state
                    .metrics
                    .provider_errors_total
                    .with_label_values(&[&provider_name])
                    .inc();

                // Report failure to QoS Observatory
                state.qos_observatory.observe_request(
                    &provider_name,
                    &model_name,
                    duration,
                    true, // Error
                );

                // Log request failed
                if let Err(log_err) = state
                    .audit_logger
                    .log_request_failed(request_id, &provider_name, &e.to_string(), duration_ms)
                    .await
                {
                    warn!("Failed to log audit event: {}", log_err);
                }

                Err(e)
            }
        }
    }
}

/// Parse model identifier into provider and model name
fn parse_model_identifier(model: &str) -> ApiResult<(String, String)> {
    if let Some((provider, model_name)) = model.split_once('/') {
        Ok((provider.to_string(), model_name.to_string()))
    } else {
        // If no slash, assume it's a model name and try to find the provider
        // Default to deepseek for now
        Ok(("deepseek".to_string(), model.to_string()))
    }
}

use securellm_core::{
    intelligence::RoutingStrategy, request::RequestParameters, Message as CoreMessage,
    MessageContent as CoreMessageContent, MessageRole as CoreMessageRole, Request as CoreRequest,
    Response as CoreResponse,
};

// ... (existing helper functions) ...

/// Create non-streaming completion with Smart Fallback
async fn create_completion(
    state: Arc<AppState>,
    req: ChatCompletionRequest,
    provider_name: String,
    model_name: String,
) -> ApiResult<ChatCompletionResponse> {
    let candidates = if provider_name != "auto" {
        vec![(provider_name.clone(), model_name.clone())]
    } else {
        vec![
            ("llamacpp".to_string(), "local-model".to_string()),
            ("deepseek".to_string(), "deepseek-chat".to_string()),
            ("groq".to_string(), "llama-3.3-70b-versatile".to_string()),
            ("openai".to_string(), "gpt-4-turbo".to_string()),
            ("anthropic".to_string(), "claude-3-sonnet-20240229".to_string()),
        ]
    };

    let ranked_candidates = state
        .routing_engine
        .select_candidates(candidates, RoutingStrategy::LowestCost);

    let mut last_error = None;

    for (p_name, m_name) in ranked_candidates {
        let provider = match state.provider_manager.get_provider(&p_name).await {
            Some(p) => p,
            None => continue,
        };

        tracing::info!("Routing request to {} (model: {})", p_name, m_name);

        let mut core_req = CoreRequest::new(p_name.clone(), m_name.clone());
        core_req.messages = req.messages.iter().cloned().map(convert_message).collect();
        core_req.parameters = RequestParameters {
            temperature: req.temperature,
            top_p: req.top_p,
            max_tokens: req.max_tokens,
            stream: false,
            stop: req.stop.clone(),
            ..Default::default()
        };

        let start = Instant::now();
        match provider.send_request(core_req).await {
            Ok(core_resp) => {
                // Success! Report to QoS and Circuit Breaker
                state
                    .qos_observatory
                    .observe_request(&p_name, &m_name, start.elapsed(), false);
                state.provider_manager.report_result(&p_name, true).await;
                return Ok(convert_response(core_resp));
            }
            Err(e) => {
                tracing::warn!("Provider {} failed: {}. Trying fallback...", p_name, e);
                // Failure! Report to QoS and Circuit Breaker
                state
                    .qos_observatory
                    .observe_request(&p_name, &m_name, start.elapsed(), true);
                state.provider_manager.report_result(&p_name, false).await;
                last_error = Some(e);
                continue;
            }
        }
    }

    Err(crate::error::ApiError::InternalError(format!(
        "All providers failed. Last error: {:?}",
        last_error
    )))
}

fn convert_message(msg: ChatMessage) -> CoreMessage {
    CoreMessage {
        role: match msg.role.as_str() {
            "system" => CoreMessageRole::System,
            "user" => CoreMessageRole::User,
            "assistant" => CoreMessageRole::Assistant,
            "function" => CoreMessageRole::Function,
            _ => CoreMessageRole::User, // Default fallback
        },
        content: CoreMessageContent::Text(msg.content),
        name: msg.name,
        metadata: None,
    }
}

fn convert_response(resp: CoreResponse) -> ChatCompletionResponse {
    ChatCompletionResponse {
        id: resp.id,
        object: "chat.completion".to_string(),
        created: resp.metadata.created_at.timestamp(),
        model: resp.model,
        choices: resp
            .choices
            .into_iter()
            .map(|c| ChatChoice {
                index: c.index,
                message: ChatMessage {
                    role: match c.message.role {
                        CoreMessageRole::System => "system".to_string(),
                        CoreMessageRole::User => "user".to_string(),
                        CoreMessageRole::Assistant => "assistant".to_string(),
                        CoreMessageRole::Function => "function".to_string(),
                    },
                    content: match c.message.content {
                        CoreMessageContent::Text(t) => t,
                        _ => "".to_string(), // TODO: Handle parts
                    },
                    name: c.message.name,
                },
                finish_reason: Some(format!("{:?}", c.finish_reason).to_lowercase()),
            })
            .collect(),
        usage: Usage {
            prompt_tokens: resp.usage.prompt_tokens,
            completion_tokens: resp.usage.completion_tokens,
            total_tokens: resp.usage.total_tokens,
        },
    }
}

/// Create streaming completion
async fn create_completion_stream(
    _state: Arc<AppState>,
    req: ChatCompletionRequest,
    _provider_name: String,
    _model_name: String,
    _request_id: Uuid,
) -> ApiResult<impl Stream<Item = Result<Event, Infallible>>> {
    // TODO: Implement actual provider streaming
    // For now, return a mock stream

    warn!("Using mock stream - provider implementation pending");

    let completion_id = format!("chatcmpl-{}", uuid::Uuid::new_v4());
    let created = chrono::Utc::now().timestamp();
    let model = req.model.clone();

    let chunks = vec![
        ChatCompletionChunk {
            id: completion_id.clone(),
            object: "chat.completion.chunk".to_string(),
            created,
            model: model.clone(),
            choices: vec![ChatChunkChoice {
                index: 0,
                delta: ChatDelta {
                    role: Some("assistant".to_string()),
                    content: None,
                },
                finish_reason: None,
            }],
        },
        ChatCompletionChunk {
            id: completion_id.clone(),
            object: "chat.completion.chunk".to_string(),
            created,
            model: model.clone(),
            choices: vec![ChatChunkChoice {
                index: 0,
                delta: ChatDelta {
                    role: None,
                    content: Some("This ".to_string()),
                },
                finish_reason: None,
            }],
        },
        ChatCompletionChunk {
            id: completion_id.clone(),
            object: "chat.completion.chunk".to_string(),
            created,
            model: model.clone(),
            choices: vec![ChatChunkChoice {
                index: 0,
                delta: ChatDelta {
                    role: None,
                    content: Some("is a mock stream.".to_string()),
                },
                finish_reason: None,
            }],
        },
        ChatCompletionChunk {
            id: completion_id.clone(),
            object: "chat.completion.chunk".to_string(),
            created,
            model: model.clone(),
            choices: vec![ChatChunkChoice {
                index: 0,
                delta: ChatDelta {
                    role: None,
                    content: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
        },
    ];

    let stream = stream::iter(chunks).map(|chunk| {
        let data = serde_json::to_string(&chunk).unwrap();
        Ok(Event::default().data(data))
    });

    Ok(stream)
}
