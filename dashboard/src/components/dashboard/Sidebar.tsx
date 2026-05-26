import { NavLink } from "react-router-dom";
import { motion } from "framer-motion";
import {
  Brain,
  LayoutDashboard,
  FolderKanban,
  Search,
  FileText,
  Settings,
  Activity,
  Shield,
  Radio,
  Users,
  Globe,
  Code,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { useStatus } from "@/hooks/useApi";

const navItems = [
  {
    title: "Dashboard",
    href: "/",
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
      <div className="flex h-16 items-center gap-3 border-b px-6">
        <motion.div
          animate={{ rotate: [0, 360] }}
          transition={{ duration: 20, repeat: Infinity, ease: "linear" }}
        >
          <Brain className="h-8 w-8 text-bridge-primary" />
        </motion.div>
        <div>
          <h1 className="text-xl font-bold tracking-tight">Bridge</h1>
          <p className="text-xs text-muted-foreground">LLM Gateway</p>
        </div>
      </div>

      {/* Status Indicator */}
      <div className="border-b p-4">
        <div className="flex items-center justify-between text-sm">
          <span className="text-muted-foreground">System Status</span>
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
            <div className="rounded bg-muted/50 p-2">
              <div className="text-muted-foreground">Projects</div>
              <div className="text-lg font-semibold">
                {status.total_projects}
              </div>
            </div>
            <div className="rounded bg-muted/50 p-2">
              <div className="text-muted-foreground">Health</div>
              <div className="text-lg font-semibold">
                {status.health_score.toFixed(0)}%
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Navigation */}
      <nav className="flex-1 space-y-1 p-4">
        <div className="mb-2 text-xs font-semibold uppercase text-muted-foreground">
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
                  ? "bg-primary text-primary-foreground"
                  : "text-muted-foreground hover:bg-muted hover:text-foreground",
              )
            }
          >
            <item.icon className="h-4 w-4" />
            {item.title}
          </NavLink>
        ))}

        {/* Intel Types Section */}
        <div className="mt-6 mb-2 text-xs font-semibold uppercase text-muted-foreground">
          Intelligence Types
        </div>
        {intelTypes.map((type) => (
          <div
            key={type.name}
            className="flex items-center gap-3 rounded-lg px-3 py-2 text-sm"
          >
            <type.icon className={cn("h-4 w-4", type.color)} />
            <span className="text-muted-foreground">{type.name}</span>
          </div>
        ))}
      </nav>

      {/* Footer */}
      <div className="border-t p-4">
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <Shield className="h-4 w-4" />
          <span>Classification: INTERNAL</span>
        </div>
        <div className="mt-1 text-xs text-muted-foreground">
          v0.1.0 | SecureLLM Bridge
        </div>
      </div>
    </div>
  );
}
