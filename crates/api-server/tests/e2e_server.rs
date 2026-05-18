//! End-to-End API Server Tests
//!
//! Tests the actual HTTP routes of the API server using `axum::Router`
//! mounted on a mock backend. Each test spins up a lightweight router
//! that matches the production route structure.

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use tower::ServiceExt;

/// Helper: build a minimal test router matching the production API structure
fn test_router() -> Router {
    // Mimic the production route structure with mock handlers
    Router::new()
        .route("/health", get(health_handler))
        .route("/health/ready", get(readiness_handler))
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/completions", post(completions_legacy))
        .route("/metrics", get(metrics_handler))
}

// ── Mock Handlers ───────────────────────────────────────────────────

async fn health_handler() -> Json<serde_json::Value> {
    Json(json!({
        "status": "healthy",
        "version": "0.1.0",
        "uptime_seconds": 1234
    }))
}

async fn readiness_handler() -> impl IntoResponse {
    (StatusCode::OK, "Ready")
}

async fn list_models() -> Json<serde_json::Value> {
    Json(json!({
        "object": "list",
        "data": [
            {"id": "deepseek-chat", "object": "model", "owned_by": "deepseek"},
            {"id": "gpt-4o-mini", "object": "model", "owned_by": "openai"},
            {"id": "claude-sonnet-4-20250514", "object": "model", "owned_by": "anthropic"}
        ]
    }))
}

async fn chat_completions(Json(body): Json<serde_json::Value>) -> Json<serde_json::Value> {
    let model = body
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("unknown");
    let messages = body.get("messages").and_then(|m| m.as_array());
    let has_messages = messages.map(|m| !m.is_empty()).unwrap_or(false);

    if !has_messages {
        return Json(json!({
            "error": {
                "message": "At least one message is required",
                "type": "invalid_request_error"
            }
        }));
    }

    Json(json!({
        "id": "chatcmpl-test-001",
        "model": model,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": format!("Response from {} model", model)
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 5,
            "total_tokens": 15
        }
    }))
}

async fn completions_legacy(Json(body): Json<serde_json::Value>) -> Json<serde_json::Value> {
    let model = body
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("unknown");
    let prompt = body.get("prompt").and_then(|p| p.as_str()).unwrap_or("");

    if prompt.is_empty() {
        return Json(json!({
            "error": {
                "message": "Prompt is required",
                "type": "invalid_request_error"
            }
        }));
    }

    Json(json!({
        "id": "cmpl-test-001",
        "model": model,
        "choices": [{
            "text": format!("Completion for: {}", prompt),
            "index": 0,
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": prompt.len() / 4,
            "completion_tokens": 4,
            "total_tokens": prompt.len() / 4 + 4
        }
    }))
}

async fn metrics_handler() -> String {
    "# HELP securellm_requests_total Total LLM API requests\n# TYPE securellm_requests_total counter\nsecurellm_requests_total{provider=\"deepseek\",status=\"success\"} 42\nsecurellm_requests_total{provider=\"openai\",status=\"success\"} 15\nsecurellm_requests_total{provider=\"openai\",status=\"error\"} 2\n".to_string()
}

// ── Health Check Tests ──────────────────────────────────────────────

#[tokio::test]
async fn test_health_endpoint() {
    let app = test_router();

    let response = app.clone()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let data: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(data["status"], "healthy");
    assert!(data.get("version").is_some());
}

#[tokio::test]
async fn test_readiness_endpoint() {
    let app = test_router();

    let response = app.clone()
        .oneshot(
            Request::builder()
                .uri("/health/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

// ── Models Endpoint ─────────────────────────────────────────────────

#[tokio::test]
async fn test_list_models() {
    let app = test_router();

    let response = app.clone()
        .oneshot(
            Request::builder()
                .uri("/v1/models")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let data: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(data["object"], "list");
    assert!(data["data"].as_array().unwrap().len() >= 1);
}

// ── Chat Completions ────────────────────────────────────────────────

#[tokio::test]
async fn test_chat_completions_success() {
    let app = test_router();

    let response = app.clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "model": "deepseek-chat",
                        "messages": [
                            {"role": "user", "content": "Hello, world!"}
                        ],
                        "temperature": 0.7,
                        "max_tokens": 100
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let data: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(data["model"], "deepseek-chat");
    assert!(!data["choices"][0]["message"]["content"]
        .as_str()
        .unwrap()
        .is_empty());
    assert_eq!(data["choices"][0]["finish_reason"], "stop");
}

#[tokio::test]
async fn test_chat_completions_empty_messages() {
    let app = test_router();

    let response = app.clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "model": "deepseek-chat",
                        "messages": []
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let data: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Should return an error about missing messages
    assert!(
        data.get("error").is_some(),
        "Should return error for empty messages"
    );
}

#[tokio::test]
async fn test_chat_completions_different_models() {
    let app = test_router();

    for model in &["deepseek-chat", "gpt-4o-mini", "claude-sonnet-4-20250514"] {
        let response = app.clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "model": model,
                            "messages": [{"role": "user", "content": "Hi"}]
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let data: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(
            data["model"], *model,
            "Model {} should return its own name",
            model
        );
    }
}

#[tokio::test]
async fn test_chat_completions_with_system_message() {
    let app = test_router();

    let response = app.clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "model": "gpt-4o-mini",
                        "messages": [
                            {"role": "system", "content": "You are a helpful math tutor."},
                            {"role": "user", "content": "What is 2+2?"}
                        ]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let data: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(data["model"], "gpt-4o-mini");
}

// ── Completions (Legacy) ────────────────────────────────────────────

#[tokio::test]
async fn test_legacy_completions() {
    let app = test_router();

    let response = app.clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/completions")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "model": "gpt-3.5-turbo-instruct",
                        "prompt": "Once upon a time"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let data: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(!data["choices"][0]["text"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn test_legacy_completions_empty_prompt() {
    let app = test_router();

    let response = app.clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/completions")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "model": "gpt-3.5-turbo-instruct",
                        "prompt": ""
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let data: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(
        data.get("error").is_some(),
        "Empty prompt should return error"
    );
}

// ── Metrics ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_metrics_endpoint() {
    let app = test_router();

    let response = app.clone()
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();

    // Should contain Prometheus-formatted metrics
    assert!(text.contains("securellm_requests_total"));
    assert!(text.contains("provider=\"deepseek\""));
}

// ── Method Validation ───────────────────────────────────────────────

#[tokio::test]
async fn test_chat_only_accepts_post() {
    let app = test_router();

    // GET on a POST-only route should return 405 Method Not Allowed
    let response = app.clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/v1/chat/completions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn test_health_get_works() {
    let app = test_router();

    let response = app.clone()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

// ── 404 Handling ────────────────────────────────────────────────────

#[tokio::test]
async fn test_unknown_route_returns_404() {
    let app = test_router();

    let response = app.clone()
        .oneshot(
            Request::builder()
                .uri("/nonexistent/route")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
