export type ProviderStatus = 'online' | 'degraded' | 'offline'

export interface ProviderRuntime {
  id: string
  name: string
  model: string
  status: ProviderStatus
  tokens: string
  latency: number
  health: number
  color: string
  requests: string
}

export interface RequestLogEntry {
  time: string
  provider: string
  model: string
  tokens: number
  latency: number
  status: number
}

export interface RateBucket {
  provider: string
  used: number
  limit: number
  burst: string
  refill: string
}

export interface AuditEvent {
  time: string
  subject: string
  action: string
  result: 'ok' | 'warn' | 'blocked'
}

export const providerRuntime: ProviderRuntime[] = [
  {
    id: 'openai',
    name: 'OpenAI',
    model: 'gpt-4o-mini',
    status: 'online',
    tokens: '18.4k',
    latency: 231,
    health: 96,
    color: '#10A37F',
    requests: '1.8k',
  },
  {
    id: 'deepseek',
    name: 'DeepSeek',
    model: 'deepseek-chat',
    status: 'online',
    tokens: '31.2k',
    latency: 284,
    health: 91,
    color: '#4F46E5',
    requests: '2.4k',
  },
  {
    id: 'groq',
    name: 'Groq',
    model: 'llama-3.3-70b-versatile',
    status: 'degraded',
    tokens: '9.7k',
    latency: 112,
    health: 78,
    color: '#F97316',
    requests: '840',
  },
  {
    id: 'mlops',
    name: 'ML Ops',
    model: 'local-gpu-router',
    status: 'online',
    tokens: '42.1k',
    latency: 74,
    health: 94,
    color: '#EC4899',
    requests: '3.1k',
  },
  {
    id: 'anthropic',
    name: 'Anthropic',
    model: 'claude-sonnet',
    status: 'offline',
    tokens: '0',
    latency: 0,
    health: 0,
    color: '#D97706',
    requests: '0',
  },
]

export const requestLog: RequestLogEntry[] = [
  { time: '14:32:11', provider: 'openai', model: 'gpt-4o-mini', tokens: 812, latency: 231, status: 200 },
  { time: '14:32:08', provider: 'deepseek', model: 'deepseek-chat', tokens: 1240, latency: 284, status: 200 },
  { time: '14:32:04', provider: 'groq', model: 'llama-3.3', tokens: 540, latency: 112, status: 429 },
  { time: '14:31:59', provider: 'ml-ops', model: 'local-gpu', tokens: 2048, latency: 74, status: 200 },
  { time: '14:31:52', provider: 'anthropic', model: 'sonnet', tokens: 0, latency: 0, status: 503 },
]

export const rateBuckets: RateBucket[] = [
  { provider: 'openai', used: 48, limit: 60, burst: '8/10', refill: '12s' },
  { provider: 'deepseek', used: 36, limit: 60, burst: '6/10', refill: '18s' },
  { provider: 'groq', used: 59, limit: 60, burst: '10/10', refill: '4s' },
  { provider: 'ml-ops', used: 21, limit: 80, burst: '3/8', refill: '31s' },
]

export const auditEvents: AuditEvent[] = [
  { time: '14:32:11', subject: 'bridge-api', action: 'chat.completions stream opened', result: 'ok' },
  { time: '14:32:04', subject: 'groq', action: 'rate limit threshold crossed', result: 'warn' },
  { time: '14:31:52', subject: 'mcp-gateway', action: 'missing bearer token rejected', result: 'blocked' },
  { time: '14:31:44', subject: 'audit-log', action: 'append-only event persisted', result: 'ok' },
]

export const openCoreModules = [
  {
    name: 'securellm-bridge',
    kind: 'Unified LLM API',
    status: 'active',
    detail: 'OpenAI-compatible chat, provider routing, SSE streaming, audit and metrics.',
  },
  {
    name: 'securellm-mcp',
    kind: 'MCP Tooling',
    status: 'active',
    detail: 'Agent tools, UX design MCP surface, knowledge, diagnostics and secure execution.',
  },
  {
    name: 'gateway-mcp',
    kind: 'GitHub Action Gateway',
    status: 'hardened',
    detail: 'Remote agent bridge with OAuth/PKCE, bearer auth, allowlist and PAT zero-leak.',
  },
]
