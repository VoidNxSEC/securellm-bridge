import { Activity, Filter, Server, SlidersHorizontal } from 'lucide-react'
import { ProviderCard } from '@/components/open-core/ProviderCard'
import { TopologyGraph } from '@/components/open-core/TopologyGraph'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { providerRuntime, requestLog } from '@/lib/open-core-data'

export function Gateway() {
  return (
    <div className="space-y-6">
      <header className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
        <div>
          <div className="mb-2 flex items-center gap-2 font-mono text-xs uppercase tracking-[0.18em] text-[#06B6D4]">
            <Server className="h-4 w-4" />
            Provider Gateway
          </div>
          <h1 className="font-display text-3xl font-semibold text-[#E2E8F0]">Routing, Streaming, Fallback</h1>
          <p className="mt-2 max-w-3xl text-sm text-[#94A3B8]">
            Operational view for OpenAI-compatible routing across SecureLLM Bridge providers.
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button variant="outline" className="border-[#1A2236] bg-transparent text-[#E2E8F0] hover:bg-[#1A2236]">
            <Filter className="mr-2 h-4 w-4" />
            Filter
          </Button>
          <Button className="bg-[#6366F1] text-white hover:bg-[#4F46E5]">
            <SlidersHorizontal className="mr-2 h-4 w-4" />
            Routing Policy
          </Button>
        </div>
      </header>

      <div className="grid gap-4 lg:grid-cols-2 xl:grid-cols-3">
        {providerRuntime.map((provider, index) => (
          <ProviderCard key={provider.id} provider={provider} index={index} />
        ))}
      </div>

      <div className="grid gap-6 xl:grid-cols-[1.2fr_0.8fr]">
        <TopologyGraph />
        <Card className="border-[#1A2236] bg-[#111827]">
          <CardHeader className="flex flex-row items-center justify-between">
            <CardTitle className="font-display text-lg text-[#E2E8F0]">Request Log</CardTitle>
            <Activity className="h-4 w-4 text-[#06B6D4]" />
          </CardHeader>
          <CardContent>
            <div className="space-y-2">
              {requestLog.map((entry) => (
                <div key={`${entry.time}-${entry.provider}`} className="grid grid-cols-[72px_1fr_64px_54px] items-center gap-3 rounded-md border border-[#1A2236] bg-[#0A0E1A] px-3 py-2 font-mono text-xs">
                  <span className="text-[#64748B]">{entry.time}</span>
                  <div className="min-w-0">
                    <div className="truncate text-[#E2E8F0]">{entry.provider}</div>
                    <div className="truncate text-[11px] text-[#64748B]">{entry.model}</div>
                  </div>
                  <span className="text-right text-[#94A3B8]">{entry.latency || '-'}ms</span>
                  <span className={entry.status < 300 ? 'text-[#22C55E]' : entry.status < 500 ? 'text-[#F59E0B]' : 'text-[#EF4444]'}>
                    {entry.status}
                  </span>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  )
}
