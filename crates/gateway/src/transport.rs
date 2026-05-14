use crate::GatewayHandler;
use anyhow::{Context, Result};
use rmcp::{
    transport::{
        stdio,
        streamable_http_server::{
            session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
        },
    },
    ServiceExt,
};
use std::net::SocketAddr;
use tracing::{error, info};

pub type GatewayHttpService = StreamableHttpService<GatewayHandler, LocalSessionManager>;

pub fn streamable_http_service(
    handler: GatewayHandler,
    config: StreamableHttpServerConfig,
) -> GatewayHttpService {
    StreamableHttpService::new(move || Ok(handler.clone()), Default::default(), config)
}

pub fn http_router(handler: GatewayHandler, config: StreamableHttpServerConfig) -> axum::Router {
    axum::Router::new().nest_service("/mcp", streamable_http_service(handler, config))
}

pub async fn serve_stdio(handler: GatewayHandler) -> Result<()> {
    let service = handler
        .serve(stdio())
        .await
        .inspect_err(|e| error!(error = ?e, "rmcp stdio serve failed"))?;

    service.waiting().await?;
    Ok(())
}

pub async fn serve_http(handler: GatewayHandler, listen_addr: SocketAddr) -> Result<()> {
    let router = http_router(handler, StreamableHttpServerConfig::default());
    let listener = tokio::net::TcpListener::bind(listen_addr)
        .await
        .with_context(|| format!("binding gateway HTTP transport on {listen_addr}"))?;
    let bound_addr = listener
        .local_addr()
        .context("reading bound HTTP address")?;

    info!(
        listen_addr = %bound_addr,
        endpoint = %format!("http://{bound_addr}/mcp"),
        "gateway HTTP transport listening"
    );

    axum::serve(listener, router)
        .await
        .context("serving gateway HTTP transport")?;
    Ok(())
}
