use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use secrecy::SecretString;
use securellm_gateway::{
    audit::JsonlSink,
    config::{GatewayConfig, GatewayTransport, RepoSlug},
    oauth::OAuthStore,
    rate_limit::RateLimitState,
    transport, GatewayContext, GatewayHandler,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tempfile::TempDir;

fn test_config(log_dir: std::path::PathBuf) -> GatewayConfig {
    GatewayConfig {
        pat: SecretString::new("ghp_test_token_not_real_value".into()),
        allowlist: vec![RepoSlug::parse("acme/widgets").unwrap()],
        agent_id: "http-test-agent".into(),
        log_dir,
        transport: GatewayTransport::Http,
        listen_addr: "127.0.0.1:0".parse().unwrap(),
        bearer_token: None,
        rate_limit_per_minute: std::num::NonZeroU32::new(10).unwrap(),
    }
}

async fn spawn_server(
    bearer_token: Option<SecretString>,
    rate_limit_per_minute: u32,
) -> (TempDir, std::net::SocketAddr, tokio::task::JoinHandle<()>) {
    let tmp = TempDir::new().unwrap();
    let mut config = test_config(tmp.path().to_path_buf());
    config.rate_limit_per_minute = std::num::NonZeroU32::new(rate_limit_per_minute).unwrap();
    let config = Arc::new(config);
    let audit = JsonlSink::open(tmp.path()).await.unwrap();
    let rate_limit = RateLimitState::new(
        config.agent_id.clone(),
        audit.clone(),
        config.rate_limit_per_minute,
    );
    let ctx = GatewayContext::new(config, audit);
    let handler = GatewayHandler::new(ctx);

    let rmcp_config = rmcp::transport::StreamableHttpServerConfig::default()
        .with_stateful_mode(false)
        .with_json_response(true)
        .with_sse_keep_alive(None);
    let router = transport::http_router(
        handler,
        rmcp_config,
        bearer_token,
        OAuthStore::new(),
        rate_limit,
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });

    (tmp, addr, server)
}

fn initialize_body() -> serde_json::Value {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-03-26",
            "capabilities": {},
            "clientInfo": { "name": "gateway-http-test", "version": "1.0" }
        }
    })
}

fn tools_list_body() -> serde_json::Value {
    json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    })
}

fn pkce_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

fn code_from_location(location: &str) -> Option<&str> {
    location
        .split('?')
        .nth(1)?
        .split('&')
        .find_map(|pair| pair.strip_prefix("code="))
}

