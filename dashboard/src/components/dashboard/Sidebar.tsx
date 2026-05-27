import { NavLink } from "react-router-dom";
import { motion } from "framer-motion";
import {
  Brain,
  LayoutDashboard,
  Network,
  FolderKanban,
  Search,
  FileText,
  Settings,
  Shield,
  Radio,
  Users,
  Globe,
  Code,
  LockKeyhole,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { useStatus } from "@/hooks/useApi";

const navItems = [
  {
    title: "Open Core",
    href: "/",
    icon: LayoutDashboard,
  },
  {
    title: "Gateway",
    href: "/gateway",
    icon: Network,
  },
  {
    title: "Security",
    href: "/security",
    icon: LockKeyhole,
  },
  {
    title: "Ecosystem",
    href: "/ecosystem",
    icon: LayoutDashboard,
  },
  {
    title: "Projects",
    href: "/projects",
    icon: FolderKanban,
  },
  {
    title: "Intelligence",
    href: "/intelligence",
    icon: Search,
  },
  {
    title: "Briefing",
    href: "/briefing",
    icon: FileText,
  },
  {
    title: "Settings",
    href: "/settings",
    icon: Settings,
  },
];

const intelTypes = [
  { name: "SIGINT", icon: Radio, color: "text-amber-500" },
  { name: "HUMINT", icon: Users, color: "text-green-500" },
  { name: "OSINT", icon: Globe, color: "text-blue-500" },
  { name: "TECHINT", icon: Code, color: "text-violet-500" },
];

export function Sidebar() {
  const { data: status } = useStatus();

  return (
    <div className="flex h-full flex-col">
      {/* Logo */}
      <div className="flex h-16 items-center gap-3 border-b border-[#1A2236] px-6">
        <motion.div
          animate={{ rotate: [0, 360] }}
          transition={{ duration: 20, repeat: Infinity, ease: "linear" }}
        >
          <Brain className="h-8 w-8 text-bridge-primary" />
        </motion.div>
        <div>
          <h1 className="font-display text-xl font-bold tracking-tight text-[#E2E8F0]">Bridge</h1>
          <p className="text-xs text-[#94A3B8]">MCP + LLM Gateway</p>
        </div>
      </div>

      {/* Status Indicator */}
      <div className="border-b border-[#1A2236] p-4">
        <div className="flex items-center justify-between text-sm">
          <span className="text-[#94A3B8]">System Status</span>
          <div className="flex items-center gap-2">
            <span
              className={cn(
                "h-2 w-2 rounded-full",
                status ? "bg-green-500 animate-pulse" : "bg-red-500",
              )}
            />
            <span className={status ? "text-green-500" : "text-red-500"}>
              {status ? "Online" : "Offline"}
            </span>
          </div>
        </div>
        {status && (
          <div className="mt-2 grid grid-cols-2 gap-2 text-xs">
            <div className="rounded bg-[#111827] p-2">
              <div className="text-[#64748B]">Projects</div>
              <div className="font-mono text-lg font-semibold text-[#E2E8F0]">
                {status.total_projects}
              </div>
            </div>
            <div className="rounded bg-[#111827] p-2">
              <div className="text-[#64748B]">Health</div>
              <div className="font-mono text-lg font-semibold text-[#E2E8F0]">
                {status.health_score.toFixed(0)}%
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Navigation */}
      <nav className="flex-1 space-y-1 p-4">
        <div className="mb-2 text-xs font-semibold uppercase text-[#64748B]">
          Navigation
        </div>
        {navItems.map((item) => (
          <NavLink
            key={item.href}
            to={item.href}
            className={({ isActive }) =>
              cn(
                "flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors",
                isActive
                  ? "bg-[#6366F1] text-white"
                  : "text-[#94A3B8] hover:bg-[#111827] hover:text-[#E2E8F0]",
              )
            }
          >
            <item.icon className="h-4 w-4" />
            {item.title}
          </NavLink>
        ))}

        {/* Intel Types Section */}
        <div className="mt-6 mb-2 text-xs font-semibold uppercase text-[#64748B]">
          Intelligence Types
        </div>
        {intelTypes.map((type) => (
          <div
            key={type.name}
            className="flex items-center gap-3 rounded-lg px-3 py-2 text-sm"
          >
            <type.icon className={cn("h-4 w-4", type.color)} />
            <span className="text-[#94A3B8]">{type.name}</span>
          </div>
        ))}
      </nav>

      {/* Footer */}
      <div className="border-t border-[#1A2236] p-4">
        <div className="flex items-center gap-2 text-xs text-[#94A3B8]">
          <Shield className="h-4 w-4" />
          <span>Classification: INTERNAL</span>
        </div>
        <div className="mt-1 text-xs text-[#64748B]">
          v0.1.0 | SecureLLM Bridge
        </div>
      </div>
    </div>
  );
}
