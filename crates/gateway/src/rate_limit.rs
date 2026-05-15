use crate::audit::{AuditEvent, JsonlSink};
use axum::{
    extract::{Request, State},
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use governor::{DefaultKeyedRateLimiter, Quota, RateLimiter};
use std::{num::NonZeroU32, sync::Arc};
use tracing::{error, warn};

#[derive(Clone)]
pub struct RateLimitState {
    agent_id: String,
    audit: JsonlSink,
    limiter: Arc<DefaultKeyedRateLimiter<String>>,
    per_minute: NonZeroU32,
}

impl RateLimitState {
    pub fn new(agent_id: impl Into<String>, audit: JsonlSink, per_minute: NonZeroU32) -> Self {
        Self {
            agent_id: agent_id.into(),
            audit,
            limiter: Arc::new(RateLimiter::keyed(Quota::per_minute(per_minute))),
            per_minute,
        }
    }
}

pub async fn enforce(State(state): State<RateLimitState>, req: Request, next: Next) -> Response {
    if state.limiter.check_key(&state.agent_id).is_ok() {
        return next.run(req).await;
    }

    warn!(
        agent_id = %state.agent_id,
        per_minute = state.per_minute.get(),
        "gateway HTTP request rate limited"
    );

    let event = AuditEvent::new(&state.agent_id, "gateway_http_request").rate_limited(format!(
        "rate limit exceeded: {} requests/minute",
        state.per_minute
    ));
    if let Err(err) = state.audit.emit(&event).await {
        error!(error = %err, "failed to write rate_limited audit event");
    }

    let mut resp = (StatusCode::TOO_MANY_REQUESTS, "rate limit exceeded").into_response();
    resp.headers_mut()
        .insert(header::RETRY_AFTER, HeaderValue::from_static("60"));
    resp
}
