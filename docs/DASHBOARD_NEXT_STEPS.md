# SecureLLM Bridge - Next Steps

**Data**: 2026-01-26
**Milestone**: Deploy Dev + Showcase Dashboard + ADR Ledger Registry
**Status**: Frontend Validado ✅ | Backend Pendente ⏳

---

## 📋 Status Atual

### ✅ Completado

- **Dashboard Frontend (React + TypeScript)**
  - ✅ Estrutura completa com 5 páginas
  - ✅ Componentes UI (Radix UI + Tailwind CSS)
  - ✅ State management (Zustand + TanStack Query)
  - ✅ Configuração Vite + TypeScript
  - ✅ Dependências instaladas (269 packages)
  - ✅ Servidor dev rodando em `http://localhost:3001/`
  - ✅ Proxy configurado para backend em `localhost:8000`

### 📊 Frontend Features

1. **Dashboard** (`/`) - Overview do ecossistema ~/arch
   - Cards de estatísticas (total projetos, intelligence, health score, alerts)
   - Lista de projetos que precisam atenção (ordenados por health score)
   - Alertas recentes
   - Sumário do Daily Briefing
   - Quick Actions

2. **Projects** (`/projects`) - Gestão de projetos
   - Listagem completa de projetos
   - Filtros por status e linguagem
   - Análise individual de projeto
   - Métricas de saúde

3. **Intelligence** (`/intelligence`) - Sistema de busca
   - Busca semântica e keyword
   - Filtros por tipo (SIGINT, HUMINT, OSINT, TECHINT)
   - Filtros por projeto
   - Visualização de resultados

4. **Briefing** (`/briefing`) - Sumários executivos
   - Daily Briefing
   - Executive Summary
   - Key developments
   - Project summaries

5. **Settings** (`/settings`) - Configurações
   - Auto-refresh settings
   - Environment selection
   - Preferências do usuário

---

## 🚧 Pendente - Backend API

### Objetivo
Criar servidor backend em Rust que sirva os dados para o dashboard frontend.

### Localização
`crates/api-server/` - Já existe estrutura base, precisa adicionar endpoints do dashboard

### Endpoints Necessários

#### 1. Status & Overview
```rust
GET /api/status
Response: {
  total_projects: number,
  active_projects: number,
  health_score: number,
  total_intelligence: number,
  alerts_count: number,
  last_scan: string | null
}
```

#### 2. Projects
```rust
GET /api/projects?status=<status>&language=<lang>&sort_by=<field>&order=<asc|desc>
Response: Project[]

GET /api/projects/:name
Response: Project

POST /api/projects/:name/summarize
Response: ProjectAnalysis
```

#### 3. Intelligence
```rust
GET /api/intelligence/query?q=<query>&types=<types>&projects=<projects>&limit=<n>&semantic=<bool>
Response: {
  query: string,
  results: IntelligenceItem[],
  total: number,
  search_type: 'semantic' | 'keyword'
}

GET /api/intelligence/stats
Response: {
  total: number,
  by_type: Record<string, number>,
  by_threat: Record<string, number>
}
```

#### 4. Briefings
```rust
GET /api/briefing/daily
Response: Briefing

GET /api/briefing/executive
Response: Briefing
```

#### 5. Alerts
```rust
GET /api/alerts
Response: Alert[]
```

#### 6. Graph
```rust
GET /api/graph/dependencies
Response: {
  nodes: Array<{ id: string, label: string }>,
  edges: Array<{ source: string, target: string }>
}
```

#### 7. Actions
```rust
POST /api/scan
Response: { message: string }
```

### Implementação Sugerida

#### Opção 1: Mock Server (Rápido para showcase)
```bash
# Criar servidor mock simples
cd crates/api-server
cargo new --bin dashboard-mock

# Implementar endpoints com dados hardcoded/fake
# Rodar em localhost:8000
```

