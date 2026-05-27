import { motion } from 'framer-motion'
import {
  Boxes,
  CheckCircle2,
  GitBranch,
  Radio,
  ShieldCheck,
  Waypoints,
} from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { openCoreModules, providerRuntime } from '@/lib/open-core-data'
import { StatusPill } from '@/components/open-core/StatusPill'
import { TopologyGraph } from '@/components/open-core/TopologyGraph'

export function OpenCore() {
  const activeProviders = providerRuntime.filter((provider) => provider.status !== 'offline').length

  return (
    <div className="space-y-6">
      <header className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
        <div>
          <div className="mb-2 flex items-center gap-2 font-mono text-xs uppercase tracking-[0.18em] text-[#06B6D4]">
            <Boxes className="h-4 w-4" />
            Open Core Control Plane
          </div>
          <h1 className="font-display text-3xl font-semibold text-[#E2E8F0]">SecureLLM MCP + Bridge</h1>
          <p className="mt-2 max-w-3xl text-sm text-[#94A3B8]">
            Unified operator surface for the open core: provider routing, MCP exposure, security telemetry,
            and UX design tooling in one work-focused dashboard.
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button className="bg-[#6366F1] text-white hover:bg-[#4F46E5]">
            <Waypoints className="mr-2 h-4 w-4" />
            Inspect Gateway
          </Button>
          <Button variant="outline" className="border-[#1A2236] bg-transparent text-[#E2E8F0] hover:bg-[#1A2236]">
            <GitBranch className="mr-2 h-4 w-4" />
            View Roadmap
          </Button>
        </div>
      </header>

      <div className="grid gap-4 md:grid-cols-3">
        <KpiCard icon={Radio} label="Streaming Providers" value={activeProviders.toString()} detail="OpenAI-compatible SSE active" />
        <KpiCard icon={ShieldCheck} label="MCP Transport" value="OAuth" detail="PKCE + bearer fallback" />
        <KpiCard icon={CheckCircle2} label="UX Specs" value="2" detail="Gateway and security modes" />
      </div>

      <div className="grid gap-6 xl:grid-cols-[1.45fr_0.9fr]">
        <TopologyGraph />

        <section className="space-y-3">
          {openCoreModules.map((module, index) => (
            <motion.article
              key={module.name}
              initial={{ opacity: 0, x: 12 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{ delay: index * 0.06 }}
              className="rounded-lg border border-[#1A2236] bg-[#111827] p-5"
            >
              <div className="flex items-start justify-between gap-3">
                <div>
                  <h2 className="font-display text-base font-semibold text-[#E2E8F0]">{module.name}</h2>
                  <p className="font-mono text-xs text-[#64748B]">{module.kind}</p>
                </div>
                <StatusPill status={module.status as 'active' | 'hardened'} />
              </div>
              <p className="mt-4 text-sm leading-6 text-[#94A3B8]">{module.detail}</p>
            </motion.article>
          ))}
        </section>
      </div>
    </div>
  )
}

function KpiCard({
  icon: Icon,
  label,
  value,
  detail,
}: {
  icon: React.ElementType
  label: string
  value: string
  detail: string
}) {
  return (
    <Card className="border-[#1A2236] bg-[#111827]">
      <CardHeader className="flex flex-row items-center justify-between pb-2">
        <CardTitle className="font-display text-sm font-medium text-[#94A3B8]">{label}</CardTitle>
        <Icon className="h-4 w-4 text-[#06B6D4]" />
      </CardHeader>
      <CardContent>
        <div className="font-mono text-3xl font-semibold text-[#E2E8F0]">{value}</div>
        <p className="mt-1 text-xs text-[#64748B]">{detail}</p>
      </CardContent>
    </Card>
  )
}
