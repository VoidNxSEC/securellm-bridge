# OpenWebUI Integration Guide

## Overview

SecureLLM Bridge is now integrated with OpenWebUI at `~/arch/docker-hub/ml-clusters/kits/ai-suite/`. The bridge acts as a unified API gateway that routes requests to multiple LLM providers with smart fallback.

## Architecture

```
OpenWebUI (Docker)
     ↓
host.docker.internal:8080 (SecureLLM Bridge)
     ↓
┌────────────────────────────────────┐
│  Provider Priority (Auto Routing)  │
├────────────────────────────────────┤
│  1. llamacpp   (local-model)       │  ← Local inference (FREE)
│  2. deepseek   (deepseek-chat)     │  ← Cheapest cloud ($0.0001/1k)
│  3. groq       (llama-3.3-70b)     │  ← Fast & cheap
│  4. openai     (gpt-4-turbo)       │  ← Industry standard
│  5. anthropic  (claude-3-sonnet)   │  ← Premium quality
└────────────────────────────────────┘
```

## Quick Start

### 1. Start Redis (Required)

```bash
nix-shell -p redis --command "redis-server --daemonize yes --port 6379 --dir /tmp"
redis-cli ping  # Should return PONG
```

### 2. Start SecureLLM Bridge

```bash
cd ~/arch/securellm-bridge
./start-server.sh
```

Or manually:

```bash
export LOG_DIR=~/arch/securellm-bridge/logs
export DATABASE_URL=sqlite:~/arch/securellm-bridge/data/models.db
export REDIS_URL=redis://localhost:6379
export SERVER_HOST=0.0.0.0
export SERVER_PORT=8080
export LLAMACPP_ENABLED=true
export LLAMACPP_BASE_URL=http://localhost:5001
export LLAMACPP_MODEL_NAME=local-model

cd ~/arch/securellm-bridge
cargo run --release --bin securellm-api-server
```

### 3. Verify Server is Running

```bash
curl http://localhost:8080/api/health | jq .
```

Expected output:
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "providers": [
    {
      "name": "llamacpp",
      "status": "healthy",
      "circuit_breaker": "closed"
    }
  ],
  "database": {"status": "healthy"},
  "redis": {"status": "healthy"}
}
```

### 4. Start OpenWebUI

```bash
cd ~/arch/docker-hub/ml-clusters/kits/ai-suite/
docker-compose up -d open-webui
```

Access OpenWebUI at: `http://localhost:3001`

## Configuration

### SecureLLM Bridge Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `SERVER_HOST` | Bind address | `0.0.0.0` |
| `SERVER_PORT` | Server port | `8080` |
| `DATABASE_URL` | SQLite database path | `sqlite:./data/models.db` |
| `REDIS_URL` | Redis connection string | `redis://localhost:6379` |
| `LOG_DIR` | Log directory | `./logs` |
| `LOG_LEVEL` | Log verbosity | `info` |

### Provider Configuration

#### LlamaCpp (Local)
```bash
export LLAMACPP_ENABLED=true
export LLAMACPP_BASE_URL=http://localhost:5001
export LLAMACPP_MODEL_NAME=local-model
```

#### DeepSeek
```bash
export DEEPSEEK_ENABLED=true
export DEEPSEEK_API_KEY=sk-your-key-here
```

#### OpenAI
```bash
export OPENAI_ENABLED=true
export OPENAI_API_KEY=sk-your-key-here
```

#### Anthropic (Claude)
```bash
export ANTHROPIC_ENABLED=true
export ANTHROPIC_API_KEY=sk-ant-your-key-here
```

#### Groq
```bash
export GROQ_ENABLED=true
export GROQ_API_KEY=gsk-your-key-here
```

#### Gemini
```bash
export GEMINI_ENABLED=true
export GEMINI_API_KEY=your-key-here
```

#### NVIDIA
```bash
export NVIDIA_ENABLED=true
export NVIDIA_API_KEY=nvapi-your-key-here
```

