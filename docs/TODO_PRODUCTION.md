# 🔧 SecureLLM Bridge — Production TODO List

**Data**: 2026-01-15  
**Auditoria**: 12 crates analisados — 10 REAL, 1 MISTO (security), 0 puramente stub  

---

## 📊 Resumo da Auditoria

| Crate | Status | Notas |
|-------|--------|-------|
| `core` | ✅ REAL | Traits, tipos, pricing, smart routing, QoS observatory |
| `security` | ⚠️ MISTO | TLS+Secrets prontos / Sanitizer+Sandbox+Crypto stubs |
| `providers` | ✅ REAL | 8 providers (DeepSeek, OpenAI, Anthropic, Gemini, Groq, LlamaCpp, Nvidia, MlOps) |
| `api-server` | ✅ REAL | Axum + SSE streaming + health + metrics + graceful shutdown |
| `gateway` | ✅ REAL | MCP server Git/GitHub com PAT redaction |
| `agents` | ✅ REAL | Agent executor + parser XML + cache + execução paralela |
| `task-manager` | ✅ REAL | Tasks + prioridade + SQLite persistence |
| `context-manager` | ✅ REAL | zstd + sliding window + LRU cache (tokenizer.rs é stub) |
| `cli` | ✅ REAL | 6 comandos + REPL (só expõe 2 providers) |
| `tui` | ✅ REAL | Zellij-style com 7 painéis, 4 modos, tema |
| `agent-tui` | ✅ REAL | Agent overlay para Zellij |
| `voice-agents` | ✅ REAL | TTS + audio capture + Wyoming protocol |

---

## 🔴 CRÍTICO — Security Gaps

### 1. Sanitizer (`crates/security/src/sanitizer.rs`) ✅ CONCLUÍDO

**Status**: ✅ COMPLETO — PII detection, prompt injection, content filtering, 43 testes

**O que foi implementado**:
- [x] **1.1 PII Detection** — 17 padrões de regex com Lazy static:
  - [x] CPF e CNPJ (formatados e não-formatados)
  - [x] Email (RFC 5322 simplificado)
  - [x] Telefone brasileiro e internacional
  - [x] IPv4 e IPv6 (incluindo abreviação ::)
  - [x] Cartão de crédito (regex + validação Luhn)
  - [x] API Keys: OpenAI, Anthropic, Google, AWS, genérica
  - [x] JWT, chave privada PEM, connection strings
