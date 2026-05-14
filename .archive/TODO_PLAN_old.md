# 🏥 PHANTOM Surgical Repair Plan

**Date:** 2026-01-07
**Objective:** Modularize the codebase, separate concerns (Vertex AI vs. Core Logic), fix critical bugs, and establish robust health checks.

## 📊 Current Status & Diagnosis

| Component | Status | Issues Identified |
|-----------|--------|-------------------|
| **CLI (`cerebro`)** | ⚠️ Partial | `ingest` fails with Error 400 (Batch Size). `query` returns empty if no data. Missing `health` check. |
| **RAG Engine** | ❌ Broken | `VertexAIEmbeddings` used without batching. 1184 docs sent > 250 limit. Tightly coupled to LangChain/Vertex. |
| **Tests** | ⚠️ Weak | Unit tests mock everything. No integration tests to catch API limits. |
| **Architecture** | 🕸️ Monolithic | `src/phantom/core` mixes logic. No clear API boundaries. |
| **CI/CD** | 🟢 Basic | Runs `scripts/ci-test.sh`. Needs expansion. |

## 🛠️ Execution Plan

### Phase 1: Stabilization & Diagnostics (Completed)
- [x] **Document Missing Coverage:** Created `docs/COVERAGE_GAP.md`.
- [x] **Implement Health Check:**
    - Created `phantom ops health` command.
    - Check: Write permissions, Vertex AI connectivity (ping), Vector DB accessibility.
- [x] **Fix Critical Bug (`ingest`):**
    - Implemented batching (chunk_size=20) in `RigorousRAGEngine.ingest`.
    - Added exponential backoff for Rate Limits.
    - Fixed Analysis engine (Indentation + TreeSitter fallback).
    - Verified fix with `just ingest` and Integration Tests.

### Phase 2: Modularization (The Surgery)
- [ ] **Define Interfaces (Abstract Base Classes):**
    - `src/phantom/interfaces/llm.py`: `LLMProvider`
    - `src/phantom/interfaces/vector_store.py`: `VectorStoreProvider`
- [ ] **Refactor `core/rag`:**
    - Move Vertex AI specific code to `src/phantom/providers/gcp/`.
    - Move Chroma specific code to `src/phantom/providers/chroma/`.
    - `RigorousRAGEngine` should depend on Interfaces, not concrete classes.
- [ ] **Decouple "Credit Burner" from "RAG":**
    - Ensure `VertexAI` usage for RAG (Trial Credits) is distinct from any production API usage.

### Phase 3: Testing & Quality Assurance
- [ ] **Integration Tests:**
    - Create `tests/integration/test_vertex_limits.py` (Real API call with batching).
- [ ] **Unit Tests:**
    - Update tests to mock the new Interfaces.
- [ ] **CI Pipeline:**
    - Update `.github/workflows/ci.yml` to run linting (`ruff`) and integration tests (if secrets present).

### Phase 4: Documentation & Polish
- [ ] **API Documentation:** Document the new Interfaces.
- [ ] **Developer Guide:** Update `CONTRIBUTING.md` with new module structure.
- [ ] **Justfile:** Update recipes to support `just health`, `just test-integration`.

## 🚀 Next Step
Awaiting approval to begin **Phase 1: Stabilization & Diagnostics**.
