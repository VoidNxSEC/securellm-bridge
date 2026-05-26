# SecureLLM API Server

OpenAI-compatible API server for unified LLM management across multiple providers.

## Features

- **OpenAI-Compatible API**: Drop-in replacement for OpenAI API
- **Multi-Provider Support**: DeepSeek, OpenAI, Anthropic, Groq, Cohere, llama.cpp/KoboldCPP
- **Advanced Reliability**: Circuit breakers, retry logic, failover
- **Production-Ready**: Connection pooling, rate limiting, caching
- **Observable**: Prometheus metrics, OpenTelemetry tracing, structured logging
- **Scalable**: SQLite registry, Redis caching, async architecture

## API Endpoints

### OpenAI-Compatible

- `GET /v1/models` - List all available models
- `POST /v1/chat/completions` - Chat completions (supports streaming)
- `POST /v1/completions` - Text completions

#### Chat Streaming

`POST /v1/chat/completions` supports OpenAI-compatible Server-Sent Events when the request includes `"stream": true`.

Current behavior:
- Returns `Content-Type: text/event-stream`.
- Emits `chat.completion.chunk` payloads with `choices[].delta`.
- Ends successful streams with `data: [DONE]`.
- Uses real provider streaming for OpenAI-compatible providers: `openai`, `deepseek`, `groq`, and `ml-ops`.
- Providers without `stream_request` support fail explicitly for streaming requests instead of returning mock data.

Example:

```bash
curl -N http://localhost:8080/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "openai/gpt-4o-mini",
    "messages": [{"role": "user", "content": "Say hello"}],
    "stream": true
  }'
```

Known limits:
- Native Anthropic streaming is not wired yet.
- Legacy `POST /v1/completions` streaming is not implemented.
- `model="auto/..."` may fallback before the first streamed chunk; after the stream starts, it does not switch providers midstream.

### Management

- `GET /api/health` - Detailed health check
- `GET /api/ready` - Readiness probe (K8s)
- `GET /api/metrics` - Prometheus metrics
- `POST /api/models/sync` - Trigger model discovery

## Configuration

### Environment Variables

See [`.env.example`](.env.example) for all available configuration options.

Key variables:
- `SERVER_PORT` - Server port (default: 8080)
- `DATABASE_URL` - SQLite database path
- `REDIS_URL` - Redis connection string
- `{PROVIDER}_ENABLED` - Enable/disable providers
- `{PROVIDER}_API_KEY` - API keys for cloud providers
- `LLAMACPP_BASE_URL` - URL for local llama.cpp server

### Configuration File (Optional)

Alternatively, use a TOML configuration file:

```bash
export CONFIG_PATH=/etc/securellm/config.toml
```

## Development

### Build

```bash
nix develop --command cargo build --release
```

### Run

```bash
# Copy and configure environment
cp .env.example .env
# Edit .env with your settings

# Run server
nix develop --command cargo run --bin securellm-api-server
```

### Database Migrations

Migrations run automatically on startup using sqlx.

To create new migrations:

```bash
nix develop --command sqlx migrate add <migration_name>
```

## Roadmap

### API Server Progress

- [x] OpenAI-compatible `/v1/chat/completions` non-streaming path.
- [x] Real SSE streaming for OpenAI-compatible providers (`openai`, `deepseek`, `groq`, `ml-ops`).
- [x] Explicit unsupported-streaming errors for providers without streaming implementations.
- [x] WireMock coverage for provider SSE parsing and API-route SSE output.
- [ ] Native Anthropic streaming.
- [ ] Legacy `/v1/completions` implementation beyond the current mock response.
- [ ] Legacy `/v1/completions` streaming.
- [ ] Streaming usage accounting from providers that emit final usage metadata.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      API Server (Axum)                       │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ OpenAI-Compatible Endpoints (/v1/*)                    │ │
│  │  - models, chat/completions, completions               │ │
│  └────────────────────────────────────────────────────────┘ │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ Management Endpoints (/api/*)                          │ │
│  │  - health, ready, metrics, models/sync                 │ │
│  └────────────────────────────────────────────────────────┘ │
│                            │                                 │
│  ┌─────────────────────────▼──────────────────────────────┐ │
│  │          Provider Manager + Circuit Breakers           │ │
│  │  ┌──────────┬──────────┬──────────┬──────────────────┐ │ │
│  │  │ DeepSeek │ OpenAI   │ Anthropic│ llama.cpp/Kobold │ │ │
│  │  └──────────┴──────────┴──────────┴──────────────────┘ │ │
│  └────────────────────────────────────────────────────────┘ │
│                            │                                 │
│  ┌────────────┬────────────▼────────────┬─────────────────┐ │
│  │  SQLite    │   Redis Cache/Rate      │  Prometheus     │ │
│  │  Registry  │   Limiting              │  Metrics        │ │
│  └────────────┴─────────────────────────┴─────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Deployment

See the `model-api` kit in the parent repository for Docker deployment.

## License

See root LICENSE file.
