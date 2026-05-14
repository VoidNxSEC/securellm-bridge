# Gateway MCP — Roadmap

> 3 tools MCP (`gateway_push_branch`, `gateway_create_pr`, `gateway_comment_pr`) que permitem agentes Claude Code em sandbox remoto fazer push/PR/comment no GitHub sem ver credenciais. Hospedado no `securellm-bridge` como primeira feature MCP concreta — o bridge se autoanunciava "MCP Server" desde sempre, agora finalmente é.

**Fontes:**
- MISSION: `/home/kernelcore/master/securellm-mcp/MISSION.md`
- Plano da execução inicial: `/home/kernelcore/.claude/plans/ok-ent-o-vamos-fazer-tranquil-castle.md`

---

## Princípios não-negociáveis

1. **PAT zero-leak.** Nunca em log, erro, retorno de tool, stacktrace. `secrecy::SecretString` em memória, `ExposeSecret` só no momento do uso. `Debug` de `GatewayConfig` redacta manualmente.
2. **Rationale obrigatório.** `>= 20 chars` em toda chamada. Sem justificativa, sem ação.
3. **Allowlist fail-closed.** Repo ausente da allowlist → rejeita antes de qualquer chamada externa.
4. **Sem verbos destrutivos.** Force-push, branch delete, repo create estão fora — agora e sempre.
5. **Audit append-only.** JSONL hoje, ledger amanhã, formato estável desde dia 1.

---

## Estado atual

### Onda 0 — Consolidação Kluster → bridge ✅
- Kluster (ancestral parado há 3 meses) arquivado com tag local `archive/pre-bridge-merge`.
- `ARCHIVED.md` no Kluster aponta pro bridge.
- Bridge confirmado como superset (DeepSeek/Anthropic/OpenAI reais vs stubs de 47 LOC).

### Onda 1 — Andaime do crate ✅
- `crates/gateway/` no workspace do bridge.
- Deps: `rmcp = "1.7"` (features `server`/`macros`/`transport-io`), `octocrab = "0.39"`, `base64 = "0.22"`, `tempfile`, `secrecy`, `schemars = "1.2"`, `sha2`.
- Bin `gateway-mcp`: parse env, fail-closed sem PAT, emite `gateway_started` no boot, sobe rmcp via stdio.

### Onda 2 — Validators, audit, handlers ✅
- `GatewayConfig::from_env()` lê `GATEWAY_GITHUB_PAT`, `GATEWAY_REPO_ALLOWLIST`, `GATEWAY_AGENT_ID`, `GATEWAY_LOG_DIR`. Fail-closed em qualquer ausência.
- Validators: rationale ≥20, pr_body ≥50, pr_title ≤256, comment_body ≥10.
- `audit::JsonlSink` async append com `tokio::sync::Mutex`. Schema: `ts, event_id (uuid_v7), agent_id, tool, repo, rationale, github_response, outcome, patch_sha256, error`.
- 3 handlers em `tools/{push_branch,create_pr,comment_pr}.rs` — validators → ação → audit (`ok`/`rejected`/`failed`).
- 3 tools registradas via `#[tool]` macro do rmcp.

### Onda 3 — Transport patch + 6 testes da MISSION ✅
- `git_ops::apply_and_push`: `TempDir` → `git clone --depth=50 --branch=<base> <url>` → `checkout -b` → `git am --keep-cr --3way --committer-date-is-author-date` (com `commit.gpgsign=false`, identity `gateway@securellm.local`) → unshallow retry on failure → `rev-parse HEAD` → `push origin`.
- `build_clone_url` embute PAT em `x-access-token:`.
- `tracing_redact::redact_pat_in_string` substitui `x-access-token:<pat>@` por `REDACTED@`.
- `reject_oversized_patch` cap em 20 commits.

**Testes:** 26 verdes (19 unit + 4 mission + 2 zero-leak + 1 push_flow).

