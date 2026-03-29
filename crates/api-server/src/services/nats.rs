//! NATS event publisher for SecureLLM Bridge.
//!
//! Publishes `llm.request.v1`, `llm.response.v1`, and `cost.incurred.v1` events
//! to the Spectre event bus.  All publishes are fire-and-forget; NATS being
//! unavailable is non-fatal.

use chrono::Utc;
use serde_json::json;
use tracing::{debug, warn};
use uuid::Uuid;

/// Wrapper around an optional async_nats client.
///
/// The `Option` lets us start the bridge even when NATS is not configured.
pub struct NatsPublisher {
    client: Option<async_nats::Client>,
}

impl NatsPublisher {
    /// Connect to NATS at `nats_url`.  Returns a publisher whether or not the
    /// connection succeeds — on failure the client is `None` and all publishes
    /// are silently dropped.
    pub async fn connect(nats_url: &str) -> Self {
        match async_nats::connect(nats_url).await {
            Ok(client) => {
                tracing::info!("NATS publisher connected: {}", nats_url);
                Self {
                    client: Some(client),
                }
            }
            Err(e) => {
                warn!("NATS publisher connection failed (non-fatal): {}", e);
                Self { client: None }
            }
        }
    }

    /// No-op publisher for use when NATS_URL is not set.
    pub fn disabled() -> Self {
        Self { client: None }
    }

    /// Publish `llm.request.v1`.
    pub async fn publish_llm_request(
        &self,
        request_id: Uuid,
        model: &str,
        provider: &str,
        prompt_tokens: u32,
    ) {
        let payload = json!({
            "event_id": Uuid::new_v4().to_string(),
            "source_service": "securellm-bridge",
            "request_id": request_id.to_string(),
            "model": model,
            "provider": provider,
            "prompt_tokens": prompt_tokens,
            "timestamp": Utc::now().to_rfc3339(),
        });
        self.publish("llm.request.v1", &payload).await;
    }

    /// Publish `llm.response.v1`.
    pub async fn publish_llm_response(
        &self,
        request_id: Uuid,
        model: &str,
        provider: &str,
        completion_tokens: u32,
        duration_ms: u64,
        status: &str,
    ) {
        let payload = json!({
            "event_id": Uuid::new_v4().to_string(),
            "source_service": "securellm-bridge",
            "request_id": request_id.to_string(),
            "model": model,
            "provider": provider,
            "completion_tokens": completion_tokens,
            "duration_ms": duration_ms,
            "status": status,
            "timestamp": Utc::now().to_rfc3339(),
        });
        self.publish("llm.response.v1", &payload).await;
    }

    /// Publish `cost.incurred.v1`.
    pub async fn publish_cost_event(
        &self,
        provider: &str,
        model: &str,
        cost_usd: f64,
        total_tokens: u32,
    ) {
        let payload = json!({
            "event_id": Uuid::new_v4().to_string(),
            "source_service": "securellm-bridge",
            "provider": provider,
            "model": model,
            "cost_usd": cost_usd,
            "total_tokens": total_tokens,
            "timestamp": Utc::now().to_rfc3339(),
        });
        self.publish("cost.incurred.v1", &payload).await;
    }

    async fn publish(&self, subject: &str, payload: &serde_json::Value) {
        let Some(ref client) = self.client else {
            return;
        };
        let data = payload.to_string().into_bytes();
        match client.publish(subject.to_string(), data.into()).await {
            Ok(_) => debug!("Published NATS event: {}", subject),
            Err(e) => warn!("NATS publish failed (non-fatal) on {}: {}", subject, e),
        }
    }
}
