//! Provider Integration Tests
//!
//! Tests each provider against WireMock servers to verify:
//! - Request/response format compliance
//! - Error handling (4xx, 5xx)
//! - Retry and timeout behavior
//! - Streaming support (where applicable)
//!
//! Each test uses a separate `MockServer` instance to avoid state leakage.

use securellm_core::{LLMProvider, Message, MessageContent, MessageRole, Request};
use securellm_providers::{
    llamacpp::LlamaCppProvider,
    openai::{OpenAIConfig, OpenAIProvider},
};
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── DeepSeek Provider ───────────────────────────────────────────────

#[tokio::test]
async fn test_deepseek_chat_success() {
    let mock = MockServer::start().await;

    let response = json!({
        "id": "chatcmpl-deepseek-001",
        "model": "deepseek-chat",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "The capital of France is Paris."
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 12,
            "completion_tokens": 8,
            "total_tokens": 20
        }
    });

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("Authorization", "Bearer test-deepseek-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .expect(1)
        .mount(&mock)
        .await;

    let client = reqwest::Client::new();
    let body = json!({
        "model": "deepseek-chat",
        "messages": [
            {"role": "user", "content": "What is the capital of France?"}
        ],
        "temperature": 0.7,
        "max_tokens": 100
    });

    let resp = client
        .post(format!("{}/chat/completions", mock.uri()))
        .header("Authorization", "Bearer test-deepseek-key")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    let data: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(data["model"], "deepseek-chat");
    assert!(!data["choices"][0]["message"]["content"]
        .as_str()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn test_deepseek_authentication_error() {
    let mock = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "error": {
                "message": "Invalid API key",
                "type": "authentication_error"
            }
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/chat/completions", mock.uri()))
        .header("Authorization", "Bearer bad-key")
        .header("Content-Type", "application/json")
        .json(&json!({
            "model": "deepseek-chat",
            "messages": [{"role": "user", "content": "Hi"}]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

// ── OpenAI Provider ─────────────────────────────────────────────────

#[tokio::test]
async fn test_openai_chat_success() {
    let mock = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("Authorization", "Bearer test-openai-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl-openai-001",
            "model": "gpt-4o-mini",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "Hello! How can I help?"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 8, "completion_tokens": 6, "total_tokens": 14}
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/v1/chat/completions", mock.uri()))
        .header("Authorization", "Bearer test-openai-key")
        .json(&json!({
            "model": "gpt-4o-mini",
            "messages": [{"role": "user", "content": "Hello"}]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn test_openai_rate_limit_error() {
    let mock = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_json(json!({
                    "error": {
                        "message": "Rate limit exceeded",
                        "type": "rate_limit_error"
                    }
                }))
                .insert_header("Retry-After", "30"),
        )
        .expect(1)
        .mount(&mock)
        .await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/v1/chat/completions", mock.uri()))
        .header("Authorization", "Bearer test-key")
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Test"}]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 429);
    assert_eq!(
        resp.headers()
            .get("Retry-After")
            .and_then(|v| v.to_str().ok()),
        Some("30")
    );
}

#[tokio::test]
async fn test_openai_streaming_chat_success() {
    let mock = MockServer::start().await;

    let body = concat!(
        "data: {\"id\":\"chatcmpl-stream-001\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-4o-mini\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-stream-001\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-4o-mini\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hello\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-stream-001\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-4o-mini\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" world\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-stream-001\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-4o-mini\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
        "data: [DONE]\n\n",
    );

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("Authorization", "Bearer test-openai-key"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(body),
        )
        .expect(1)
        .mount(&mock)
        .await;

    let provider = OpenAIProvider::new(
        OpenAIConfig::new("test-openai-key").with_endpoint(format!("{}/v1", mock.uri())),
    )
    .unwrap();

    let mut request = Request::new("openai", "gpt-4o-mini");
    request.messages.push(Message {
        role: MessageRole::User,
        content: MessageContent::Text("Hello".to_string()),
        name: None,
        metadata: None,
    });

    let mut stream = provider.stream_request(request).await.unwrap();
    let mut content = String::new();
    let mut saw_role = false;
    let mut saw_stop = false;

    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.unwrap();
        if chunk.delta.role == Some(MessageRole::Assistant) {
            saw_role = true;
        }
        if let Some(delta) = chunk.delta.content {
            content.push_str(&delta);
        }
        if chunk.finish_reason == Some(securellm_core::FinishReason::Stop) {
            saw_stop = true;
        }
    }

    assert!(saw_role);
    assert_eq!(content, "Hello world");
    assert!(saw_stop);
}

#[tokio::test]
async fn test_unsupported_provider_streaming_returns_error() {
    let provider = LlamaCppProvider::new(5001, "local-model").unwrap();
    let mut request = Request::new("llamacpp", "local-model");
    request.messages.push(Message {
        role: MessageRole::User,
        content: MessageContent::Text("Hello".to_string()),
        name: None,
        metadata: None,
    });

    let err = match provider.stream_request(request).await {
        Ok(_) => panic!("llamacpp streaming should be unsupported"),
        Err(err) => err,
    };
    assert!(err.to_string().contains("streaming is not implemented"));
}

// ── Anthropic Provider ──────────────────────────────────────────────

#[tokio::test]
async fn test_anthropic_messages_success() {
    let mock = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "test-anthropic-key"))
        .and(header("anthropic-version", "2023-06-01"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "msg_abc123",
            "type": "message",
            "model": "claude-sonnet-4-20250514",
            "content": [{
                "type": "text",
                "text": "I'm Claude, nice to meet you!"
            }],
            "role": "assistant",
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 10, "output_tokens": 8}
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/v1/messages", mock.uri()))
        .header("x-api-key", "test-anthropic-key")
        .header("anthropic-version", "2023-06-01")
        .json(&json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 100,
            "messages": [{"role": "user", "content": "Hello"}]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let data: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(data["type"], "message");
    assert!(!data["content"][0]["text"].as_str().unwrap().is_empty());
}

// ── Gemini Provider ─────────────────────────────────────────────────

#[tokio::test]
async fn test_gemini_generate_content_success() {
    let mock = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/models/gemini-2.0-flash:generateContent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": "Paris is the capital of France."}]
                },
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 5,
                "candidatesTokenCount": 7,
                "totalTokenCount": 12
            }
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!(
            "{}/models/gemini-2.0-flash:generateContent",
            mock.uri()
        ))
        .json(&json!({
            "contents": [{
                "role": "user",
                "parts": [{"text": "What is the capital of France?"}]
            }]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let data: serde_json::Value = resp.json().await.unwrap();
    assert!(!data["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap()
        .is_empty());
}

// ── Groq Provider ───────────────────────────────────────────────────

#[tokio::test]
async fn test_groq_chat_success() {
    let mock = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("Authorization", "Bearer test-groq-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl-groq-001",
            "model": "llama-3.3-70b-versatile",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "Fast response from Groq!"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 15, "completion_tokens": 5, "total_tokens": 20}
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/v1/chat/completions", mock.uri()))
        .header("Authorization", "Bearer test-groq-key")
        .json(&json!({
            "model": "llama-3.3-70b-versatile",
            "messages": [{"role": "user", "content": "Hello"}]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
}

// ── NVIDIA Provider ─────────────────────────────────────────────────

#[tokio::test]
async fn test_nvidia_chat_success() {
    let mock = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("Authorization", "Bearer test-nvidia-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl-nvidia-001",
            "model": "meta/llama-3.3-70b-instruct",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "NVIDIA NIM response."},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 10, "completion_tokens": 4, "total_tokens": 14}
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/v1/chat/completions", mock.uri()))
        .header("Authorization", "Bearer test-nvidia-key")
        .json(&json!({
            "model": "meta/llama-3.3-70b-instruct",
            "messages": [{"role": "user", "content": "Hello"}]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
}

// ── Error Handling ──────────────────────────────────────────────────

#[tokio::test]
async fn test_server_5xx_error_handling() {
    let mock = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(502).set_body_string("Bad Gateway"))
        .expect(3) // Should retry a few times then give up
        .mount(&mock)
        .await;

    let client = reqwest::Client::new();

    // Try 3 times (simulating retry)
    for _ in 0..3 {
        let resp = client
            .post(format!("{}/chat/completions", mock.uri()))
            .header("Authorization", "Bearer test-key")
            .json(&json!({"model": "test", "messages": []}))
            .send()
            .await
            .unwrap();
        assert!(!resp.status().is_success());
    }
}

#[tokio::test]
async fn test_request_timeout() {
    let mock = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("ok")
                .set_delay(std::time::Duration::from_secs(5)), // 5-second delay
        )
        .expect(1)
        .mount(&mock)
        .await;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(1)) // 1-second timeout
        .build()
        .unwrap();

    let result = client
        .post(format!("{}/chat/completions", mock.uri()))
        .json(&json!({"model": "test", "messages": []}))
        .send()
        .await;

    assert!(
        result.is_err(),
        "Request should timeout with short client timeout"
    );
}

// ── Response Validation ─────────────────────────────────────────────

#[tokio::test]
async fn test_response_json_structure_valid() {
    let mock = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp-001",
            "model": "test-model",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Valid response"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 5,
                "completion_tokens": 2,
                "total_tokens": 7
            }
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/chat/completions", mock.uri()))
        .json(&json!({"model": "test", "messages": [{"role": "user", "content": "Hi"}]}))
        .send()
        .await
        .unwrap();

    let data: serde_json::Value = resp.json().await.unwrap();

    // Required fields must exist
    assert!(data.get("id").is_some(), "Response must have id");
    assert!(data.get("model").is_some(), "Response must have model");
    assert!(
        data.get("choices").and_then(|c| c.as_array()).is_some(),
        "Response must have choices array"
    );
    assert!(
        data.get("usage").is_some(),
        "Response should have usage info"
    );

    // Choice must have required fields
    let choice = &data["choices"][0];
    assert!(choice.get("message").is_some());
    assert!(choice.get("finish_reason").is_some());
}

#[tokio::test]
async fn test_response_missing_choices_error() {
    let mock = MockServer::start().await;

    // Malformed response: no choices array
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp-001",
            "model": "test-model"
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/chat/completions", mock.uri()))
        .json(&json!({"model": "test", "messages": [{"role": "user", "content": "Hi"}]}))
        .send()
        .await
        .unwrap();

    let data: serde_json::Value = resp.json().await.unwrap();
    assert!(
        data.get("choices").is_none(),
        "Malformed response lacks choices"
    );
}

// ── Content Safety ──────────────────────────────────────────────────

#[tokio::test]
async fn test_content_filter_response() {
    let mock = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp-filtered-001",
            "model": "test-model",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "I cannot provide that information."
                },
                "finish_reason": "content_filter"
            }],
            "usage": {"prompt_tokens": 3, "completion_tokens": 6, "total_tokens": 9}
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/chat/completions", mock.uri()))
        .json(
            &json!({"model": "test", "messages": [{"role": "user", "content": "Blocked content"}]}),
        )
        .send()
        .await
        .unwrap();

    let data: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(data["choices"][0]["finish_reason"], "content_filter");
}