**MISSION compliance:** T1 (rationale curto) · T2 (allowlist) · T3 (push happy path, autor preservado) · T4 (JSONL acumulativo válido) · T5 (PAT zero-leak em 2 cenários) · T6 (boot sem PAT exit ≤2s).

---

## Próximas ondas

### Onda 4 — Transport HTTP + auth (em andamento)

**Meta:** desbloquear o cenário-alvo da MISSION — agente Claude Code em sandbox remoto da web consegue conectar. Hoje o transport é stdio, exige spawn local.

**Progresso em 2026-05-14:** 4.1–4.3 implementados e validados. `rmcp 1.7.0` foi vendorizado em `vendor/rmcp-1.7.0` porque a feature `server-side-http` declarava `rand = 0.10`, que quebrava resolução com `chacha20 ^0.10.0`; o código do `rmcp` não usa `rand` nessa surface, então o patch local remove só essa dependência fantasma. Validação: `nix develop --command cargo test -p securellm-gateway` verde, incluindo `/mcp` HTTP initialize.

| # | Step | Arquivos | Estimativa |
|---|---|---|---|
| 4.1 | ✅ Adicionar feature `transport-streamable-http-server` ao rmcp em workspace deps. Verificar API atual do rmcp pra confirmar shape. | `Cargo.toml` (workspace), `crates/gateway/Cargo.toml`, `vendor/rmcp-1.7.0` | feito |
| 4.2 | ✅ Modo de transport configurável: env `GATEWAY_TRANSPORT={stdio,http}`, default `stdio`. Bin escolhe runtime. | `crates/gateway/src/bin/gateway-mcp.rs`, `crates/gateway/src/config.rs`, `crates/gateway/src/transport.rs` | feito |
| 4.3 | ✅ HTTP server escutando em `GATEWAY_LISTEN_ADDR` (default `127.0.0.1:8765`). Endpoint MCP por path padronizado do rmcp. | `crates/gateway/src/transport.rs`, `crates/gateway/tests/http_transport.rs` | feito |
| 4.4 | Auth na frente. Decisão: **mTLS** (bridge já tem `rustls`+`rustls-pemfile` no workspace). Cert do cliente identifica o agente — substitui `GATEWAY_AGENT_ID` env (que vira fallback dev). | `crates/gateway/src/auth.rs`, config | próxima |
| 4.5 | Rate limit por agent_id. `governor` já é workspace dep. Default: 10 calls/min por agente. Excedeu → 429 + audit `rate_limited`. | `crates/gateway/src/rate_limit.rs` | 1h |
| 4.6 | Teste de integração: bridge sobe servidor HTTP, cliente teste se conecta com cert válido, faz `tools/list`, e cert inválido recusa. Tooling: `reqwest` + cert auto-gerado em tempdir. | `crates/gateway/tests/http_auth.rs` | 2h |
| 4.7 | Doc operacional: `crates/gateway/DEPLOY.md` com snippet de geração de cert do agente e config do client MCP. | `crates/gateway/DEPLOY.md` | 30min |

**DoD:** sandbox-side fake (curl ou httpie com cert) consegue chamar `tools/list`. Sem cert → recusa. Cert revogado → recusa. Audit JSONL reflete identidade do cert.

**Riscos:**
- rmcp `transport-streamable-http-server` é mais novo que `transport-io`. PF-A: confirmar maturidade antes do trabalho, igual fizemos com rmcp em geral.
- mTLS tem ergonomia ruim pra agentes "na web" que talvez não controlem cert. Fallback: bearer token assinado, mas perde rotação granular.

**Estimativa total:** ~8h.

---

### Onda 5 — Operacional

**Meta:** rodar como serviço, não como `cargo run`.

