import { type ClassValue, clsx } from 'clsx'
import { twMerge } from 'tailwind-merge'

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

export function getHealthColor(score: number): string {
  if (score >= 70) return 'text-green-500'
  if (score >= 50) return 'text-yellow-500'
  return 'text-red-500'
}

export function getThreatColor(level: string): string {
  if (level === 'critical') return 'text-red-500'
  if (level === 'high') return 'text-orange-500'
  if (level === 'medium') return 'text-yellow-500'
  if (level === 'low') return 'text-green-500'
  return 'text-blue-500'
}

export function getIntelTypeColor(type: string): string {
  if (type === 'sigint') return 'text-amber-500'
  if (type === 'humint') return 'text-green-500'
  if (type === 'osint') return 'text-blue-500'
  if (type === 'techint') return 'text-violet-500'
  return 'text-muted-foreground'
}

export function formatRelativeTime(dateString: string | null): string {
  if (!dateString) return 'Never'

  const date = new Date(dateString)
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffSec = Math.floor(diffMs / 1000)
  const diffMin = Math.floor(diffSec / 60)
  const diffHr = Math.floor(diffMin / 60)
  const diffDay = Math.floor(diffHr / 24)

  if (diffDay > 0) return `${diffDay}d ago`
  if (diffHr > 0) return `${diffHr}h ago`
  if (diffMin > 0) return `${diffMin}m ago`
  return 'Just now'
}

export function formatDate(dateString: string | null): string {
  if (!dateString) return 'N/A'

  const date = new Date(dateString)
  return date.toLocaleString('en-US', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}

export function formatNumber(num: number): string {
  if (num >= 1000000) {
    return `${(num / 1000000).toFixed(1)}M`
  }
  if (num >= 1000) {
    return `${(num / 1000).toFixed(1)}K`
  }
  return num.toString()
}
