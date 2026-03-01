import { useCallback, useEffect, useState } from 'react'
import type { ComponentType, SVGProps } from 'react'
import {
  PanelLeft,
  Plus,
  Folder,
  Save,
  Sun,
  Moon,
  Maximize,
  Minimize,
  Blocks,
} from 'lucide-react'
import ClaudeLogo from '@/components/icons/claude-logo'
import OpenAILogo from '@/components/icons/openai-logo'
import OpenCodeLogo from '@/components/icons/opencode-logo'
import FigmaLogo from '@/components/icons/figma-logo'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import {
  Tooltip,
  TooltipTrigger,
  TooltipContent,
} from '@/components/ui/tooltip'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import {
  supportsFileSystemAccess,
  writeToFileHandle,
  saveDocumentAs,
  downloadDocument,
  openDocumentFS,
  openDocument,
} from '@/utils/file-operations'
import { syncCanvasPositionsToStore } from '@/canvas/use-canvas-sync'
import { zoomToFitContent } from '@/canvas/use-fabric-canvas'
import { useAgentSettingsStore } from '@/stores/agent-settings-store'
import type { AIProviderType } from '@/types/agent-settings'

const PROVIDER_ICONS: Record<AIProviderType, ComponentType<SVGProps<SVGSVGElement>>> = {
  anthropic: ClaudeLogo,
  openai: OpenAILogo,
  opencode: OpenCodeLogo,
}

const PROVIDER_ORDER: AIProviderType[] = ['anthropic', 'openai', 'opencode']

function AgentStatusButton() {
  const providers = useAgentSettingsStore((s) => s.providers)
  const mcpIntegrations = useAgentSettingsStore((s) => s.mcpIntegrations)
  const connectedTypes = PROVIDER_ORDER.filter((t) => providers[t].isConnected)
  const agentCount = connectedTypes.length
  const mcpCount = mcpIntegrations.filter((m) => m.enabled).length
  const hasAny = agentCount > 0 || mcpCount > 0

  const tooltipParts: string[] = []
  if (agentCount > 0) tooltipParts.push(`${agentCount} agent${agentCount !== 1 ? 's' : ''}`)
  if (mcpCount > 0) tooltipParts.push(`${mcpCount} MCP`)

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => useAgentSettingsStore.getState().setDialogOpen(true)}
          className="h-7 px-2 text-muted-foreground hover:text-foreground"
        >
          {hasAny ? (
            <div className="flex items-center gap-1.5">
              {agentCount > 0 && (
                <div className="flex items-center -space-x-1.5">
                  {connectedTypes.map((type) => {
                    const Icon = PROVIDER_ICONS[type]
                    return (
                      <div
                        key={type}
                        className="w-5 h-5 rounded-md bg-foreground/10 flex items-center justify-center ring-1 ring-card"
                      >
                        <Icon className="w-3 h-3" />
                      </div>
                    )
                  })}
                </div>
              )}
              {agentCount === 0 && (
                <Blocks size={14} strokeWidth={1.5} />
              )}
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 shrink-0" />
              <span className="text-[11px] text-muted-foreground hidden sm:inline">
                {tooltipParts.join(' · ')}
              </span>
            </div>
          ) : (
            <div className="flex items-center gap-1.5">
              <Blocks size={14} strokeWidth={1.5} />
              <span className={cn('text-[11px]', 'hidden sm:inline')}>
                Agents & MCP
              </span>
            </div>
          )}
        </Button>
      </TooltipTrigger>
      <TooltipContent side="bottom">
        {hasAny ? tooltipParts.join(' · ') + ' connected' : 'Setup Agents & MCP'}
      </TooltipContent>
    </Tooltip>
  )
}

