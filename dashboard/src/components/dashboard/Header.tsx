import { useState } from 'react'
import { motion } from 'framer-motion'
import {
  Menu,
  RefreshCw,
  Bell,
  Search,
  Clock,
  ChevronDown,
  Mic,
  Settings,
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Badge } from '@/components/ui/badge'
import { useDashboardStore, useQueryStore } from '@/stores/dashboard'
import { useAlerts, useScanMutation } from '@/hooks/useApi'
import { TIME_RANGES } from '@/types'
import { cn } from '@/lib/utils'

export function Header() {
  const {
    toggleSidebar,
    timeRange,
    setTimeRange,
    autoRefresh,
    setAutoRefresh,
  } = useDashboardStore()
  const { setQuery } = useQueryStore()
  const { data: alerts } = useAlerts()
  const scanMutation = useScanMutation()

  const [showTimeDropdown, setShowTimeDropdown] = useState(false)
  const [searchValue, setSearchValue] = useState('')

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault()
    if (searchValue.trim()) {
      setQuery(searchValue.trim())
      window.location.href = `/intelligence?q=${encodeURIComponent(searchValue.trim())}`
    }
  }

  const handleScan = () => {
    scanMutation.mutate({ full_scan: true, collect_intelligence: true })
  }

  return (
    <header className="sticky top-0 z-40 flex h-16 items-center justify-between border-b border-[#1A2236] bg-[#0A0E1A]/95 px-4 backdrop-blur supports-[backdrop-filter]:bg-[#0A0E1A]/80 sm:px-6">
      {/* Left Section */}
      <div className="flex items-center gap-4">
        <Button
          variant="ghost"
          size="icon"
          onClick={toggleSidebar}
          className="shrink-0"
        >
          <Menu className="h-5 w-5" />
        </Button>

        {/* Search */}
        <form onSubmit={handleSearch} className="relative hidden md:block">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search intelligence..."
            className="w-80 border-[#1A2236] bg-[#111827] pl-10 pr-10 text-[#E2E8F0]"
            value={searchValue}
            onChange={(e) => setSearchValue(e.target.value)}
          />
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="absolute right-1 top-1/2 h-7 w-7 -translate-y-1/2"
            title="Voice search"
          >
            <Mic className="h-4 w-4 text-muted-foreground" />
          </Button>
        </form>
      </div>

      {/* Right Section - Controls */}
      <div className="flex items-center gap-2">
        {/* Time Range Picker */}
        <div className="relative">
          <Button
            variant="outline"
            size="sm"
            className="gap-2 border-[#1A2236] bg-transparent text-[#E2E8F0] hover:bg-[#111827]"
            onClick={() => setShowTimeDropdown(!showTimeDropdown)}
          >
            <Clock className="h-4 w-4" />
            {TIME_RANGES.find((t) => t.value === timeRange)?.label || 'Select'}
            <ChevronDown className="h-4 w-4" />
          </Button>

          {showTimeDropdown && (
            <motion.div
              initial={{ opacity: 0, y: -10 }}
              animate={{ opacity: 1, y: 0 }}
              className="absolute right-0 top-full z-50 mt-1 w-48 rounded-md border border-[#1A2236] bg-[#111827] p-1 shadow-md"
            >
              {TIME_RANGES.map((range) => (
                <button
                  key={range.value}
                  onClick={() => {
                    setTimeRange(range.value)
                    setShowTimeDropdown(false)
                  }}
                  className={cn(
                    'flex w-full items-center rounded-sm px-2 py-1.5 text-sm',
                    timeRange === range.value
                      ? 'bg-[#6366F1] text-white'
                      : 'text-[#94A3B8] hover:bg-[#1A2236]'
                  )}
                >
                  {range.label}
                </button>
              ))}
            </motion.div>
          )}
        </div>

        {/* Auto Refresh Toggle */}
        <Button
          variant={autoRefresh ? 'bridge' : 'outline'}
          size="sm"
          className="gap-2"
          onClick={() => setAutoRefresh(!autoRefresh)}
        >
          <RefreshCw
            className={cn(
              'h-4 w-4',
              autoRefresh && 'animate-spin',
              scanMutation.isPending && 'animate-spin'
            )}
          />
          {autoRefresh ? 'Auto' : 'Manual'}
        </Button>

        {/* Scan Button */}
        <Button
          variant="outline"
          size="sm"
          className="border-[#1A2236] bg-transparent text-[#E2E8F0] hover:bg-[#111827]"
          onClick={handleScan}
          disabled={scanMutation.isPending}
        >
          {scanMutation.isPending ? 'Scanning...' : 'Scan'}
        </Button>

        {/* Alerts */}
        <Button variant="ghost" size="icon" className="relative">
          <Bell className="h-5 w-5" />
          {alerts && alerts.length > 0 && (
            <Badge
              variant="destructive"
              className="absolute -right-1 -top-1 h-5 w-5 rounded-full p-0 text-xs"
            >
              {alerts.length}
            </Badge>
          )}
        </Button>

        {/* Settings */}
        <Button variant="ghost" size="icon">
          <Settings className="h-5 w-5" />
        </Button>
      </div>
    </header>
  )
}