## OpenWebUI Configuration

The docker-compose.yml has been updated to use SecureLLM Bridge:

```yaml
environment:
  - OPENAI_API_BASE_URL=http://host.docker.internal:8080/v1
  - OPENAI_API_KEY=not-needed
```

## API Endpoints

### Health Check
```bash
GET /api/health
```

### List Models
```bash
GET /v1/models
```

### Chat Completions
```bash
POST /v1/chat/completions
Content-Type: application/json

{
  "model": "auto",  // or "llamacpp/local-model", "deepseek/deepseek-chat", etc.
  "messages": [
    {"role": "user", "content": "Hello!"}
  ]
}
```

## Smart Routing

When using `"model": "auto"`, SecureLLM Bridge automatically selects the best provider:

1. **LlamaCpp first** - Free local inference
2. **DeepSeek** - Cheapest cloud option (~$0.0001/1k tokens)
3. **Groq** - Fast inference with competitive pricing
4. **OpenAI** - Industry standard, high reliability
5. **Anthropic** - Premium quality, final fallback

If a provider fails, the bridge automatically tries the next one in the chain.

## Monitoring

### Server Logs
```bash
tail -f ~/arch/securellm-bridge/logs/server.log
```

### Audit Logs
```bash
tail -f ~/arch/securellm-bridge/logs/audit.2026-02-01.log
```

### Redis Status
```bash
redis-cli ping
redis-cli info stats
```

## Troubleshooting

### Server won't start

**Problem**: `Failed to create log directory`
**Solution**: Use absolute paths or set `LOG_DIR` environment variable

**Problem**: `Failed to connect to database`
**Solution**: Ensure `data/` directory exists and is writable

**Problem**: `Failed to create Redis pool`
**Solution**: Start Redis with `redis-server --daemonize yes`

### OpenWebUI can't connect

**Problem**: `Connection refused to host.docker.internal:8080`
**Solution**:
1. Verify SecureLLM Bridge is running: `curl http://localhost:8080/api/health`
2. Check Docker networking: `docker exec open-webui-suite ping host.docker.internal`

### No models available

**Problem**: `/v1/models` returns empty list
**Solution**: Enable at least one provider (e.g., `LLAMACPP_ENABLED=true`)

## Testing

### Test Health Endpoint
```bash
curl http://localhost:8080/api/health | jq .
```

### Test Chat Completion
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "auto",
    "messages": [{"role": "user", "content": "Hello, how are you?"}]
  }' | jq .
```

### Test from OpenWebUI Container
```bash
docker exec open-webui-suite curl -s http://host.docker.internal:8080/api/health
```

## Performance

Typical response times:
- **LlamaCpp (local)**: 200-500ms
- **DeepSeek**: 500-1000ms
- **Groq**: 300-800ms (fast inference)
- **OpenAI GPT-4**: 1000-3000ms
- **Anthropic Claude**: 800-2000ms

## Security

- **Rate Limiting**: Enabled per provider with circuit breakers
- **Audit Logging**: All requests logged to `logs/audit.YYYY-MM-DD.log`
- **API Keys**: Stored securely using `secrecy` crate
- **No Auth Required**: For local development (set `REQUIRE_AUTH=true` for production)

## Next Steps

1. **Add API Keys**: Configure cloud providers in `start-server.sh`
2. **Test Fallback**: Disable LlamaCpp to test cloud provider routing
3. **Monitor Usage**: Check audit logs for token usage and costs
4. **Production Setup**: Enable authentication and configure NixOS service

## Support

- **Logs**: `~/arch/securellm-bridge/logs/`
- **Database**: `~/arch/securellm-bridge/data/models.db`
- **Configuration**: `~/arch/securellm-bridge/crates/api-server/.env`
- **OpenWebUI**: `~/arch/docker-hub/ml-clusters/kits/ai-suite/docker-compose.yml`
