use crate::oauth::OAuthStore;
use axum::{
    extract::{Request, State},
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use secrecy::{ExposeSecret, SecretString};
use std::sync::Arc;
use tracing::warn;

/// Accepts either a static bearer token (dev/fallback) or an OAuth-issued token.
/// Returns 401 with WWW-Authenticate header so MCP clients can discover the OAuth server.
pub async fn bearer_or_oauth_auth(
    State((static_token, oauth_store)): State<(Option<SecretString>, Arc<OAuthStore>)>,
    req: Request,
    next: Next,
) -> Response {
    let host = req
        .headers()
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("localhost")
        .to_string();

    let provided = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::trim);

    if let Some(token) = provided {
        if let Some(expected) = &static_token {
            if token == expected.expose_secret().as_str() {
                return next.run(req).await;
            }
        }
        if oauth_store.is_valid_token(token).await {
            return next.run(req).await;
        }
        warn!("gateway auth rejected: invalid token");
    } else {
        warn!("gateway auth rejected: missing Authorization header");
    }

    unauthorized_response(&host)
}

fn unauthorized_response(host: &str) -> Response {
    let resource_metadata = format!("https://{}/.well-known/oauth-protected-resource", host);
    let www_auth = format!("Bearer resource_metadata=\"{}\"", resource_metadata);
    let mut resp = StatusCode::UNAUTHORIZED.into_response();
    if let Ok(val) = HeaderValue::from_str(&www_auth) {
        resp.headers_mut().insert(header::WWW_AUTHENTICATE, val);
    }
    resp
}
