use crate::{auth, oauth, oauth::OAuthStore, rate_limit, GatewayHandler};
use anyhow::{Context, Result};
use axum::{middleware, routing::get};
use rmcp::{
    transport::{
        stdio,
        streamable_http_server::{
            session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
        },
    },
    ServiceExt,
};
use secrecy::SecretString;
use std::{net::SocketAddr, sync::Arc};
use tracing::{error, info};

pub type GatewayHttpService = StreamableHttpService<GatewayHandler, LocalSessionManager>;

pub fn streamable_http_service(
    handler: GatewayHandler,
    config: StreamableHttpServerConfig,
) -> GatewayHttpService {
    StreamableHttpService::new(move || Ok(handler.clone()), Default::default(), config)
}

pub fn http_router(
    handler: GatewayHandler,
    config: StreamableHttpServerConfig,
    bearer_token: Option<SecretString>,
    oauth_store: Arc<OAuthStore>,
    rate_limit: rate_limit::RateLimitState,
) -> axum::Router {
    // /mcp is protected by bearer or OAuth token.
    let mcp = axum::Router::new()
        .nest_service("/mcp", streamable_http_service(handler, config))
        .layer(middleware::from_fn_with_state(
            rate_limit,
            rate_limit::enforce,
        ))
        .layer(middleware::from_fn_with_state(
            (bearer_token, Arc::clone(&oauth_store)),
            auth::bearer_or_oauth_auth,
        ));

    // OAuth endpoints are public; security comes from PKCE plus the consent page.
    let oauth = axum::Router::new()
        .route(
            "/.well-known/oauth-authorization-server",
            get(oauth::metadata),
        )
        .route(
            "/.well-known/oauth-protected-resource",
            get(oauth::protected_resource_metadata),
        )
        .route(
            "/authorize",
            get(oauth::authorize_page).post(oauth::authorize_submit),
        )
        .route("/token", axum::routing::post(oauth::token))
        .with_state(oauth_store);

    axum::Router::new().merge(mcp).merge(oauth)
}

pub async fn serve_stdio(handler: GatewayHandler) -> Result<()> {
    let service = handler
        .serve(stdio())
        .await
        .inspect_err(|e| error!(error = ?e, "rmcp stdio serve failed"))?;

    service.waiting().await?;
    Ok(())
}

pub async fn serve_http(
    handler: GatewayHandler,
    listen_addr: SocketAddr,
    bearer_token: Option<SecretString>,
    rate_limit: rate_limit::RateLimitState,
) -> Result<()> {
    let oauth_store = OAuthStore::new();
    let router = http_router(
        handler,
        StreamableHttpServerConfig::default(),
        bearer_token,
        oauth_store,
        rate_limit,
    );
    let listener = tokio::net::TcpListener::bind(listen_addr)
        .await
        .with_context(|| format!("binding gateway HTTP transport on {listen_addr}"))?;
    let bound_addr = listener
        .local_addr()
        .context("reading bound HTTP address")?;

    info!(
        listen_addr = %bound_addr,
        endpoint = %format!("http://{bound_addr}/mcp"),
        oauth_metadata = %format!("http://{bound_addr}/.well-known/oauth-authorization-server"),
        "gateway HTTP transport listening"
    );

    axum::serve(listener, router)
        .await
        .context("serving gateway HTTP transport")?;
    Ok(())
}
