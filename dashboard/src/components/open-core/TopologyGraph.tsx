import { motion } from 'framer-motion'
import { providerRuntime } from '@/lib/open-core-data'

export function TopologyGraph() {
  const providers = providerRuntime.slice(0, 4)

  return (
    <section className="relative min-h-[360px] overflow-hidden rounded-lg border border-[#1A2236] bg-[#0A0E1A] p-6">
      <div className="mb-4 flex items-center justify-between">
        <div>
          <h2 className="font-display text-lg font-semibold text-[#E2E8F0]">Provider Topology</h2>
          <p className="text-sm text-[#94A3B8]">Token flow from MCP clients through SecureLLM Bridge</p>
        </div>
        <div className="font-mono text-xs text-[#06B6D4]">live / 4 edges</div>
      </div>

      <div className="relative h-[270px]">
        <svg className="absolute inset-0 h-full w-full" viewBox="0 0 720 270" role="img" aria-label="Provider topology graph">
          <defs>
            <linearGradient id="edge" x1="0%" y1="0%" x2="100%" y2="0%">
              <stop offset="0%" stopColor="#06B6D4" stopOpacity="0.15" />
              <stop offset="50%" stopColor="#6366F1" stopOpacity="0.8" />
              <stop offset="100%" stopColor="#A855F7" stopOpacity="0.15" />
            </linearGradient>
          </defs>
          <path d="M100 135 C230 30 340 30 360 135" stroke="url(#edge)" strokeWidth="2" fill="none" strokeDasharray="7 8" />
          <path d="M100 135 C230 90 300 95 360 135" stroke="url(#edge)" strokeWidth="2" fill="none" strokeDasharray="7 8" />
          <path d="M360 135 C450 80 530 55 620 62" stroke="url(#edge)" strokeWidth="2" fill="none" strokeDasharray="7 8" />
          <path d="M360 135 C470 128 540 132 620 135" stroke="url(#edge)" strokeWidth="2" fill="none" strokeDasharray="7 8" />
          <path d="M360 135 C450 188 530 215 620 208" stroke="url(#edge)" strokeWidth="2" fill="none" strokeDasharray="7 8" />
        </svg>

        <Node x="left-[8%]" y="top-[43%]" label="MCP Clients" sublabel="Claude / Codex / Remote agents" color="#06B6D4" />
        <Node x="left-[45%]" y="top-[38%]" label="SecureLLM Bridge" sublabel="routing + audit + SSE" color="#6366F1" large />
        {providers.map((provider, index) => (
          <Node
            key={provider.id}
            x="left-[82%]"
            y={['top-[9%]', 'top-[36%]', 'top-[63%]', 'top-[78%]'][index]}
            label={provider.name}
            sublabel={provider.model}
            color={provider.color}
          />
        ))}
      </div>
    </section>
  )
}

function Node({
  x,
  y,
  label,
  sublabel,
  color,
  large,
}: {
  x: string
  y: string
  label: string
  sublabel: string
  color: string
  large?: boolean
}) {
  return (
    <motion.div
      initial={{ scale: 0.9, opacity: 0 }}
      animate={{ scale: 1, opacity: 1 }}
      transition={{ duration: 0.35 }}
      className={`absolute ${x} ${y} max-w-[170px] -translate-x-1/2 rounded-lg border border-white/10 bg-[#111827]/95 p-3 shadow-[0_0_22px_rgba(0,0,0,0.25)]`}
    >
      <div className="flex items-center gap-2">
        <span
          className={large ? 'h-3 w-3 rounded-full animate-pulse' : 'h-2.5 w-2.5 rounded-full'}
          style={{ backgroundColor: color, boxShadow: `0 0 18px ${color}` }}
        />
        <span className="truncate font-display text-sm font-semibold text-[#E2E8F0]">{label}</span>
      </div>
      <p className="mt-1 truncate font-mono text-[11px] text-[#94A3B8]">{sublabel}</p>
    </motion.div>
  )
}
