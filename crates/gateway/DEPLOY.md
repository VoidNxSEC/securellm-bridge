# SecureLLM Gateway Deploy

This guide covers the HTTP MCP gateway in `crates/gateway`.

## Runtime Shape

The gateway exposes:

- `POST /mcp`: MCP streamable HTTP endpoint.
- `GET /.well-known/oauth-authorization-server`: OAuth metadata.
- `GET /.well-known/oauth-protected-resource`: protected-resource metadata for MCP clients.
- `GET /authorize`: OAuth consent page.
- `POST /authorize`: local consent submission.
- `POST /token`: OAuth authorization-code token exchange.

`/mcp` accepts either:

- A static fallback bearer token from `GATEWAY_BEARER_TOKEN`.
- An OAuth/PKCE access token issued by this gateway.

The GitHub PAT never leaves the server. It is only used inside server-side handlers.

## Required Env

```bash
export GATEWAY_GITHUB_PAT='ghp_...'
export GATEWAY_REPO_ALLOWLIST='owner/repo,owner/another-repo'
export GATEWAY_AGENT_ID='claude-web-agent-01'
export GATEWAY_TRANSPORT='http'
```

Optional env:

```bash
export GATEWAY_LISTEN_ADDR='127.0.0.1:8765'
export GATEWAY_LOG_DIR="$HOME/.local/share/voidnx-gateway"
export GATEWAY_BEARER_TOKEN='replace-with-long-random-token'
export GATEWAY_RATE_LIMIT_PER_MINUTE='10'
```

If `GATEWAY_BEARER_TOKEN` is unset, the static bearer fallback is disabled and HTTP clients must use OAuth/PKCE.

For production, prefer a file-backed GitHub PAT instead of exporting the PAT in the process environment:

```bash
unset GATEWAY_GITHUB_PAT
export GATEWAY_GITHUB_PAT_FILE='/run/secrets/gateway_github_pat'
```

If neither `GATEWAY_GITHUB_PAT` nor `GATEWAY_GITHUB_PAT_FILE` is set, the gateway reads `/run/secrets/gateway_github_pat`. The file may contain a trailing newline; it is trimmed before use.

## Run Locally

```bash
cargo run -p securellm-gateway --bin gateway-mcp
```

The server logs the bound `/mcp` endpoint and OAuth metadata endpoint on startup.

For production, expose the gateway behind TLS. OAuth metadata currently advertises `https://{Host}`, so the public Host header should match the TLS endpoint clients use.

## Static Bearer Smoke Test

```bash
curl -fsS \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -H "Authorization: Bearer $GATEWAY_BEARER_TOKEN" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' \
  http://127.0.0.1:8765/mcp
```

Expected result: JSON-RPC response listing:

- `gateway_push_branch`
- `gateway_create_pr`
- `gateway_comment_pr`

## OAuth/PKCE Flow

1. Client creates a high-entropy `code_verifier`.
2. Client computes `code_challenge = BASE64URL(SHA256(code_verifier))`.
3. Client opens:

```text
https://gateway.example.com/authorize?response_type=code&client_id=<client>&redirect_uri=<callback>&state=<state>&code_challenge=<challenge>&code_challenge_method=S256
```

4. User authorizes the client on the consent page.
5. Gateway redirects to:

```text
<callback>?code=<authorization-code>&state=<state>
```

6. Client exchanges the code:

```bash
curl -fsS \
  -H 'Content-Type: application/x-www-form-urlencoded' \
  -d "grant_type=authorization_code" \
  -d "code=$CODE" \
  -d "redirect_uri=$REDIRECT_URI" \
  -d "client_id=$CLIENT_ID" \
  -d "code_verifier=$CODE_VERIFIER" \
  https://gateway.example.com/token
```

7. Client calls `/mcp` with:

```text
Authorization: Bearer <access_token>
```

Tokens are in-memory today. Restarting the gateway invalidates issued OAuth tokens.

## Rate Limit

`GATEWAY_RATE_LIMIT_PER_MINUTE` controls the per-agent HTTP request quota. Default: `10`.

When exceeded, `/mcp` returns:

- HTTP `429 Too Many Requests`
- `Retry-After: 60`
- Audit event with `outcome = "rate_limited"`

## Audit

Audit events are written to:

```text
$GATEWAY_LOG_DIR/events.jsonl
```

The audit log records boot, tool outcomes, rejection/failure details, patch SHA-256 values, and rate-limit events. It must not contain GitHub PATs or bearer tokens.

## Validation

```bash
cargo test -p securellm-gateway
cargo test -p securellm-gateway --test http_transport
```

The HTTP test covers:

- `/mcp` initialize over HTTP.
- Static bearer accept/reject.
- OAuth/PKCE authorization-code flow into `tools/list`.
- Rate-limit `429` plus `rate_limited` audit.

## NixOS Service

The flake exports `nixosModules.gateway-service`. The module creates `securellm-gateway.service`, loads the PAT with systemd `LoadCredential=`, and passes the credential path through `GATEWAY_GITHUB_PAT_FILE`.

Example:

```nix
{
  imports = [
    inputs.securellm-bridge.nixosModules.gateway-service
  ];

  services.securellm-bridge.gateway = {
    enable = true;
    package = inputs.securellm-bridge.packages.${pkgs.system}.gateway;
    repoAllowlist = [ "owner/repo" ];
    agentId = "claude-web-agent-01";
    listenAddr = "127.0.0.1:8765";
    githubPatFile = "/run/secrets/gateway_github_pat";
    rateLimitPerMinute = 10;
  };
}
```

`packages.${pkgs.system}.gateway` is the intended dedicated package target for the next operational step. Until that package exists, set `package` to any derivation that provides `bin/gateway-mcp`.
