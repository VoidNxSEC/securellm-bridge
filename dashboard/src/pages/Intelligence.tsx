import { useState } from 'react'
import { useSearchParams } from 'react-router-dom'
import { motion } from 'framer-motion'
import {
  Search,
  Radio,
  Users,
  Globe,
  Code,
  Filter,
  Sparkles,
  Mic,
  StopCircle,
} from 'lucide-react'
import { Card, CardContent } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { useIntelligenceQuery, useIntelligenceStats } from '@/hooks/useApi'
import { cn } from '@/lib/utils'
import type { IntelligenceType, IntelligenceItem } from '@/types'

const INTEL_TYPES = [
  { value: 'sigint', label: 'Signals', icon: Radio, color: 'text-amber-500' },
  { value: 'humint', label: 'Human', icon: Users, color: 'text-green-500' },
  { value: 'osint', label: 'Open Source', icon: Globe, color: 'text-blue-500' },
  { value: 'techint', label: 'Technical', icon: Code, color: 'text-violet-500' },
]

export function Intelligence() {
  const [searchParams, setSearchParams] = useSearchParams()
  const initialQuery = searchParams.get('q') || ''

  const [query, setQuery] = useState(initialQuery)
  const [searchQuery, setSearchQuery] = useState(initialQuery)
  const [selectedTypes, setSelectedTypes] = useState<IntelligenceType[]>([])
  const [isListening, setIsListening] = useState(false)

  const { data: results, isLoading } = useIntelligenceQuery(searchQuery, {
    types: selectedTypes.length > 0 ? selectedTypes : undefined,
    semantic: true,
    limit: 50,
  })

  const { data: stats } = useIntelligenceStats()

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault()
    setSearchQuery(query)
    setSearchParams(query ? { q: query } : {})
  }

  const toggleType = (type: IntelligenceType) => {
    setSelectedTypes((prev) =>
      prev.includes(type) ? prev.filter((t) => t !== type) : [...prev, type]
    )
  }

  // Voice recognition (placeholder)
  const toggleVoice = () => {
    setIsListening(!isListening)
    // TODO: Implement actual voice recognition
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-3xl font-bold tracking-tight">Intelligence</h1>
        <p className="text-muted-foreground">
          Search and explore intelligence from ~/arch ecosystem
        </p>
      </div>

      {/* Stats */}
      <div className="grid gap-4 md:grid-cols-4">
        {INTEL_TYPES.map((type) => (
          <Card key={type.value} className="cursor-pointer hover:border-primary/50 transition-colors"
            onClick={() => toggleType(type.value as IntelligenceType)}
          >
            <CardContent className="p-4">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <type.icon className={cn('h-5 w-5', type.color)} />
                  <span className="font-medium">{type.label}</span>
                </div>
                <Badge
                  variant={selectedTypes.includes(type.value as IntelligenceType) ? 'default' : 'secondary'}
                >
                  {stats?.by_type?.[type.value] || 0}
                </Badge>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>

      {/* Search */}
      <Card>
        <CardContent className="p-6">
          <form onSubmit={handleSearch} className="flex gap-4">
            <div className="relative flex-1">
              <Search className="absolute left-3 top-1/2 h-5 w-5 -translate-y-1/2 text-muted-foreground" />
              <Input
                placeholder="Search intelligence... (e.g., 'How does authentication work?')"
                className="pl-12 pr-12 h-12 text-lg"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
              />
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="absolute right-2 top-1/2 -translate-y-1/2"
                onClick={toggleVoice}
              >
                {isListening ? (
                  <StopCircle className="h-5 w-5 text-red-500 animate-pulse" />
                ) : (
                  <Mic className="h-5 w-5 text-muted-foreground" />
                )}
              </Button>
            </div>
            <Button type="submit" size="lg" className="gap-2">
              <Sparkles className="h-5 w-5" />
              Search
            </Button>
          </form>

          {/* Filters */}
          <div className="flex items-center gap-4 mt-4">
            <Filter className="h-4 w-4 text-muted-foreground" />
            <span className="text-sm text-muted-foreground">Filter by type:</span>
            {INTEL_TYPES.map((type) => (
              <Button
                key={type.value}
                variant={selectedTypes.includes(type.value as IntelligenceType) ? 'default' : 'outline'}
                size="sm"
                className="gap-2"
                onClick={() => toggleType(type.value as IntelligenceType)}
              >
                <type.icon className={cn('h-4 w-4', type.color)} />
                {type.label}
              </Button>
            ))}
            {selectedTypes.length > 0 && (
              <Button variant="ghost" size="sm" onClick={() => setSelectedTypes([])}>
                Clear
              </Button>
            )}
          </div>
        </CardContent>
      </Card>

      {/* Results */}
      {searchQuery && (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-semibold">
              {isLoading ? 'Searching...' : `${results?.total || 0} results`}
            </h2>
            {results?.search_type && (
              <Badge variant="secondary">{results.search_type} search</Badge>
            )}
          </div>

          <div className="space-y-3">
            {results?.results.map((item, i) => (
              <IntelligenceCard key={item.id} item={item} index={i} />
            ))}
          </div>

          {results?.results.length === 0 && !isLoading && (
            <Card>
              <CardContent className="p-8 text-center text-muted-foreground">
                No intelligence found for "{searchQuery}"
              </CardContent>
            </Card>
          )}
        </div>
      )}

      {/* Empty State */}
      {!searchQuery && (
        <Card>
          <CardContent className="p-12 text-center">
            <Search className="h-12 w-12 mx-auto mb-4 text-muted-foreground" />
            <h3 className="text-lg font-semibold mb-2">Search Intelligence</h3>
            <p className="text-muted-foreground max-w-md mx-auto">
              Enter a query to search across all indexed intelligence from your projects.
              Use natural language for semantic search.
            </p>
            <div className="mt-6 flex flex-wrap justify-center gap-2">
              {[
                'How does the RAG engine work?',
                'Security configurations',
                'API endpoints',
                'Database schemas',
              ].map((example) => (
                <Button
                  key={example}
                  variant="outline"
                  size="sm"
                  onClick={() => {
                    setQuery(example)
                    setSearchQuery(example)
                  }}
                >
                  {example}
                </Button>
              ))}
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  )
}

function IntelligenceCard({ item, index }: { item: IntelligenceItem; index: number }) {
  const typeConfig = INTEL_TYPES.find((t) => t.value === item.type)
  const TypeIcon = typeConfig?.icon || Code

  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay: index * 0.03 }}
    >
      <Card className="hover:border-primary/50 transition-colors">
        <CardContent className="p-4">
          <div className="flex items-start gap-4">
            <div className={cn('p-2 rounded-lg bg-muted', typeConfig?.color)}>
              <TypeIcon className="h-5 w-5" />
            </div>

            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2 mb-1">
                <h3 className="font-semibold truncate">{item.title}</h3>
                {item.score && (
                  <Badge variant="secondary" className="shrink-0">
                    {(item.score * 100).toFixed(0)}% match
                  </Badge>
                )}
              </div>

              <p className="text-sm text-muted-foreground line-clamp-2 mb-2">
                {item.content}
              </p>

              <div className="flex items-center gap-3 text-xs text-muted-foreground">
                <Badge variant={item.type as any}>{item.type}</Badge>
                <Badge variant={item.threat_level as any}>{item.threat_level}</Badge>
                <span>{item.source}</span>
                {item.related_projects?.length > 0 && (
                  <span>• {item.related_projects.join(', ')}</span>
                )}
              </div>
            </div>
          </div>
        </CardContent>
      </Card>
    </motion.div>
  )
}
