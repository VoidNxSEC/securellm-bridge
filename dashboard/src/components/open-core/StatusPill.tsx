import { CheckCircle2, AlertTriangle, XCircle } from 'lucide-react'
import { cn } from '@/lib/utils'

interface StatusPillProps {
  status: 'online' | 'degraded' | 'offline' | 'active' | 'hardened' | 'ok' | 'warn' | 'blocked'
}

const statusConfig = {
  online: { label: 'Online', icon: CheckCircle2, className: 'border-[#22C55E]/40 bg-[#22C55E]/10 text-[#86EFAC]' },
  active: { label: 'Active', icon: CheckCircle2, className: 'border-[#22C55E]/40 bg-[#22C55E]/10 text-[#86EFAC]' },
  ok: { label: 'OK', icon: CheckCircle2, className: 'border-[#22C55E]/40 bg-[#22C55E]/10 text-[#86EFAC]' },
  degraded: { label: 'Degraded', icon: AlertTriangle, className: 'border-[#F59E0B]/40 bg-[#F59E0B]/10 text-[#FCD34D]' },
  warn: { label: 'Warning', icon: AlertTriangle, className: 'border-[#F59E0B]/40 bg-[#F59E0B]/10 text-[#FCD34D]' },
  hardened: { label: 'Hardened', icon: CheckCircle2, className: 'border-[#06B6D4]/40 bg-[#06B6D4]/10 text-[#67E8F9]' },
  offline: { label: 'Offline', icon: XCircle, className: 'border-[#EF4444]/40 bg-[#EF4444]/10 text-[#FCA5A5]' },
  blocked: { label: 'Blocked', icon: XCircle, className: 'border-[#EF4444]/40 bg-[#EF4444]/10 text-[#FCA5A5]' },
}

export function StatusPill({ status }: StatusPillProps) {
  const config = statusConfig[status]
  const Icon = config.icon

  return (
    <span className={cn('inline-flex items-center gap-1 rounded-md border px-2 py-1 text-xs font-medium', config.className)}>
      <Icon className="h-3 w-3" />
      {config.label}
    </span>
  )
}
