# VoidNxLabs Public API — Software Registry

Arquitetura segura para expor softwares voidnxlabs publicamente **sem comprometer metadados do servidor**.

## 🏗️ Arquitetura

```
┌──────────────────────────┐
│  Clientes Externos       │  Nix, Linux, CI/CD
│  (Públicos)              │
└────────────┬─────────────┘
             │ HTTPS
             ▼
┌──────────────────────────────────────────┐
│  SecureLLM Bridge Gateway (Público)      │
│  ✓ OAuth/JWT (opcional)                  │
│  ✓ Rate limiting                         │
│  ✓ CORS configurado                      │
│                                          │
│  GET /softwares              ← lista     │
│  GET /softwares/{name}       ← metadados │
│  GET /softwares/category/:cat ← filtro   │
└────────────┬─────────────────────────────┘
             │ localhost (privado)
             ▼
┌──────────────────────────┐
│  MCP Server (Privado)    │
│  No host/port exposed    │
└──────────────────────────┘
```

## 📋 Que dados são expostos?

**Público** (via `/softwares`):
```json
{
  "name": "spider-nix",
  "version": "0.1.0",
  "description": "Domain reconnaissance toolkit",
  "category": "security",
  "repository": "https://github.com/voidnxlabs/spider-nix",
  "homepage": "https://voidnxlabs.io",
  "license": "Apache-2.0",
  "maintainers": ["voidnxlabs <dev@voidnxlabs.io>"],
  "source_types": ["nix", "cargo"],
  "source_url": "https://github.com/voidnxlabs/spider-nix"
}
```

**Nunca exposto:**
- Host/porta do servidor MCP
- Chaves de API internas
- Tokens de autenticação
- Configurações privadas

## 🚀 Consumir via Nix

### Opção 1: Via flake do bridge
```bash
nix flake show github:voidnxlabs/securellm-bridge#

# Instalar um software
nix run github:voidnxlabs/securellm-bridge#spider-nix -- --version
```

### Opção 2: Consultar API diretamente
```bash
# Listar softwares
curl https://api.voidnxlabs.io/softwares | jq .

# Info específico
curl https://api.voidnxlabs.io/softwares/spider-nix | jq .data

# Por categoria
curl https://api.voidnxlabs.io/softwares/category/security | jq .
```

### Opção 3: Usar o cliente Nix (convenience script)
```bash
nix run github:voidnxlabs/securellm-bridge#registry-client -- list
nix run github:voidnxlabs/securellm-bridge#registry-client -- info spider-nix
```

## 💻 Consumir via Linux

### Via curl + package manager
```bash
# Descobrir URL do repositório
SOURCE_URL=$(curl -s https://api.voidnxlabs.io/softwares/spider-nix | \
  jq -r '.data.source_url')

# Clonar e buildar
git clone $SOURCE_URL
cd spider-nix
cargo build --release
sudo install target/release/spider-nix /usr/local/bin/
```

### Via repositório de pacotes (apt/dnf/pacman)
```bash
# Ubuntu/Debian
sudo add-apt-repository ppa:voidnxlabs/ppa
sudo apt install spider-nix cerebro phantom

# Fedora
sudo dnf copr enable voidnxlabs/releases
sudo dnf install spider-nix cerebro phantom

# Arch
yay -S spider-nix cerebro phantom
```

## 🔒 Privacidade & Segurança

- ✅ **Zero exposição de infraestrutura**: Host/porta/chaves nunca no manifesto
- ✅ **Separação público/privado**: `.mcp.json` vs `.mcp.private.json` (encriptado com sops)
- ✅ **Descoberta descentralizada**: Clientes consultam API pública, não arquivo de configuração
- ✅ **Autenticação no gateway**: Apenas clientes autenticados alcançam MCP privado
- ✅ **Rate limiting**: Proteção contra abuso

## 📚 Configuração no Bridge

### 1. Adicionar módulo softwares
```bash
# Já criado em: crates/gateway/src/softwares.rs
# Já adicionado em: crates/gateway/src/lib.rs
```

### 2. Registrar rotas no main.rs do gateway
```rust
use crate::routes::{software_routes, AppState};
use crate::softwares::SoftwareRegistry;

#[tokio::main]
async fn main() {
    let registry = SoftwareRegistry::new();
    let state = AppState { registry };
    
    let app = Router::new()
        .nest("/", software_routes().with_state(state.clone()))
        .layer(/* auth middleware */);
    
    // Iniciar servidor...
}
```

### 3. Testar localmente
```bash
# Buildar bridge
cd securellm-bridge
cargo build

# Rodar gateway
cargo run --bin gateway

# Testar
curl http://localhost:3000/softwares | jq .
curl http://localhost:3000/softwares/spider-nix | jq .data
```

## 🎯 Próximos Passos

- [ ] Integrar com GitHub Actions para publicar metadados automaticamente
- [ ] Adicionar versioning e histórico de releases
- [ ] Implementar webhook para sops-decrypt durante build
- [ ] Cache CDN para `/softwares` (imutável)
- [ ] Signar metadados com chave privada (provenance)

## 📞 Contato

`dev@voidnxlabs.io`
