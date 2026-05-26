# Quick Test & Deployment Guide

## 🔨 Orquestração Completa

Veja [../../ORCHESTRATION.md](../../ORCHESTRATION.md) para:
- Docker Compose setup
- Nix develop mode
- Monitoramento e logs
- Deploy em K8s/Cloud Run

## ✅ Quick Start

### Via Script (Recomendado)

```bash
cd /home/kernelcore/master

# Docker Compose (tudo junto)
./securellm-orchestrate.sh build
./securellm-orchestrate.sh start
./securellm-orchestrate.sh test
./securellm-orchestrate.sh logs gateway

# Gerenciar
./securellm-orchestrate.sh status
./securellm-orchestrate.sh stop
./securellm-orchestrate.sh clean
```

### Via Docker Compose (Manual)

```bash
cd /home/kernelcore/master

# Build + start
docker-compose up -d

# Test
curl http://localhost:3000/softwares | jq .
curl http://localhost:3000/softwares/spider-nix | jq .data
curl http://localhost:3000/softwares/category/security | jq .

# Logs
docker-compose logs -f gateway

# Stop
docker-compose down
```

### Via Nix (Local Development)

Ver [../../ORCHESTRATION.md](../../ORCHESTRATION.md#via-nix-desenvolvimento-local)

## 📋 Endpoints Disponíveis

| Endpoint | Método | Autenticação | Descrição |
|----------|--------|--------------|-----------|
| `/softwares` | GET | ❌ Não | Lista todos os softwares |
| `/softwares/{name}` | GET | ❌ Não | Metadados de um software |
| `/softwares/category/{cat}` | GET | ❌ Não | Filtra por categoria |
| `/mcp` | * | ✅ Sim | MCP privado (autenticação OAuth/Bearer) |

## 🚀 Deployment

### Registrar no manifesto público (securellm-mcp)
```bash
# Usar .mcp.public.json gerado
cp /home/kernelcore/master/securellm-mcp/.mcp.public.json /publish/
```

### Docker (produção)
```bash
# Build Docker image
docker build -f docker/gateway.Dockerfile -t voidnxlabs/securellm-gateway:latest .

# Publicar
docker push voidnxlabs/securellm-gateway:latest

# Deploy
docker run -p 80:3000 voidnxlabs/securellm-gateway:latest
```

### Kubernetes (produção)
```yaml
apiVersion: v1
kind: Service
metadata:
  name: securellm-gateway
spec:
  selector:
    app: securellm-gateway
  ports:
    - name: http
      port: 80
      targetPort: 3000
  type: LoadBalancer
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: securellm-gateway
spec:
  replicas: 3
  selector:
    matchLabels:
      app: securellm-gateway
  template:
    metadata:
      labels:
        app: securellm-gateway
    spec:
      containers:
      - name: gateway
        image: voidnxlabs/securellm-gateway:latest
        ports:
        - containerPort: 3000
        env:
        - name: LISTEN_ADDR
          value: "0.0.0.0:3000"
```

## 🔒 Segurança em Produção

- ✅ Endpoints `/softwares/*` são públicos (cache em CDN)
- ✅ Endpoint `/mcp` requer autenticação (OAuth com PKCE)
- ✅ Rate limiting automático no gateway
- ✅ Audit trail em `$LOG_DIR` (sops-encrypted)

## 📚 Arquivos Criados

- `crates/gateway/src/softwares.rs` — registry
- `crates/gateway/src/routes.rs` — HTTP routes
- `crates/gateway/src/lib.rs` — adicionado mod routes
- `crates/gateway/src/transport.rs` — integrado no router
- `securellm-mcp/.mcp.public.json` — manifesto público
- `securellm-mcp/flake-consumer.nix` — flake para consumir

## 🎯 Próximos Passos

1. [ ] Esperar build compilar
2. [ ] Testar endpoints localmente
3. [ ] Integrar com CI/CD (GitHub Actions)
4. [ ] Deploy em staging (api-staging.voidnxlabs.io)
5. [ ] Deploy em produção (api.voidnxlabs.io)
6. [ ] Publicar flake no flake.parts/voidnxlabs-softwares
