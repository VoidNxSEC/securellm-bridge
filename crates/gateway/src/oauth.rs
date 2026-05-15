use axum::{
    extract::{Form, Host, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
    Json,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;
use uuid::Uuid;

const CODE_TTL: Duration = Duration::from_secs(120);
const TOKEN_TTL: Duration = Duration::from_secs(86_400 * 7);

struct AuthCode {
    code_challenge: String,
    redirect_uri: String,
    client_id: String,
    issued_at: Instant,
}

struct IssuedToken {
    issued_at: Instant,
}

#[derive(Default)]
pub struct OAuthStore {
    codes: Mutex<HashMap<String, AuthCode>>,
    tokens: Mutex<HashMap<String, IssuedToken>>,
}

impl OAuthStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub async fn is_valid_token(&self, token: &str) -> bool {
        let tokens = self.tokens.lock().await;
        tokens
            .get(token)
            .map_or(false, |t| t.issued_at.elapsed() < TOKEN_TTL)
    }

    async fn store_code(&self, code: String, entry: AuthCode) {
        let mut codes = self.codes.lock().await;
        codes.retain(|_, v| v.issued_at.elapsed() < CODE_TTL);
        codes.insert(code, entry);
    }

    async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
        client_id: &str,
    ) -> Option<String> {
        let mut codes = self.codes.lock().await;
        let entry = codes.remove(code)?;

        if entry.issued_at.elapsed() >= CODE_TTL {
            return None;
        }
        if entry.redirect_uri != redirect_uri || entry.client_id != client_id {
            return None;
        }

        // PKCE S256: BASE64URL(SHA256(code_verifier)) == code_challenge
        let mut hasher = Sha256::new();
        hasher.update(code_verifier.as_bytes());
        let computed = URL_SAFE_NO_PAD.encode(hasher.finalize());
        if computed != entry.code_challenge {
            return None;
        }

        let token = Uuid::new_v4().simple().to_string();
        Some(token)
    }

    async fn issue_token(&self, token: String) {
        let mut tokens = self.tokens.lock().await;
        tokens.retain(|_, v| v.issued_at.elapsed() < TOKEN_TTL);
        tokens.insert(
            token,
            IssuedToken {
                issued_at: Instant::now(),
            },
        );
    }
}

// ── Handlers ────────────────────────────────────────────────────────────────

/// RFC 8414 — OAuth Authorization Server Metadata
pub async fn metadata(Host(host): Host) -> Json<serde_json::Value> {
    let base = format!("https://{host}");
    Json(serde_json::json!({
        "issuer": base,
        "authorization_endpoint": format!("{base}/authorize"),
        "token_endpoint": format!("{base}/token"),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code"],
        "code_challenge_methods_supported": ["S256"],
    }))
}

/// RFC 9728 — Protected Resource Metadata (points MCP clients to the OAuth server)
pub async fn protected_resource_metadata(Host(host): Host) -> Json<serde_json::Value> {
    let base = format!("https://{host}");
    Json(serde_json::json!({
        "resource": format!("{base}/mcp"),
        "authorization_servers": [base],
    }))
}

#[derive(Deserialize)]
pub struct AuthorizeQuery {
    response_type: String,
    client_id: String,
    redirect_uri: String,
    state: Option<String>,
    code_challenge: String,
    code_challenge_method: Option<String>,
}

pub async fn authorize_page(Query(p): Query<AuthorizeQuery>) -> impl IntoResponse {
    if p.response_type != "code" {
        return (
            StatusCode::BAD_REQUEST,
            Html("<h1>Unsupported response_type</h1>".to_string()),
        )
            .into_response();
    }
    if p.code_challenge_method.as_deref().unwrap_or("") != "S256" {
        return (
            StatusCode::BAD_REQUEST,
            Html("<h1>Only S256 code_challenge_method is supported</h1>".to_string()),
        )
            .into_response();
    }

    Html(authorize_html(
        &he(&p.client_id),
        &he(&p.redirect_uri),
        &he(p.state.as_deref().unwrap_or("")),
        &he(&p.code_challenge),
    ))
    .into_response()
}