#### Opção 2: Integração Real (Produção)
```rust
// Em crates/api-server/src/routes/dashboard.rs

use axum::{Router, routing::get, Json};

pub fn router() -> Router {
    Router::new()
        .route("/status", get(get_status))
        .route("/projects", get(get_projects))
        .route("/projects/:name", get(get_project))
        // ... outros endpoints
}

// Implementar lógica real:
// - Scan do filesystem ~/arch
// - Análise de projetos (health score, languages, commits)
// - Sistema de intelligence (indexação, busca)
// - Geração de briefings
```

### Estrutura de Dados Real

O backend precisa implementar:

1. **Project Scanner**
   - Escanear diretórios em `~/arch`
   - Detectar linguagens (git, package.json, Cargo.toml, etc)
   - Calcular health score (commits recentes, testes, docs)
   - Status (active, maintenance, deprecated)

2. **Intelligence System**
   - Indexar artefatos (README, CHANGELOG, commits, issues)
   - Categorizar por tipo (SIGINT, HUMINT, OSINT, TECHINT)
   - Sistema de busca (keyword + semantic embedding)
   - Threat level classification

3. **Briefing Generator**
   - Agregar dados de projetos
   - Identificar key developments
   - Gerar sumários executivos
   - Detectar alertas automáticos

4. **Dependency Graph**
   - Analisar imports/dependencies
   - Construir grafo de relações
   - Detectar dependências circulares

---

## 📝 ADR Ledger Registry

### Objetivo
Registrar decisão arquitetural da nova feature do Dashboard no ADR ledger do projeto.

### Estrutura ADR

Criar arquivo `docs/adr/XXXX-dashboard-frontend-architecture.md`:

```markdown
# ADR-XXXX: Dashboard Frontend Architecture

## Status
Proposed / Accepted / Deprecated / Superseded

## Context
- Necessidade de interface visual para monitoramento do ecossistema ~/arch
- Requisitos de observabilidade e intelligence gathering
- Showcase para validação do conceito

## Decision
### Frontend
- Framework: React 18 + TypeScript
- Build Tool: Vite 5.0.8 (SWC)
- State: Zustand + TanStack Query
- UI: Tailwind CSS + Radix UI + Framer Motion
- Routing: React Router DOM

### Backend
- Runtime: Rust (Tokio + Axum)
- API: RESTful JSON sobre HTTP
- Port: 8000 (desenvolvimento)
- Proxy: Vite dev server (3001 → 8000)

### Architecture Pattern
- Client-server separation (SPA frontend + API backend)
- Real-time updates via polling (auto-refresh configurable)
- Type-safe contracts (TypeScript interfaces ↔ Rust serde types)

## Consequences

### Positive
- Separação clara frontend/backend
- Type safety end-to-end
- Hot reload rápido (Vite)
- Componentização reusável (Radix UI)
- Performance (Rust backend)

### Negative
- Dois runtimes (Node + Rust)
- Complexidade de build (npm + cargo)
- Necessidade de CORS em produção

### Neutral
- Proxy development (produção usa Nginx/Caddy)
- State management via hooks (pode escalar para Redux se necessário)

## Alternatives Considered
1. **SSR com Next.js**: Mais complexo, overkill para dashboard interno
2. **Rust fullstack (Leptos/Dioxus)**: Menos maduro, falta de bibliotecas UI
3. **TUI apenas (Ratatui)**: Bom para CLI, mas dificulta visualização de grafos

## Related
- M1-M3: Agent overlay implementation
- TUI Redesign: Terminal interface complementar
```

### Localização do ADR Ledger

```bash
# Verificar onde está o ADR ledger do projeto
ls -la docs/adr/
# ou
ls -la architecture/decisions/
# ou
grep -r "ADR\|Architecture Decision" .
```

### Comando para Registro