- [x] **1.2 Prompt Injection Detection** — 6 heurísticas:
  - [x] "Ignore previous instructions" (Inglês + Português)
  - [x] "System prompt leakage" (reveal/show/print your system prompt)
  - [x] Delimiter injection (###, ---, <|im_start|>, [INST], etc.)
  - [x] Role confusion (system:, assistant:, admin:)
  - [x] Base64 payloads obfuscados
  - [x] Jailbreak patterns (DAN mode, developer mode, etc.)
- [x] **1.3 Content Filtering**:
  - [x] Blocklist de 5 categorias (bomb/weapon, suicide, child exploitation, malware, doxxing)
  - [x] Configurável via `SanitizerConfig` (ativa/desativa por categoria)
- [x] **1.4 SanitizerConfig**:
  - [x] `redact_pii` — substitui PII por `[REDACTED:TYPE]`
  - [x] `block_injection` — bloqueia requisição com prompt injection
  - [x] `block_harmful_content` — bloqueia conteúdo nocivo
  - [x] `max_message_length` — limite de tamanho por mensagem
  - [x] `custom_patterns` — regexes definidos pelo usuário
- [x] **1.5 Testes** — 43 testes: PII detection, redaction, injection, blocking, response sanitization

**Arquivo**: `crates/security/src/sanitizer.rs`

---

### 2. Sandbox ✅ PARCIAL (Phase 1) (`crates/security/src/sandbox.rs`)

**Status**: STUB — `execute()` retorna erro "Sandboxing not yet implemented"

**O que foi implementado (Phases 1-3 da ADR-0001)**:
- [x] **2.1 Process Isolation** — `unshare` namespaces (user, mount, PID, network)
    - [x] Processo filho isolado com `kill_on_drop`
    - [x] `wait_with_output()` + `tokio::time::timeout`
- [x] **2.2 Resource Limits** — cgroups v2 com graceful degradation
    - [x] `memory.max` aplicado via `CgroupManager`
    - [x] `pids.max` aplicado via `CgroupManager`
    - [x] Timeout via `tokio::time::timeout`
- [x] **2.3 Network Restrictions**:
    - [x] Network namespace isolado com `unshare -n`
  
- [x] **2.4 Filesystem Access**:
    - [x] `None` → tmpfs vazio via `unshare` mount setup
    - [x] `ReadOnly` → mounts preservados com proc
    - [x] `Full` → hereda sistema com --mount-proc
- [x] **2.5 Testes** — 13 testes (execução, timeout, network, cgroups, cleanup)

**Arquivos**: `crates/security/src/sandbox.rs`, `crates/security/src/cgroup_helper.rs`, `nix/modules/sandbox.nix`

---

### 3. Crypto Encrypt/Decrypt (`crates/security/src/crypto.rs`) ✅ CONCLUÍDO

**Status**: ✅ COMPLETO — AES-256-GCM, ChaCha20-Poly1305, key wrapping, 20 testes

**O que foi implementado**:
- [x] **3.1 AES-256-GCM Encryption**:
  - [x] `encrypt(key, plaintext) -> ciphertext` com nonce aleatório (12 bytes prepended)
  - [x] `decrypt(key, ciphertext) -> plaintext` com verificação de tag (tamper detection)
  - [x] `encrypt_with_random_key()` para keys efêmeras
- [x] **3.2 ChaCha20-Poly1305**:
  - [x] `encrypt_chacha()` / `decrypt_chacha()` para CPUs sem AES-NI
- [x] **3.3 Key Wrapping**:
  - [x] `derive_key_from_password()` via Argon2id
  - [x] `encrypt_with_password()` / `decrypt_with_password()` (salt + nonce + tag)
- [x] **3.4 Testes** — 20 testes: roundtrip, wrong key, tampering, truncation, nonce uniqueness, large payload (1MB), cross-algorithm safety, invalid key length

---

## 🟡 MÉDIO — Integration & Completeness Gaps

### 4. CLI Provider Factory (`crates/cli/src/provider_factory.rs`)

**Status**: PARCIAL — só expõe DeepSeek + OpenAI, mas existem 8 providers implementados

**O que foi implementado (Phases 1-3 da ADR-0001)**:
- [ ] **4.1 Adicionar providers ao `IMPLEMENTED_PROVIDERS`**:
  - [ ] `anthropic`
  - [ ] `gemini`
  - [ ] `groq`
  - [ ] `llamacpp`
  - [ ] `nvidia`
  - [ ] `mlops`
- [ ] **4.2 Adicionar branches no `build_provider()`** para cada provider
- [ ] **4.3 Adicionar `default_model()`** para cada provider
- [ ] **4.4 Testes** — `build_info_provider` para cada provider novo

**Arquivo**: `crates/cli/src/provider_factory.rs`

---

### 5. Context Tokenizer (`crates/context-manager/src/tokenizer.rs`)

**Status**: STUB — só tem comentário placeholder

**O que foi implementado (Phases 1-3 da ADR-0001)**:
- [ ] **5.1 Token counting** usando `tiktoken-rs` (já é dependência):
  - [ ] Suporte a modelos OpenAI (cl100k_base, p50k, r50k)
  - [ ] Suporte a modelos DeepSeek
  - [ ] Fallback: estimativa por char/4
- [ ] **5.2 Token truncation**:
  - [ ] Truncar mensagens mantendo as mais recentes
  - [ ] Preservar system message
- [ ] **5.3 Testes** — Contagem de tokens vs referência conhecida

**Arquivo**: `crates/context-manager/src/tokenizer.rs`

---

### 6. Integration Tests para Providers

**Status**: Não existem testes de integração com APIs reais

**O que foi implementado (Phases 1-3 da ADR-0001)**:
- [ ] **6.1 Testes com WireMock** (já é dev-dependency):
  - [ ] DeepSeek: mock de chat completion
  - [ ] OpenAI: mock de chat completion
  - [ ] Anthropic: mock de mensagens
- [ ] **6.2 Testes end-to-end** com `api-server`:
  - [ ] POST `/v1/chat/completions` → resposta válida
  - [ ] SSE streaming → chunks corretos
  - [ ] Rate limiting → 429 após exceder limite

---

## 🟢 BAIXO — Polish & Documentation

### 7. Observability

- [ ] **7.1 Tracing spans** nos pontos críticos:
  - [ ] Provider calls (latência por provider)
  - [ ] Token counting (custo estimado)
  - [ ] Sanitizer pass (tempo de sanitização)
- [ ] **7.2 Métricas Prometheus** no `/metrics`:
  - [ ] `securellm_requests_total{provider, status}`
  - [ ] `securellm_tokens_total{provider, type}`
  - [ ] `securellm_latency_seconds{provider, quantile}`

### 8. CI/CD

- [ ] **8.1 Pipeline completa** (`.github/workflows/ci.yml`):
  - [ ] `cargo fmt --check`
  - [ ] `cargo clippy -- -D warnings`
  - [ ] `cargo test --workspace`
  - [ ] `cargo audit`
- [ ] **8.2 Docker build** no pipeline
- [ ] **8.3 Release automation** — tag → build → publish

### 9. Documentação

- [ ] **9.1 API docs** — `cargo doc` publicável
- [ ] **9.2 Provider guide** — como adicionar novo provider
- [ ] **9.3 Security guide** — como configurar TLS, rate limiting
- [ ] **9.4 Deploy guide** — Docker, NixOS, Kubernetes

| # | Tarefa | Prioridade | Status |
|---|--------|-----------|--------|
| 1.1 | PII Detection | ✅ DONE |
| 1.2 | Prompt Injection Detection | ✅ DONE |
| 1.3 | Content Filtering | ✅ DONE |
| 1.4 | Sanitizer Tests (43 tests) | ✅ DONE |
| 2.1 | Process Isolation (unshare) | ✅ DONE |
| 2.2 | Resource Limits (cgroups) — graceful degradation | ✅ DONE |
| 2.3 | Network Restrictions (net namespace) | ✅ DONE |
| 2.4 | Filesystem Access (tmpfs/readonly) | ✅ DONE |
| 2.5 | Sandbox Tests (13 tests) | ✅ DONE |
| 3.1 | AES-256-GCM Encrypt/Decrypt | 🔴 CRÍTICO | ✅ DONE |
| 3.2 | ChaCha20-Poly1305 | 🔴 CRÍTICO | ✅ DONE |
| 3.3 | Key Wrapping (Argon2id) | 🔴 CRÍTICO | ✅ DONE |
| 3.4 | Crypto Tests (20 tests) | 🔴 CRÍTICO | ✅ DONE |
| 4 | CLI: todos os 8 providers + testes | ✅ DONE |
| 5 | Tokenizer implementation | 🟡 MÉDIO | ⬜ TODO |
| 6 | Integration Tests | 🟡 MÉDIO | ⬜ TODO |
| 7 | Observability (tracing + metrics) | 🟢 BAIXO | ⬜ TODO |
| 8 | CI/CD Pipeline | 🟢 BAIXO | ⬜ TODO |
| 9 | Documentação | 🟢 BAIXO | ⬜ TODO |

**Total**: 19 tarefas | **Críticas**: 12 | **Médias**: 3 | **Baixas**: 4

---

*Última atualização: 2026-01-15 — Auditoria de código*
