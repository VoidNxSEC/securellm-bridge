import type {
  EcosystemStatus,
  Project,
  ProjectAnalysis,
  QueryResult,
  Briefing,
  Alert,
  DependencyGraph,
  IntelligenceType,
  IntelligenceStats,
  ScanResult,
} from '@/types'

const API_BASE = '/api'

class ApiClient {
  private async request<T>(endpoint: string, options?: RequestInit): Promise<T> {
    const response = await fetch(`${API_BASE}${endpoint}`, {
      ...options,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
      },
    })

    if (!response.ok) {
      throw new Error(`API Error: ${response.status} ${response.statusText}`)
    }

    return response.json()
  }

  // Status
  async getStatus(): Promise<EcosystemStatus> {
    return this.request('/status')
  }

  // Projects
  async getProjects(params?: {
    status?: string
    language?: string
    sort_by?: string
    order?: 'asc' | 'desc'
  }): Promise<Project[]> {
    const query = new URLSearchParams()
    if (params?.status) query.set('status', params.status)
    if (params?.language) query.set('language', params.language)
    if (params?.sort_by) query.set('sort_by', params.sort_by)
    if (params?.order) query.set('order', params.order)

    const queryString = query.toString()
    return this.request(`/projects${queryString ? `?${queryString}` : ''}`)
  }

  async getProject(name: string): Promise<Project> {
    return this.request(`/projects/${encodeURIComponent(name)}`)
  }

  async summarizeProject(name: string): Promise<ProjectAnalysis> {
    return this.request(`/projects/${encodeURIComponent(name)}/summarize`, {
      method: 'POST',
    })
  }

  // Intelligence
  async queryIntelligence(params: {
    query: string
    types?: IntelligenceType[]
    projects?: string[]
    limit?: number
    semantic?: boolean
  }): Promise<QueryResult> {
    const query = new URLSearchParams()
    query.set('q', params.query)
    if (params.types?.length) query.set('types', params.types.join(','))
    if (params.projects?.length) query.set('projects', params.projects.join(','))
    if (params.limit) query.set('limit', params.limit.toString())
    if (params.semantic !== undefined) query.set('semantic', params.semantic.toString())

    return this.request(`/intelligence/query?${query.toString()}`)
  }

  async getIntelligenceStats(): Promise<IntelligenceStats> {
    return this.request('/intelligence/stats')
  }

  // Briefings
  async getDailyBriefing(): Promise<Briefing> {
    return this.request('/briefing/daily')
  }

  async getExecutiveBriefing(): Promise<Briefing> {
    return this.request('/briefing/executive')
  }

  // Alerts
  async getAlerts(): Promise<Alert[]> {
    return this.request('/alerts')
  }

  // Graph
  async getDependencyGraph(): Promise<DependencyGraph> {
    return this.request('/graph/dependencies')
  }

  // Actions
  async triggerScan(params?: { full_scan?: boolean; collect_intelligence?: boolean }): Promise<ScanResult> {
    return this.request('/scan', {
      method: 'POST',
      body: params ? JSON.stringify(params) : undefined,
    })
  }
}

const api = new ApiClient()
export default api