```bash
# Criar novo ADR
./scripts/new-adr.sh "Dashboard Frontend Architecture"

# ou manual
cp docs/adr/template.md docs/adr/XXXX-dashboard-frontend-architecture.md
vim docs/adr/XXXX-dashboard-frontend-architecture.md

# Commit
git add docs/adr/
git commit -m "docs: add ADR for dashboard frontend architecture"
```

---

## 🎯 Roadmap de Execução

### Sprint 1: Backend Mock (2-4 horas)
- [ ] Criar `crates/api-server/src/bin/dashboard-mock.rs`
- [ ] Implementar endpoints com dados hardcoded
- [ ] CORS configuration para desenvolvimento
- [ ] Rodar servidor em localhost:8000
- [ ] Validar integração frontend ↔ backend

### Sprint 2: Backend Real (1-2 semanas)
- [ ] Filesystem scanner (`~/arch`)
- [ ] Project analyzer (health, languages, commits)
- [ ] Intelligence indexer (README, CHANGELOG, etc)
- [ ] Busca keyword (ripgrep/tantivy)
- [ ] Busca semântica (embeddings via API ou local)
- [ ] Briefing generator (agregação + LLM summary)
- [ ] Dependency graph builder

### Sprint 3: ADR + Documentation (1 dia)
- [ ] Criar ADR formal
- [ ] Atualizar CLAUDE.md
- [ ] Documentar API endpoints (OpenAPI spec)
- [ ] Screenshots para showcase
- [ ] Demo script

### Sprint 4: Production Readiness (3-5 dias)
- [ ] Docker Compose (frontend + backend)
- [ ] Nginx reverse proxy configuration
- [ ] Environment variables (.env)
- [ ] Logging & monitoring setup
- [ ] Security hardening (rate limiting, auth)
- [ ] CI/CD pipeline (build + test)

---

## 🔧 Comandos Úteis

### Development

```bash
# Terminal 1: Frontend
cd dashboard
nix develop ../ --command npm run dev
# http://localhost:3001

# Terminal 2: Backend (quando implementado)
cargo run --bin dashboard-server
# http://localhost:8000

# Terminal 3: Watch logs
tail -f /var/log/securellm/api-server.log
```

### Build Production

```bash
# Frontend
cd dashboard
npm run build
# Output: dashboard/dist/

# Backend
cargo build --release --bin dashboard-server
# Output: target/release/dashboard-server

# Docker
docker-compose up -d
```

### Testing

```bash
# Frontend
cd dashboard
npm run lint
npm run test  # (quando testes forem adicionados)

# Backend
cargo test --package securellm-api-server

# Integration
curl http://localhost:8000/api/status
curl http://localhost:8000/api/projects
```

---

## 📚 Referências

### Frontend Stack
- Vite: https://vitejs.dev/
- React Query: https://tanstack.com/query/latest
- Radix UI: https://www.radix-ui.com/
- Tailwind: https://tailwindcss.com/
- Zustand: https://zustand-demo.pmnd.rs/

### Backend Stack
- Axum: https://docs.rs/axum/latest/axum/
- Tower: https://docs.rs/tower/latest/tower/
- SQLx: https://github.com/launchbadge/sqlx
- Tokio: https://tokio.rs/

### Architecture
- ADR Template: https://github.com/joelparkerhenderson/architecture-decision-record
- API Design: https://restfulapi.net/

---

## ✅ Checklist Final (Showcase Ready)

- [x] Frontend compilando sem erros
- [x] Servidor dev rodando (localhost:3001)
- [x] Todas as páginas renderizando
- [ ] Backend servindo dados (localhost:8000)
- [ ] Frontend ↔ Backend integrado
- [ ] ADR documentado
- [ ] Screenshots capturadas
- [ ] Demo preparado

---

**Próximo Passo Imediato**: Escolher entre Mock Server (rápido) ou Backend Real (completo) e implementar os endpoints da API.

**Estimativa**: Mock Server = 2-4h | Backend Real = 1-2 semanas
