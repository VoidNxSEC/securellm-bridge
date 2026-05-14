# 🚀 SecureLLM Bridge: Production Readiness Roadmap

Este documento define a "Full Stack Check-list" necessária para elevar o projeto de *Prototype* para *Production Grade*.

## 🏗️ Phase 1: The Bridge Architecture (Rust ↔ Python)
O código Python (Phantom/Cerebro) atualmente roda isolado. O servidor Rust precisa orquestrar isso.

- [ ] **Architecture Decision:** Definir padrão de integração.
    - *Opção A (Performance):* Embedding via `pyo3` (Rust carrega o interpretador Python).
    - *Opção B (Isolamento/Recomendado):* Sidecar API (Rust dispara um processo Python FastAPI local e comunica via HTTP/UDS).
- [ ] **Process Manager:** Implementar gerenciador de processo no Rust para iniciar/monitorar o `phantom`.
- [ ] **Data Protocol:** Definir Schemas JSON estritos para troca de mensagens (Requests de Análise ↔ Resultados).

## 🛡️ Phase 2: Security Hardening (The "Secure" in SecureLLM)
Os arquivos em `crates/security` são apenas stubs atualmente.

- [ ] **Input Sanitization (`sanitizer.rs`):**
    - [ ] Implementar Regex patterns para detectar PII (CPFs, Emails, Keys).
    - [ ] Bloquear Injeção de Prompt (Prompt Injection heuristics).
- [ ] **Secrets Management (`secrets.rs`):**
    - [ ] Integração com Keyring do sistema ou Vault (HashiCorp/Env).
    - [ ] Nunca logar chaves de API em texto plano (redaction).
- [ ] **Network Security (`tls.rs`):**
    - [ ] Enforce mTLS para comunicação entre Agentes.
    - [ ] Configuração de HTTPS no `api-server`.

## 💾 Phase 3: Persistence & State
O sistema atual depende muito de arquivos JSONL e SQLite efêmero.

- [ ] **Database Migration:**
    - [ ] Consolidar `ssh_sessions.db` e logs em um SQLite robusto (com WAL mode ativado) ou PostgreSQL.
    - [ ] Criar migrations via `sqlx` (Rust).
- [ ] **Caching Layer:**
    - [ ] Implementar Redis (já presente no `flake.nix`) para cache de respostas de LLM (economizar tokens).
    - [ ] Rate Limiting por IP/Token no Rust (`governor` crate).

## 📊 Phase 4: Observability & Reliability
Para produção, "funcionar" não basta; precisamos saber *como* está funcionando.

- [ ] **Structured Logging:**
    - [ ] Implementar `tracing` (Rust) e `structlog` (Python) com correlação de IDs (TraceID).
- [ ] **Metrics:**
    - [ ] Expor endpoint `/metrics` (Prometheus compatível).
    - [ ] Monitorar: Latência de inferência, Taxa de erros, Uso de memória do processo Python.
- [ ] **Error Handling:**
    - [ ] Mapear erros do Python para `Enum` de erros no Rust.
    - [ ] Implementar Retry policies (backoff exponencial) para falhas de API externa.

## 📦 Phase 5: CI/CD & Deployment
- [ ] **Dockerização Híbrida:** Criar Dockerfile Multi-stage (Build Rust + Setup Python env).
- [ ] **Nix Flake Output:** Garantir que `nix build .#docker` gere a imagem OCI correta.
- [ ] **GitHub Actions:** Pipeline de testes (Unitários Rust + Integração Python).

---
*Created by Gemini CLI Assistant - 2026-02-04*
