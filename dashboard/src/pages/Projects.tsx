import { useState } from 'react'
import { useParams } from 'react-router-dom'
import { motion } from 'framer-motion'
import {
  FolderKanban,
  Search,
  Filter,
  Grid,
  List,
  ExternalLink,
  Clock,
} from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Progress } from '@/components/ui/progress'
import { useProjects, useProject } from '@/hooks/useApi'
import { getHealthColor, formatRelativeTime, cn } from '@/lib/utils'
import type { Project } from '@/types'

export function Projects() {
  const { projectName } = useParams()
  const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid')
  const [searchTerm, setSearchTerm] = useState('')
  const [statusFilter, setStatusFilter] = useState<string>('all')

  const { data: projects, isLoading } = useProjects()
  const { data: projectDetail } = useProject(projectName || '')

  // Filter projects
  const filteredProjects = projects?.filter((p) => {
    const matchesSearch = p.name.toLowerCase().includes(searchTerm.toLowerCase())
    const matchesStatus = statusFilter === 'all' || p.status === statusFilter
    return matchesSearch && matchesStatus
  })

  if (projectName && projectDetail) {
    return <ProjectDetail project={projectDetail} />
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Projects</h1>
          <p className="text-muted-foreground">
            {projects?.length || 0} projects in ~/arch ecosystem
          </p>
        </div>
      </div>

      {/* Filters */}
      <div className="flex flex-wrap items-center gap-4">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search projects..."
            className="pl-10"
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
          />
        </div>

        <div className="flex items-center gap-2">
          <Filter className="h-4 w-4 text-muted-foreground" />
          {['all', 'active', 'maintenance', 'deprecated', 'archived'].map((status) => (
            <Button
              key={status}
              variant={statusFilter === status ? 'default' : 'outline'}
              size="sm"
              onClick={() => setStatusFilter(status)}
            >
              {status.charAt(0).toUpperCase() + status.slice(1)}
            </Button>
          ))}
        </div>

        <div className="flex items-center gap-1 border rounded-md">
          <Button
            variant={viewMode === 'grid' ? 'secondary' : 'ghost'}
            size="icon"
            onClick={() => setViewMode('grid')}
          >
            <Grid className="h-4 w-4" />
          </Button>
          <Button
            variant={viewMode === 'list' ? 'secondary' : 'ghost'}
            size="icon"
            onClick={() => setViewMode('list')}
          >
            <List className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {/* Projects Grid/List */}
      {isLoading ? (
        <div className="text-center py-12 text-muted-foreground">
          Loading projects...
        </div>
      ) : viewMode === 'grid' ? (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {filteredProjects?.map((project, i) => (
            <ProjectCard key={project.name} project={project} index={i} />
          ))}
        </div>
      ) : (
        <div className="space-y-2">
          {filteredProjects?.map((project, i) => (
            <ProjectRow key={project.name} project={project} index={i} />
          ))}
        </div>
      )}

      {filteredProjects?.length === 0 && (
        <div className="text-center py-12 text-muted-foreground">
          No projects found matching your criteria
        </div>
      )}
    </div>
  )
}

function ProjectCard({ project, index }: { project: Project; index: number }) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay: index * 0.05 }}
    >
      <Card className="hover:border-primary/50 transition-colors">
        <CardHeader className="pb-2">
          <div className="flex items-start justify-between">
            <div className="flex items-center gap-2">
              <FolderKanban className="h-5 w-5 text-muted-foreground" />
              <CardTitle className="text-lg">{project.name}</CardTitle>
            </div>
            <Badge variant={project.status as any}>{project.status}</Badge>
          </div>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground line-clamp-2 mb-4">
            {project.description || 'No description available'}
          </p>

          <div className="flex flex-wrap gap-1 mb-4">
            {project.languages.slice(0, 3).map((lang) => (
              <Badge key={lang} variant="secondary" className="text-xs">
                {lang}
              </Badge>
            ))}
            {project.languages.length > 3 && (
              <Badge variant="secondary" className="text-xs">
                +{project.languages.length - 3}
              </Badge>
            )}
          </div>

          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <Clock className="h-3 w-3" />
              {formatRelativeTime(project.last_commit)}
            </div>
            <div className="flex items-center gap-2">
              <span className={cn('text-lg font-bold', getHealthColor(project.health_score))}>
                {project.health_score.toFixed(0)}%
              </span>
              <Progress
                value={project.health_score}
                className="w-16"
                indicatorClassName={
                  project.health_score >= 70
                    ? 'bg-green-500'
                    : project.health_score >= 50
                    ? 'bg-yellow-500'
                    : 'bg-red-500'
                }
              />
            </div>
          </div>

          <Button variant="outline" size="sm" className="w-full mt-4" asChild>
            <a href={`/projects/${project.name}`}>
              View Details
              <ExternalLink className="ml-2 h-3 w-3" />
            </a>
          </Button>
        </CardContent>
      </Card>
    </motion.div>
  )
}

