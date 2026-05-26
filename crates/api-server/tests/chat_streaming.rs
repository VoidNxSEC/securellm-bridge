use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    routing::post,
    Router,
};
use securellm_api_server::{
    config::{CircuitBreakerConfig, Config, ProviderConfig},
    routes,
    state::AppState,
};
use serde_json::json;
use tower::ServiceExt;
use wiremock::matchers::{header as wm_header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn openai_only_config(openai_base_url: String) -> Config {
    let mut config = Config::default();
    config.database.url = "sqlite::memory:".to_string();
    config.providers.deepseek = None;
    config.providers.anthropic = None;
    config.providers.groq = None;
    config.providers.cohere = None;
    config.providers.llamacpp = None;
    config.providers.gemini = None;
    config.providers.nvidia = None;
    config.providers.ml_ops = None;
    config.providers.openai = Some(ProviderConfig {
        enabled: true,
        api_key: "test-openai-key".to_string(),
        base_url: Some(openai_base_url),
        timeout_secs: 5,
        max_retries: 0,
        rate_limit_per_minute: 100,
        circuit_breaker: CircuitBreakerConfig {
            failure_threshold: 1,
            success_threshold: 1,
            timeout_secs: 10,
        },
    });
    config
}

#[tokio::test]
async fn chat_completions_stream_true_returns_real_sse() {
    let mock = MockServer::start().await;
    let upstream_body = concat!(
        "data: {\"id\":\"chatcmpl-route-001\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-route-001\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"real\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-route-001\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" stream\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-route-001\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
        "data: [DONE]\n\n",
    );

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(wm_header("Authorization", "Bearer test-openai-key"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(upstream_body),
        )
        .expect(1)
        .mount(&mock)
        .await;

    let state = AppState::new(openai_only_config(format!("{}/v1", mock.uri())))
        .await
        .unwrap();
    let app = Router::new()
        .route("/v1/chat/completions", post(routes::chat::chat_completions))
        .with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/chat/completions")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "model": "openai/gpt-test",
                        "messages": [{"role": "user", "content": "Hello"}],
                        "stream": true
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("text/event-stream")
    );

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("data:"));
    assert!(body.contains("real"));
    assert!(body.contains("stream"));
    assert!(body.contains("[DONE]"));
    assert!(!body.contains("mock stream"));
}
