import { KeyRound, LockKeyhole, RadioTower, ShieldCheck } from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Progress } from '@/components/ui/progress'
import { auditEvents, rateBuckets } from '@/lib/open-core-data'
import { StatusPill } from '@/components/open-core/StatusPill'

export function Security() {
  return (
    <div className="space-y-6">
      <header>
        <div className="mb-2 flex items-center gap-2 font-mono text-xs uppercase tracking-[0.18em] text-[#06B6D4]">
          <ShieldCheck className="h-4 w-4" />
          Security Signal Plane
        </div>
        <h1 className="font-display text-3xl font-semibold text-[#E2E8F0]">Bridge Security + Owasaka Hooks</h1>
        <p className="mt-2 max-w-3xl text-sm text-[#94A3B8]">
          TLS, rate limits, audit trails, sandbox posture, and the planned Owasaka SIEM health/metrics/WebSocket integration.
        </p>
      </header>

      <div className="grid gap-4 md:grid-cols-4">
        <SecurityMetric icon={LockKeyhole} label="TLS" value="TLS 1.3" detail="Owasaka supports HSTS + TLS config" />
        <SecurityMetric icon={KeyRound} label="MCP Auth" value="OAuth" detail="PKCE + bearer token enforcement" />
        <SecurityMetric icon={RadioTower} label="Owasaka" value="Ready" detail="/health, /metrics, WS hub" />
        <SecurityMetric icon={ShieldCheck} label="Audit" value="Append" detail="JSONL today, ledger-ready path" />
      </div>

      <div className="grid gap-6 xl:grid-cols-[0.9fr_1.1fr]">
        <Card className="border-[#1A2236] bg-[#111827]">
          <CardHeader>
            <CardTitle className="font-display text-lg text-[#E2E8F0]">Rate Limit Buckets</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            {rateBuckets.map((bucket) => {
              const pct = Math.round((bucket.used / bucket.limit) * 100)
              return (
                <div key={bucket.provider} className="rounded-md border border-[#1A2236] bg-[#0A0E1A] p-3">
                  <div className="mb-2 flex items-center justify-between font-mono text-xs">
                    <span className="text-[#E2E8F0]">{bucket.provider}</span>
                    <span className={pct > 90 ? 'text-[#F59E0B]' : 'text-[#94A3B8]'}>
                      {bucket.used}/{bucket.limit} req/min
                    </span>
                  </div>
                  <Progress
                    value={pct}
                    className="h-2 bg-[#1A2236]"
                    indicatorClassName={pct > 90 ? 'bg-[#F59E0B]' : 'bg-[#06B6D4]'}
                  />
                  <div className="mt-2 flex justify-between font-mono text-[11px] text-[#64748B]">
                    <span>Burst {bucket.burst}</span>
                    <span>Refill {bucket.refill}</span>
                  </div>
                </div>
              )
            })}
          </CardContent>
        </Card>

        <Card className="border-[#1A2236] bg-[#111827]">
          <CardHeader>
            <CardTitle className="font-display text-lg text-[#E2E8F0]">Audit + SIEM Feed</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            {auditEvents.map((event) => (
              <div key={`${event.time}-${event.action}`} className="grid grid-cols-[76px_120px_1fr_auto] items-center gap-3 rounded-md border border-[#1A2236] bg-[#0A0E1A] px-3 py-2 font-mono text-xs">
                <span className="text-[#64748B]">{event.time}</span>
                <span className="truncate text-[#94A3B8]">{event.subject}</span>
                <span className="truncate text-[#E2E8F0]">{event.action}</span>
                <StatusPill status={event.result} />
              </div>
            ))}
            <div className="rounded-md border border-dashed border-[#06B6D4]/30 bg-[#06B6D4]/5 p-4 text-sm text-[#94A3B8]">
              Owasaka synergy target: consume `/readyz` snapshots, Prometheus `/metrics`, and WebSocket broadcasts as security signals for Bridge and MCP activity.
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  )
}

function SecurityMetric({
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
      <CardContent className="p-4">
        <Icon className="mb-4 h-5 w-5 text-[#06B6D4]" />
        <div className="font-display text-sm text-[#94A3B8]">{label}</div>
        <div className="mt-1 font-mono text-xl font-semibold text-[#E2E8F0]">{value}</div>
        <p className="mt-2 text-xs leading-5 text-[#64748B]">{detail}</p>
      </CardContent>
    </Card>
  )
}