function ProjectRow({ project, index }: { project: Project; index: number }) {
  return (
    <motion.div
      initial={{ opacity: 0, x: -20 }}
      animate={{ opacity: 1, x: 0 }}
      transition={{ delay: index * 0.03 }}
    >
      <Card className="hover:border-primary/50 transition-colors">
        <CardContent className="p-4">
          <div className="flex items-center gap-4">
            <FolderKanban className="h-8 w-8 text-muted-foreground shrink-0" />

            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2">
                <h3 className="font-semibold truncate">{project.name}</h3>
                <Badge variant={project.status as any}>{project.status}</Badge>
              </div>
              <p className="text-sm text-muted-foreground truncate">
                {project.description || 'No description'}
              </p>
            </div>

            <div className="flex items-center gap-4 text-sm text-muted-foreground">
              <div className="flex flex-wrap gap-1">
                {project.languages.slice(0, 2).map((lang) => (
                  <Badge key={lang} variant="secondary" className="text-xs">
                    {lang}
                  </Badge>
                ))}
              </div>

              <div className="flex items-center gap-1">
                <Clock className="h-3 w-3" />
                {formatRelativeTime(project.last_commit)}
              </div>

              <div className="flex items-center gap-2">
                <span className={cn('font-bold', getHealthColor(project.health_score))}>
                  {project.health_score.toFixed(0)}%
                </span>
                <Progress
                  value={project.health_score}
                  className="w-20"
                  indicatorClassName={
                    project.health_score >= 70
                      ? 'bg-green-500'
                      : project.health_score >= 50
                      ? 'bg-yellow-500'
                      : 'bg-red-500'
                  }
                />
              </div>

              <Button variant="ghost" size="sm" asChild>
                <a href={`/projects/${project.name}`}>
                  <ExternalLink className="h-4 w-4" />
                </a>
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>
    </motion.div>
  )
}

function ProjectDetail({ project }: { project: Project }) {
  return (
    <div className="space-y-6">
      <div className="flex items-center gap-4">
        <Button variant="ghost" asChild>
          <a href="/projects">← Back to Projects</a>
        </Button>
      </div>

      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight flex items-center gap-3">
            <FolderKanban className="h-8 w-8" />
            {project.name}
          </h1>
          <p className="text-muted-foreground mt-1">
            {project.description || 'No description available'}
          </p>
        </div>
        <Badge variant={project.status as any} className="text-sm">
          {project.status}
        </Badge>
      </div>

      <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-4">
        <Card>
          <CardContent className="p-4">
            <div className="text-sm text-muted-foreground">Health Score</div>
            <div className={cn('text-3xl font-bold', getHealthColor(project.health_score))}>
              {project.health_score.toFixed(0)}%
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="p-4">
            <div className="text-sm text-muted-foreground">Languages</div>
            <div className="flex flex-wrap gap-1 mt-1">
              {project.languages.map((lang) => (
                <Badge key={lang} variant="secondary">{lang}</Badge>
              ))}
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="p-4">
            <div className="text-sm text-muted-foreground">Last Commit</div>
            <div className="text-lg font-semibold">
              {formatRelativeTime(project.last_commit)}
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="p-4">
            <div className="text-sm text-muted-foreground">Path</div>
            <div className="text-sm font-mono truncate">{project.path}</div>
          </CardContent>
        </Card>
      </div>

      {/* Analysis */}
      <Card>
        <CardHeader>
          <CardTitle>Metadata</CardTitle>
        </CardHeader>
        <CardContent>
          <pre className="overflow-auto rounded bg-muted/50 p-4 text-xs">
            {JSON.stringify(project.metadata, null, 2)}
          </pre>
        </CardContent>
      </Card>
    </div>
  )
}