| # | Step | Arquivos | Estimativa |
|---|---|---|---|
| 5.1 | SOPS-encrypted PAT em `/run/secrets/gateway_github_pat`. Adicionar fonte ao `GatewayConfig::from_env` na ordem `GATEWAY_GITHUB_PAT` (env) → `/run/secrets/gateway_github_pat` (SOPS) → falha. | `crates/gateway/src/config.rs` | 1h |
| 5.2 | Systemd unit `securellm-gateway.service` com `LoadCredential=` pro SOPS path + restart on-failure + journal logging. | `nix/modules/gateway-service.nix` ou similar | 1.5h |
| 5.3 | Observability: tracing layer OTLP opcional via `tracing-opentelemetry`. Métricas básicas: `gateway_tool_calls_total{tool,outcome}`, `gateway_audit_writes_total`, `gateway_active_agents`. | `crates/gateway/src/observability.rs` | 3h |
| 5.4 | Health check endpoint `GET /health` em modo HTTP. Retorna `{audit_writable: bool, github_reachable: bool, allowlist_size: int}`. | `crates/gateway/src/transport.rs` | 1h |
| 5.5 | Logrotate / size cap pro `events.jsonl`. Rotação por dia em `events.YYYYMMDD.jsonl`, último symlink `events.jsonl`. | `crates/gateway/src/audit.rs` | 2h |
| 5.6 | Container/Nix package: `nix build .#gateway` produz binário standalone. Atualizar `flake.nix` (que hoje aponta pra `mcp-server/` inexistente). | `flake.nix`, `nix/` | 2h |

**DoD:** `systemctl start securellm-gateway` sobe limpo, `journalctl -u securellm-gateway` mostra tracing JSON, métricas exportadas, restart automático em falha.

**Estimativa total:** ~10h.

---

### Onda 6 — Migração pro adr-ledger

**Meta:** quando o adr-ledger ficar maduro, swap o `JsonlSink` por backend de ledger sem tocar nos handlers.

| # | Step | Arquivos | Estimativa |
|---|---|---|---|
| 6.1 | Trait `AuditBackend` que abstrai `emit(event) -> Result<()>`. `JsonlSink` vira impl default. | `crates/gateway/src/audit.rs` | 1h |
| 6.2 | Impl `LedgerSink` que conversa com adr-ledger. API do ledger ainda não existe — coordenar quando ele chegar. | `crates/gateway/src/audit/ledger.rs` | depende do ledger |
| 6.3 | Modo dual durante transição: escreve no JSONL E no ledger, valida que ambos produzem mesma sequência. | `crates/gateway/src/audit.rs` | 2h |
| 6.4 | Ferramenta de import: replay do `events.jsonl` histórico → ledger, idempotente por `event_id` (UUIDv7 monotônico ajuda). | `crates/gateway/src/bin/audit-import.rs` | 3h |
| 6.5 | `event_id` evolui pra incluir signature criptográfica do gateway. Public key publicada em `/.well-known/gateway-keys.json`. | `crates/gateway/src/auth.rs` | 2h |
| 6.6 | Cutover: depois de N dias de dual-write sem divergência, JSONL passa a ser cache local + ledger é fonte da verdade. | docs + config flag | — |

**DoD:** ledger e JSONL produzem trilhas equivalentes durante 7 dias seguidos; replay do JSONL histórico no ledger é reproducível bit-a-bit.

**Estimativa total:** depende do ledger.

---

## Backlog não-priorizado

Itens que não justificam onda própria:

- **Wiremock pra happy paths de `create_pr`/`comment_pr`.** Reject paths já cobertos. Só falta o caminho que bate no GitHub API mockado. ~1h.
- **Cross-check `agent_id` via `_meta.agentId`** se `rmcp::RequestContext` expuser. Env continua autoritativo. ~30min.
- **Atualizar `flake.nix`** do bridge: descrição já se autodeclara MCP Server, agora vale verdade; e o env hint `cd mcp-server && npm run build` aponta pra diretório que não existe. ~30min.
- **Atualizar `CLAUDE.md` e `PHOENIX_ARCHITECTURE_REPORT.md`** do bridge: caracterizavam o bridge como MCP server desde sempre; documentar a primeira implementação concreta. ~30min.
- **Patches binários** (`GIT binary patch` marker). Só comentado em `git_ops.rs`, sem teste. ~1h.
- **Push da tag `archive/pre-bridge-merge` no Kluster** quando você decidir. ~5min.
- **Mining DeepSeek do Kluster** (bridge tem 463 LOC vs 441 do Kluster — talvez tenha bits no Kluster que valem). Decidido fora do escopo da Onda 0. ~2h.
- **Allowlist em arquivo TOML** em vez de env var CSV. MISSION sugere mas o repo bate só env vars. Trocar custaria dep nova de `toml`. ~1h.
- **Concurrency stress test.** Hoje 1 push por vez OK; com volume real precisa fila ou semaphore. ~2h pra escrever o stress + tunar.

