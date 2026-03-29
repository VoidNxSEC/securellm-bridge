use axum::{extract::State, http::StatusCode, response::IntoResponse};
use prometheus::Encoder;
use std::sync::Arc;
use tracing::debug;

use crate::state::AppState;

/// GET /api/metrics - Prometheus metrics endpoint
pub async fn metrics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    debug!("Metrics requested");

    let encoder = prometheus::TextEncoder::new();
    let metric_families = state.metrics.registry.gather();
    let mut buffer = Vec::new();

    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            [("content-type", "text/plain")],
            format!("# ERROR encoding metrics: {e}").into_bytes(),
        );
    }

    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
        buffer,
    )
}
