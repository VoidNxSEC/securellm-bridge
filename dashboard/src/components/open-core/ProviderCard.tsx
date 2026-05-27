import { motion } from 'framer-motion'
import { Cpu, Settings, Zap } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Progress } from '@/components/ui/progress'
import { ProviderRuntime } from '@/lib/open-core-data'
import { StatusPill } from './StatusPill'
import { cn } from '@/lib/utils'

const borderByStatus = {
  online: 'border-l-[#22C55E]',
  degraded: 'border-l-[#F59E0B]',
  offline: 'border-l-[#EF4444] opacity-60 grayscale',
}

export function ProviderCard({ provider, index }: { provider: ProviderRuntime; index: number }) {
  return (
    <motion.section
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay: index * 0.06 }}
      className={cn(
        'min-h-[210px] rounded-lg border border-[#1A2236] border-l-[3px] bg-[#111827] p-5 transition duration-200 hover:scale-[1.01] hover:border-[#6366F1]/50 hover:shadow-[0_0_18px_rgba(99,102,241,0.14)]',
        borderByStatus[provider.status]
      )}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="flex min-w-0 items-center gap-3">
          <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-md border border-white/10 bg-[#1A2236]" style={{ color: provider.color }}>
            <Cpu className="h-5 w-5" />
          </div>
          <div className="min-w-0">
            <h3 className="truncate font-display text-base font-semibold text-[#E2E8F0]">{provider.name}</h3>
            <p className="truncate font-mono text-xs text-[#94A3B8]">{provider.model}</p>
          </div>
        </div>
        <StatusPill status={provider.status} />
      </div>

      <div className="mt-5 grid grid-cols-3 gap-3 font-mono text-sm">
        <Metric label="Tokens" value={provider.tokens} />
        <Metric label="Latency" value={provider.status === 'offline' ? '-' : `${provider.latency}ms`} />
        <Metric label="Req" value={provider.requests} />
      </div>

      <div className="mt-5">
        <div className="mb-2 flex items-center justify-between font-mono text-xs text-[#94A3B8]">
          <span>Health</span>
          <span>{provider.health}%</span>
        </div>
        <Progress
          value={provider.health}
          className="h-2 bg-[#1A2236]"
          indicatorClassName={provider.health >= 90 ? 'bg-[#22C55E]' : provider.health >= 70 ? 'bg-[#F59E0B]' : 'bg-[#EF4444]'}
        />
      </div>

      <div className="mt-5 flex gap-2">
        <Button variant="outline" size="sm" className="h-8 border-[#1A2236] bg-transparent text-xs text-[#E2E8F0] hover:bg-[#1A2236]">
          <Zap className="mr-1 h-3 w-3" />
          Test
        </Button>
        <Button variant="outline" size="sm" className="h-8 border-[#1A2236] bg-transparent text-xs text-[#E2E8F0] hover:bg-[#1A2236]">
          <Settings className="mr-1 h-3 w-3" />
          Config
        </Button>
      </div>
    </motion.section>
  )
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <div className="text-[11px] uppercase text-[#64748B]">{label}</div>
      <div className="mt-1 text-[#E2E8F0]">{value}</div>
    </div>
  )
}