---

## Decisões arquiteturais registradas

| ID | Decisão | Razão |
|---|---|---|
| **D1** | `rmcp` 1.7 como SDK MCP em vez de JSON-RPC hand-rolled | PF-1 confirmou maturidade: 1.x estável, release 2026-05-13 (cadência ~2 semanas), macros `#[tool]` ergonômicas, exemplos `_stdio.rs` no repo oficial |
| **D2** | `git` CLI via `tokio::process::Command` em vez de `git2`/libgit2 | libgit2 não implementa `git am` — usaríamos `git apply` e perderíamos autor/data/mensagem dos commits. Sandbox já tem git binário. Injeção de PAT trivial via URL |
| **D3** | Opção B (patch-based, stateless) em vez de Opção A (clone server-side) | Stateless, paraleliza por agente, patch faz parte natural do audit (`patch_sha256`), agente controla conteúdo dos commits |
| **D4** | base64 do mbox raw como encoding do patch | Evita inferno de escape JSON com `\n`/`"`/`\r`; sobrevive ao transport sem mudança; overhead +33% aceitável |
| **D5** | `agent_id` via env var `GATEWAY_AGENT_ID` (autoritativo) | MCP spec não passa identidade em tool calls. Cross-check opcional contra `_meta.agentId` se rmcp expuser. Em modo HTTP+mTLS, CN do cert substitui a env |
| **D6** | Audit JSONL próprio em vez de reusar `crates/core/src/audit.rs::AuditEvent` | Domínios diferentes: core é LLM-token-cost (`prompt_tokens`, `cost_usd`), gateway é ação-justificativa (`rationale`, `github_response`) |
| **D7** | Casa: crate novo no `securellm-bridge`, não no `securellm-mcp` | Diversificação + valorização do bridge que tinha infra robusta mas sem feature anchor de propósito próprio. Bridge já se autoanunciava MCP Server na doc |
| **D8** | `schemars` 1.2 (não 0.8) | rmcp 1.7 puxa schemars 1.2.1; mismatch de versão quebra `JsonSchema` em `Parameters<T>` |

---

## Glossário operacional

- **T1–T6**: testes obrigatórios da MISSION (rationale curto, allowlist, push happy, JSONL válido, PAT zero-leak, boot sem PAT)
- **PF-1/2/3**: pre-flight checks (rmcp maturity, git CLI vs git2, agent_id strategy)
- **Onda**: agrupamento de sprints que entrega uma capacidade completa
- **`gateway_started`**: evento de audit emitido no boot — primeira linha do `events.jsonl` de cada sessão
- **`patch_sha256`**: hash SHA-256 do patch mbox raw, registrado no audit pra content-addressing futuro no ledger
- **agente "na web"**: instância Claude Code rodando em sandbox remoto (claude.ai/code) — o cliente-alvo final desta arquitetura

---

## Como evoluir este documento

- Onda fechada → marcar ✅ + um link curto pro commit/PR
- Decisão nova que muda comportamento → adicionar linha em "Decisões arquiteturais registradas"
- Item de backlog promovido a onda → mover seção + numerar
- Nunca apagar Decisões — superseder com nova entrada referenciando a antiga (`D6 → D6'` se mudar audit shape, por exemplo)