#[tokio::test(flavor = "multi_thread")]
async fn http_transport_serves_initialize_on_mcp_path() -> anyhow::Result<()> {
    let token = SecretString::new("transport-test-token".into());
    let (_tmp, addr, server) = spawn_server(Some(token), 10).await;

    let response = reqwest::Client::new()
        .post(format!("http://{addr}/mcp"))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("Authorization", "Bearer transport-test-token")
        .json(&initialize_body())
        .send()
        .await?;

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body: serde_json::Value = response.json().await?;
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["id"], 1);
    assert!(body["result"]["capabilities"]["tools"].is_object());

    server.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn oauth_pkce_token_can_call_tools_list() -> anyhow::Result<()> {
    let (_tmp, addr, server) = spawn_server(None, 10).await;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let verifier = "abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let challenge = pkce_challenge(verifier);
    let client_id = "gateway-http-oauth-test";
    let redirect_uri = "http://client.example/callback";

    let consent_page = client
        .get(format!("http://{addr}/authorize"))
        .query(&[
            ("response_type", "code"),
            ("client_id", client_id),
            ("redirect_uri", redirect_uri),
            ("state", "state-123"),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
        ])
        .send()
        .await?;
    assert_eq!(consent_page.status(), reqwest::StatusCode::OK);
    assert!(consent_page.text().await?.contains(client_id));

    let authorize = client
        .post(format!("http://{addr}/authorize"))
        .form(&[
            ("client_id", client_id),
            ("redirect_uri", redirect_uri),
            ("state", "state-123"),
            ("code_challenge", challenge.as_str()),
        ])
        .send()
        .await?;

    assert_eq!(authorize.status(), reqwest::StatusCode::SEE_OTHER);
    let location = authorize
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .expect("authorize response must redirect with a Location header");
    assert!(location.starts_with(redirect_uri));
    assert!(location.contains("state=state-123"));
    let code = code_from_location(location).expect("redirect must include code");

    let token_response = client
        .post(format!("http://{addr}/token"))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", client_id),
            ("code_verifier", verifier),
        ])
        .send()
        .await?;

    assert_eq!(token_response.status(), reqwest::StatusCode::OK);
    let token_body: serde_json::Value = token_response.json().await?;
    assert_eq!(token_body["token_type"], "Bearer");
    let access_token = token_body["access_token"]
        .as_str()
        .expect("token response must include access_token");

    let tools_response = client
        .post(format!("http://{addr}/mcp"))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .bearer_auth(access_token)
        .json(&tools_list_body())
        .send()
        .await?;

    assert_eq!(tools_response.status(), reqwest::StatusCode::OK);
    let tools_body: serde_json::Value = tools_response.json().await?;
    let tools = tools_body["result"]["tools"]
        .as_array()
        .expect("tools/list should return an array of tools");
    let names: Vec<&str> = tools
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    assert!(names.contains(&"gateway_push_branch"));
    assert!(names.contains(&"gateway_create_pr"));
    assert!(names.contains(&"gateway_comment_pr"));

    server.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn bearer_auth_accepts_correct_token() -> anyhow::Result<()> {
    let token = SecretString::new("test-secret-token".into());
    let (_tmp, addr, server) = spawn_server(Some(token), 10).await;

    let response = reqwest::Client::new()
        .post(format!("http://{addr}/mcp"))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("Authorization", "Bearer test-secret-token")
        .json(&initialize_body())
        .send()
        .await?;

    assert_eq!(response.status(), reqwest::StatusCode::OK);

    server.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn bearer_auth_rejects_missing_header() -> anyhow::Result<()> {
    let token = SecretString::new("test-secret-token".into());
    let (_tmp, addr, server) = spawn_server(Some(token), 10).await;

    let response = reqwest::Client::new()
        .post(format!("http://{addr}/mcp"))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .json(&initialize_body())
        .send()
        .await?;

    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);

    server.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn bearer_auth_rejects_wrong_token() -> anyhow::Result<()> {
    let token = SecretString::new("test-secret-token".into());
    let (_tmp, addr, server) = spawn_server(Some(token), 10).await;

    let response = reqwest::Client::new()
        .post(format!("http://{addr}/mcp"))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("Authorization", "Bearer wrong-token")
        .json(&initialize_body())
        .send()
        .await?;

    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);

    server.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn rate_limit_rejects_after_configured_burst_and_audits() -> anyhow::Result<()> {
    let token = SecretString::new("test-secret-token".into());
    let (tmp, addr, server) = spawn_server(Some(token), 1).await;
    let client = reqwest::Client::new();

    let first = client
        .post(format!("http://{addr}/mcp"))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("Authorization", "Bearer test-secret-token")
        .json(&initialize_body())
        .send()
        .await?;
    assert_eq!(first.status(), reqwest::StatusCode::OK);

    let second = client
        .post(format!("http://{addr}/mcp"))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("Authorization", "Bearer test-secret-token")
        .json(&initialize_body())
        .send()
        .await?;
    assert_eq!(second.status(), reqwest::StatusCode::TOO_MANY_REQUESTS);

    let raw = tokio::fs::read_to_string(tmp.path().join("events.jsonl")).await?;
    assert!(raw.contains(r#""outcome":"rate_limited""#));
    assert!(raw.contains(r#""tool":"gateway_http_request""#));
    assert!(!raw.contains("test-secret-token"));

    server.abort();
    Ok(())
}
