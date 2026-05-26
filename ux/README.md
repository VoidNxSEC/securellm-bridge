# SecureLLM Bridge — UX Design Mode

> Declarative UX specifications → AI agents → Production components

## Overview

The **UX Design Mode** transforms UI development from vague prompts into precise, validated specifications. AI agents receive complete design context — colors, typography, layout, animation — and generate components that follow the Bridge design system exactly.

```
┌──────────────────────┐     ┌─────────────────────┐     ┌──────────────────┐
│  UX Spec (YAML)      │ ──▶ │  MCP Tools           │ ──▶ │  AI Agent        │
│  ux/specs/*.yml      │     │  ux_list_specs       │     │  (Claude/Cline)  │
│                       │     │  ux_get_spec         │     │                  │
│  Colors, fonts,       │     │  ux_generate_prompt  │     │  Generates:      │
│  layout, animation,   │     │  ux_validate         │     │  React + TS +    │
│  components           │     │  ux_design_system    │     │  Tailwind        │
└──────────────────────┘     └─────────────────────┘     └──────────────────┘
```

## MCP Tools

| Tool | Description |
|------|-------------|
| `ux_list_specs` | List all available UX specs |
| `ux_get_spec` | Get a full UX spec (yaml/json/markdown/prompt) |
| `ux_generate_prompt` | Generate optimized agent prompt for a component |
| `ux_validate_component` | Validate generated code against spec |
| `ux_design_system` | Get design system reference (colors/typography/layout) |
| `ux_create_spec` | Create a new UX spec from template |

## UX Specs

| Spec | Purpose |
|------|---------|
| `bridge-gateway` | Main provider management dashboard |
| `bridge-security` | Security panel (TLS, rate-limits, audit) |

## Quickstart

### Use with Agent

```
Agent: "Create a provider status card for the SecureLLM Bridge dashboard"

→ Agent calls ux_design_system("colors")
→ Agent calls ux_generate_prompt("bridge-gateway", "provider_card")
→ Agent generates ProviderCard.tsx following exact spec
→ Agent calls ux_validate_component("bridge-gateway", "provider_card", code)
```

### Create a New Spec

```bash
# Via MCP tool:
ux_create_spec("my-new-view", "Description of what this view does")

# Or manually:
cp ux/specs/bridge-gateway.yml ux/specs/my-new-view.yml
vim ux/specs/my-new-view.yml
```

## Integration

Add to `securellm-mcp` server:

```typescript
import { getUxTools, handleUxTool } from './ux/tools/bridge-ux-server';

// Register tools
const uxTools = getUxTools();
for (const tool of uxTools) {
  server.registerTool(tool, handleUxTool);
}
```

## Design Philosophy

1. **Declarative over imperative**: Specs are YAML, version-controlled, reviewable
2. **Agent-native**: Prompts include exact values, not vague descriptions
3. **Validatable**: Every component can be checked against spec automatically
4. **Reusable**: Specs inherit from base (e.g., `extends: bridge-gateway`)
5. **Bridge identity**: Industrial precision, dark mode, JetBrains Mono, Space Grotesk

## Anti-Patterns the Spec Prevents

❌ "Make it look modern" (vague)
❌ Purple gradients on white (generic AI slop)
❌ Inter/Roboto defaults (boring)
❌ Centered hero sections (templates)
❌ No animation budget (static feels dead)

✅ "Use #0A0E1A background, Space Grotesk headers, indigo accents, staggered reveal at 0.08s"