#[derive(Deserialize)]
pub struct AuthorizeForm {
    client_id: String,
    redirect_uri: String,
    state: String,
    code_challenge: String,
}

pub async fn authorize_submit(
    State(store): State<Arc<OAuthStore>>,
    Form(f): Form<AuthorizeForm>,
) -> impl IntoResponse {
    let code = Uuid::new_v4().simple().to_string();
    let state_clone = f.state.clone();
    let redirect_uri_clone = f.redirect_uri.clone();

    store
        .store_code(
            code.clone(),
            AuthCode {
                code_challenge: f.code_challenge,
                redirect_uri: f.redirect_uri,
                client_id: f.client_id,
                issued_at: Instant::now(),
            },
        )
        .await;

    let location = if state_clone.is_empty() {
        format!("{}?code={}", redirect_uri_clone, code)
    } else {
        format!("{}?code={}&state={}", redirect_uri_clone, code, state_clone)
    };

    Redirect::to(&location)
}

#[derive(Deserialize)]
pub struct TokenRequest {
    grant_type: String,
    code: String,
    redirect_uri: String,
    client_id: String,
    code_verifier: String,
}

#[derive(Serialize)]
struct TokenResponse {
    access_token: String,
    token_type: &'static str,
    expires_in: u64,
}

pub async fn token(
    State(store): State<Arc<OAuthStore>>,
    Form(f): Form<TokenRequest>,
) -> impl IntoResponse {
    if f.grant_type != "authorization_code" {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "unsupported_grant_type"})),
        )
            .into_response();
    }

    let Some(access_token) = store
        .exchange_code(&f.code, &f.code_verifier, &f.redirect_uri, &f.client_id)
        .await
    else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid_grant"})),
        )
            .into_response();
    };

    store.issue_token(access_token.clone()).await;

    Json(TokenResponse {
        access_token,
        token_type: "Bearer",
        expires_in: TOKEN_TTL.as_secs(),
    })
    .into_response()
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn he(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn authorize_html(
    client_id: &str,
    redirect_uri: &str,
    state: &str,
    code_challenge: &str,
) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>SecureLLM Gateway — Authorize</title>
<style>
  *{{box-sizing:border-box;margin:0;padding:0}}
  body{{font-family:system-ui,sans-serif;background:#0d0d0d;color:#e2e2e2;display:flex;align-items:center;justify-content:center;min-height:100vh;padding:1rem}}
  .card{{background:#161616;border:1px solid #2a2a2a;border-radius:12px;padding:2rem;max-width:440px;width:100%}}
  h1{{font-size:1.1rem;font-weight:600;margin-bottom:.25rem}}
  .sub{{color:#666;font-size:.85rem;margin-bottom:1.5rem}}
  .box{{background:#1e1e1e;border:1px solid #2a2a2a;border-radius:8px;padding:1rem;margin-bottom:1.5rem;font-size:.9rem;line-height:1.6}}
  .label{{color:#666;font-size:.75rem;text-transform:uppercase;letter-spacing:.05em;margin-bottom:.25rem}}
  .value{{color:#a8d8a8;word-break:break-all}}
  button{{width:100%;padding:.75rem;background:#2563eb;color:#fff;border:none;border-radius:8px;font-size:1rem;cursor:pointer;font-weight:500}}
  button:hover{{background:#1d4ed8}}
</style>
</head>
<body>
<div class="card">
  <h1>SecureLLM Gateway</h1>
  <p class="sub">An application is requesting access</p>
  <div class="box">
    <div class="label">Client</div>
    <div class="value">{client_id}</div>
  </div>
  <p style="font-size:.85rem;color:#888;margin-bottom:1.5rem">
    If authorized, this client can push branches and open pull requests on your allowlisted repositories.
  </p>
  <form method="post">
    <input type="hidden" name="client_id" value="{client_id}">
    <input type="hidden" name="redirect_uri" value="{redirect_uri}">
    <input type="hidden" name="state" value="{state}">
    <input type="hidden" name="code_challenge" value="{code_challenge}">
    <button type="submit">Authorize</button>
  </form>
</div>
</body>
</html>"#
    )
}
