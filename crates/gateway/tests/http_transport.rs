use secrecy::SecretString;
use securellm_gateway::{
    audit::JsonlSink,
    config::{GatewayConfig, GatewayTransport, RepoSlug},
    transport, GatewayContext, GatewayHandler,
};
use serde_json::json;
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
    }
}

async fn test_handler() -> (TempDir, GatewayHandler) {
    let tmp = TempDir::new().unwrap();
    let config = Arc::new(test_config(tmp.path().to_path_buf()));
    let audit = JsonlSink::open(tmp.path()).await.unwrap();
    let ctx = GatewayContext::new(config, audit);
    (tmp, GatewayHandler::new(ctx))
}

#[tokio::test(flavor = "multi_thread")]
async fn http_transport_serves_initialize_on_mcp_path() -> anyhow::Result<()> {
    let (_tmp, handler) = test_handler().await;
    let rmcp_config = rmcp::transport::StreamableHttpServerConfig::default()
        .with_stateful_mode(false)
        .with_json_response(true)
        .with_sse_keep_alive(None);
    let router = transport::http_router(handler, rmcp_config);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });

    let response = reqwest::Client::new()
        .post(format!("http://{addr}/mcp"))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {
                    "name": "gateway-http-test",
                    "version": "1.0"
                }
            }
        }))
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
