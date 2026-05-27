// Intelligence Types
export type IntelligenceType = 'sigint' | 'humint' | 'osint' | 'techint'
export type ThreatLevel = 'critical' | 'high' | 'medium' | 'low' | 'info'
export type ProjectStatus = 'active' | 'maintenance' | 'deprecated' | 'archived' | 'unknown'

export interface Project {
  name: string
  path: string
  description: string
  languages: string[]
  status: ProjectStatus
  health_score: number
  last_commit: string | null
  last_indexed: string | null
  metadata: Record<string, unknown>
}

export interface ProjectAnalysis {
  project: string
  timestamp: string
  health_score: number
  status: ProjectStatus
  insights: string[]
  recommendations: string[]
  metrics: {
    health_factors: Record<string, number>
  }
}

export interface IntelligenceStats {
  total: number
  by_type: Record<string, number>
  by_threat: Record<string, number>
  indexer_stats?: {
    model: string
    embedding_dim: number
    indexed_items: number
    index_size_mb?: number
  }
}

export interface ScanResult {
  message: string
  projects_found?: number
  intelligence_collected?: number
  indexed_items?: number
  duration_seconds?: number
}

export interface IntelligenceItem {
  id: string
  type: IntelligenceType
  source: string
  title: string
  content: string
  threat_level: ThreatLevel
  timestamp: string
  tags: string[]
  related_projects: string[]
  score?: number
}

export interface EcosystemStatus {
  total_projects: number
  active_projects: number
  health_score: number
  total_intelligence: number
  alerts_count: number
  last_scan: string | null
}

export interface Alert {
  type: string
  project?: string
  message: string
  score?: number
  id?: string
  title?: string
}

export interface Briefing {
  type: string
  classification: string
  timestamp: string
  summary?: string
  headline?: string
  ecosystem_status?: {
    total_projects: number
    active_projects: number
    health_score: number
  }
  key_developments?: Array<{
    type: string
    title: string
    source: string
    threat_level: ThreatLevel
  }>
  alerts?: Alert[]
  project_summaries?: Array<{
    name: string
    health_score: number
    status: ProjectStatus
    insights: string[]
  }>
}

export interface QueryResult {
  query: string
  results: IntelligenceItem[]
  total: number
  search_type: 'semantic' | 'keyword'
}

export interface DependencyGraph {
  nodes: Array<{ id: string; label: string }>
  edges: Array<{ source: string; target: string }>
}

// Store Types
export interface DashboardState {
  timeRange: TimeRange
  environment: string
  selectedServices: string[]
  autoRefresh: boolean
  refreshInterval: number
  setTimeRange: (range: TimeRange) => void
  setEnvironment: (env: string) => void
  setSelectedServices: (services: string[]) => void
  setAutoRefresh: (enabled: boolean) => void
  setRefreshInterval: (interval: number) => void
}

export type TimeRange = '1h' | '6h' | '24h' | '7d' | '30d' | 'custom'

export interface TimeRangeValue {
  label: string
  value: TimeRange
  hours: number
}

export const TIME_RANGES: TimeRangeValue[] = [
  { label: 'Last Hour', value: '1h', hours: 1 },
  { label: 'Last 6 Hours', value: '6h', hours: 6 },
  { label: 'Last 24 Hours', value: '24h', hours: 24 },
  { label: 'Last 7 Days', value: '7d', hours: 168 },
  { label: 'Last 30 Days', value: '30d', hours: 720 },
]

export const ENVIRONMENTS = ['production', 'staging', 'development', 'all']

export const INTEL_TYPE_CONFIG: Record<IntelligenceType, { label: string; color: string; icon: string }> = {
  sigint: { label: 'Signals', color: 'intel-sigint', icon: 'radio' },
  humint: { label: 'Human', color: 'intel-humint', icon: 'users' },
  osint: { label: 'Open Source', color: 'intel-osint', icon: 'globe' },
  techint: { label: 'Technical', color: 'intel-techint', icon: 'code' },
}

export const THREAT_LEVEL_CONFIG: Record<ThreatLevel, { label: string; color: string }> = {
  critical: { label: 'Critical', color: 'threat-critical' },
  high: { label: 'High', color: 'threat-high' },
  medium: { label: 'Medium', color: 'threat-medium' },
  low: { label: 'Low', color: 'threat-low' },
  info: { label: 'Info', color: 'threat-info' },
}