export default function TopBar() {
  const toggleLayerPanel = useCanvasStore((s) => s.toggleLayerPanel)
  const layerPanelOpen = useCanvasStore((s) => s.layerPanelOpen)
  const fileName = useDocumentStore((s) => s.fileName)
  const isDirty = useDocumentStore((s) => s.isDirty)

  const [theme, setTheme] = useState<'dark' | 'light'>('dark')
  const [isFullscreen, setIsFullscreen] = useState(false)

  // Restore saved theme after hydration
  useEffect(() => {
    try {
      const saved = localStorage.getItem('openpencil-theme')
      if (saved === 'light') {
        document.documentElement.classList.add('light')
        setTheme('light')
      }
    } catch {
      // ignore
    }
  }, [])

  // Listen to fullscreen changes
  useEffect(() => {
    const handler = () => setIsFullscreen(!!document.fullscreenElement)
    document.addEventListener('fullscreenchange', handler)
    return () => document.removeEventListener('fullscreenchange', handler)
  }, [])

  const toggleTheme = useCallback(() => {
    const next = theme === 'dark' ? 'light' : 'dark'
    if (next === 'light') {
      document.documentElement.classList.add('light')
    } else {
      document.documentElement.classList.remove('light')
    }
    setTheme(next)
    try {
      localStorage.setItem('openpencil-theme', next)
    } catch {
      // ignore
    }
  }, [theme])

  const toggleFullscreen = useCallback(() => {
    if (document.fullscreenElement) {
      document.exitFullscreen()
    } else {
      document.documentElement.requestFullscreen()
    }
  }, [])

  const handleNew = useCallback(() => {
    useDocumentStore.getState().newDocument()
    requestAnimationFrame(() => zoomToFitContent())
  }, [])

  const handleSave = useCallback(() => {
    syncCanvasPositionsToStore()
    const store = useDocumentStore.getState()
    const { document: doc, fileName: fn, fileHandle } = store

    if (fileHandle) {
      writeToFileHandle(fileHandle, doc).then(() => store.markClean())
    } else if (fn) {
      downloadDocument(doc, fn)
      store.markClean()
    } else if (supportsFileSystemAccess()) {
      saveDocumentAs(doc, 'untitled.op').then((result) => {
        if (result) {
          useDocumentStore.setState({
            fileName: result.fileName,
            fileHandle: result.handle,
            isDirty: false,
          })
        }
      })
    } else {
      store.setSaveDialogOpen(true)
    }
  }, [])

  const handleOpen = useCallback(() => {
    if (supportsFileSystemAccess()) {
      openDocumentFS().then((result) => {
        if (result) {
          useDocumentStore
            .getState()
            .loadDocument(result.doc, result.fileName, result.handle)
          requestAnimationFrame(() => zoomToFitContent())
        }
      })
    } else {
      openDocument().then((result) => {
        if (result) {
          useDocumentStore.getState().loadDocument(result.doc, result.fileName)
          requestAnimationFrame(() => zoomToFitContent())
        }
      })
    }
  }, [])

  const displayName = fileName ?? 'Untitled'

  return (
    <div className="h-10 bg-card border-b border-border flex items-center px-2 shrink-0 select-none app-region-drag">
      {/* Left section */}
      <div className="flex items-center gap-0.5 app-region-no-drag electron-traffic-light-pad">
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={toggleLayerPanel}
              className={layerPanelOpen ? 'text-foreground' : 'text-muted-foreground'}
            >
              <PanelLeft size={15} strokeWidth={1.5} />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">
            {layerPanelOpen ? 'Hide layers' : 'Show layers'}
          </TooltipContent>
        </Tooltip>

        <div className="w-px h-3.5 bg-border/60 mx-1" />

        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="icon-sm" className="text-muted-foreground" onClick={handleNew}>
              <Plus size={16} strokeWidth={1.5} />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">New</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="icon-sm" className="text-muted-foreground" onClick={handleOpen}>
              <Folder size={15} strokeWidth={1.5} />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">Open</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="icon-sm" className="text-muted-foreground" onClick={handleSave}>
              <Save size={15} strokeWidth={1.5} />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">Save</TooltipContent>
        </Tooltip>

        <div className="w-px h-3.5 bg-border/60 mx-1" />

        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon-sm"
              className="text-muted-foreground"
              onClick={() => useCanvasStore.getState().setFigmaImportDialogOpen(true)}
            >
              <FigmaLogo className="w-3.5 h-3.5" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">Import Figma</TooltipContent>
        </Tooltip>
      </div>

      {/* Center section — file name */}
      <div className="flex-1 flex items-center justify-center min-w-0">
        <span className="text-xs text-foreground truncate">
          {displayName}
        </span>
        {isDirty && (
          <span className="text-xs text-muted-foreground ml-1.5">
            — Edited
          </span>
        )}
      </div>

      {/* Right section */}
      <div className="flex items-center gap-0.5 app-region-no-drag">
        <AgentStatusButton />

        <div className="w-px h-3.5 bg-border/60 mx-1" />

        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="icon-sm" className="text-muted-foreground" onClick={toggleTheme}>
              {theme === 'dark' ? <Sun size={15} strokeWidth={1.5} /> : <Moon size={15} strokeWidth={1.5} />}
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">
            {theme === 'dark' ? 'Light mode' : 'Dark mode'}
          </TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="icon-sm" className="text-muted-foreground" onClick={toggleFullscreen}>
              {isFullscreen ? <Minimize size={15} strokeWidth={1.5} /> : <Maximize size={15} strokeWidth={1.5} />}
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">
            {isFullscreen ? 'Exit fullscreen' : 'Fullscreen'}
          </TooltipContent>
        </Tooltip>
      </div>
    </div>
  )
}
