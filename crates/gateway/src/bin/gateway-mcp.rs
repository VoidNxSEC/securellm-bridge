use anyhow::{Context, Result};
use securellm_gateway::{
    audit::{AuditEvent, JsonlSink},
    transport as gateway_transport, GatewayConfig, GatewayContext, GatewayHandler,
    GatewayTransport,
};
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let config = GatewayConfig::from_env().context("loading gateway config from env")?;
    info!(?config, "gateway config loaded");
    let transport = config.transport;
    let listen_addr = config.listen_addr;

    let audit = JsonlSink::open(&config.log_dir)
        .await
        .context("opening audit sink")?;

    let boot_event = AuditEvent::new(&config.agent_id, "gateway_started");
    audit
        .emit(&boot_event.ok(serde_json::json!({
            "allowlist_size": config.allowlist.len(),
            "log_dir": config.log_dir.display().to_string(),
            "transport": transport.to_string(),
            "listen_addr": listen_addr.to_string(),
        })))
        .await
        .context("emitting boot audit event")?;
    info!(path = ?audit.path(), "audit sink ready");

    let ctx = GatewayContext::new(Arc::new(config), audit);
    let handler = GatewayHandler::new(ctx);

    match transport {
        GatewayTransport::Stdio => gateway_transport::serve_stdio(handler).await,
        GatewayTransport::Http => gateway_transport::serve_http(handler, listen_addr).await,
    }
}
