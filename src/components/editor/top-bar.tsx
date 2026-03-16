import { useCallback, useEffect, useState } from 'react'
import { appStorage, initAppStorage } from '@/utils/app-storage'
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
import { useTranslation } from 'react-i18next'
import ClaudeLogo from '@/components/icons/claude-logo'
import OpenAILogo from '@/components/icons/openai-logo'
import OpenCodeLogo from '@/components/icons/opencode-logo'
import CopilotLogo from '@/components/icons/copilot-logo'
import FigmaLogo from '@/components/icons/figma-logo'
import LanguageSelector from '@/components/shared/language-selector'
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
  isElectron,
  writeToFileHandle,
  writeToFilePath,
  saveDocumentAs,
  downloadDocument,
  openDocumentFS,
  openDocument,
} from '@/utils/file-operations'
import { syncCanvasPositionsToStore } from '@/canvas/skia-engine-ref'
import { zoomToFitContent } from '@/canvas/skia-engine-ref'
import { normalizePenDocument } from '@/utils/normalize-pen-file'
import { useAgentSettingsStore } from '@/stores/agent-settings-store'
import type { AIProviderType } from '@/types/agent-settings'

/** Convert a computed CSS color value (oklch/rgb/etc.) to #rrggbb via an offscreen canvas. */
function cssToHex(raw: string): string | null {
  const v = raw.trim()
  if (!v) return null
  try {
    const ctx = document.createElement('canvas').getContext('2d')
    if (!ctx) return null
    ctx.fillStyle = v
    const hex = ctx.fillStyle // browser normalises to #rrggbb
    return hex.startsWith('#') ? hex : null
  } catch {
    return null
  }
}

const PROVIDER_ICONS: Record<AIProviderType, ComponentType<SVGProps<SVGSVGElement>>> = {
  anthropic: ClaudeLogo,
  openai: OpenAILogo,
  opencode: OpenCodeLogo,
  copilot: CopilotLogo,
}

const PROVIDER_ORDER: AIProviderType[] = ['anthropic', 'openai', 'opencode', 'copilot']

function AgentStatusButton() {
  const { t } = useTranslation()
  const providers = useAgentSettingsStore((s) => s.providers)
  const mcpIntegrations = useAgentSettingsStore((s) => s.mcpIntegrations)
  const connectedTypes = PROVIDER_ORDER.filter((tp) => providers[tp].isConnected)
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
                {t('topbar.agentsAndMcp')}
              </span>
            </div>
          )}
        </Button>
      </TooltipTrigger>
      <TooltipContent side="bottom">
        {hasAny ? tooltipParts.join(' · ') + ' ' + t('topbar.connected') : t('topbar.setupAgentsMcp')}
      </TooltipContent>
    </Tooltip>
  )
}

export default function TopBar() {
  const { t } = useTranslation()
  const toggleLayerPanel = useCanvasStore((s) => s.toggleLayerPanel)
  const layerPanelOpen = useCanvasStore((s) => s.layerPanelOpen)
  const fileName = useDocumentStore((s) => s.fileName)
  const isDirty = useDocumentStore((s) => s.isDirty)

  const [theme, setTheme] = useState<'dark' | 'light'>('dark')
  const [isFullscreen, setIsFullscreen] = useState(false)

  // Read computed CSS --card and --card-foreground as hex for Electron overlay
  const syncOverlayColors = useCallback((t: 'dark' | 'light') => {
    if (!window.electronAPI?.setTheme) return
    // Allow a frame for CSS to apply after class toggle
    requestAnimationFrame(() => {
      const s = getComputedStyle(document.documentElement)
      const bg = cssToHex(s.getPropertyValue('--card'))
      const fg = cssToHex(s.getPropertyValue('--card-foreground'))
      window.electronAPI!.setTheme(t, bg && fg ? { bg, fg } : undefined)
    })
  }, [])

  // Restore saved theme after hydration.
  // initAppStorage() must run first in Electron to populate the IPC cache,
  // since appStorage.getItem is synchronous.
  useEffect(() => {
    const restore = async () => {
      await initAppStorage()
      const saved = appStorage.getItem('openpencil-theme')
      if (saved === 'light') {
        document.documentElement.classList.add('light')
        setTheme('light')
        syncOverlayColors('light')
      } else {
        syncOverlayColors('dark')
      }
    }
    restore()
  }, [syncOverlayColors])

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
    syncOverlayColors(next)
    appStorage.setItem('openpencil-theme', next)
  }, [theme, syncOverlayColors])

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

  /**
   * Unified save: if the current file is .op with a known handle/path, save
   * in-place; otherwise trigger "save as .op".
   */
  const handleSave = useCallback(async () => {
    try {
      syncCanvasPositionsToStore()
    } catch (err) {
      console.error('[Save] syncCanvasPositionsToStore failed:', err)
    }
    const store = useDocumentStore.getState()
    const { document: doc, fileName: fn, fileHandle, filePath } = store

    const isOpFile = fn ? /\.op$/i.test(fn) : false
    const suggestedName = fn
      ? fn.replace(/\.(pen|op|json)$/i, '') + '.op'
      : 'untitled.op'

    try {
      // Electron with known .op path → direct write
      if (isElectron() && filePath && isOpFile) {
        await writeToFilePath(filePath, doc)
        store.markClean()
        return
      }

      // Browser with valid .op file handle → direct write
      if (fileHandle && isOpFile) {
        try {
          await writeToFileHandle(fileHandle, doc)
          store.markClean()
          return
        } catch (err) {
          console.warn('[Save] File handle write failed, falling back:', err)
          useDocumentStore.setState({ fileHandle: null })
        }
      }

      // No in-place target (new file, .pen file, or stale handle) → save as .op
      if (isElectron()) {
        const savedPath = await window.electronAPI!.saveFile(
          JSON.stringify(doc),
          suggestedName,
        )
        if (savedPath) {
          useDocumentStore.setState({
            fileName: savedPath.split(/[/\\]/).pop() || suggestedName,
            filePath: savedPath,
            fileHandle: null,
            isDirty: false,
          })
        }
      } else if (supportsFileSystemAccess()) {
        const result = await saveDocumentAs(doc, suggestedName)
        if (result) {
          useDocumentStore.setState({
            fileName: result.fileName,
            fileHandle: result.handle,
            isDirty: false,
          })
        }
      } else {
        downloadDocument(doc, suggestedName)
        store.markClean()
      }
    } catch (err) {
      console.error('[Save] Failed to save document:', err)
      try {
        downloadDocument(doc, suggestedName)
        store.markClean()
      } catch (dlErr) {
        console.error('[Save] Download fallback also failed:', dlErr)
      }
    }
  }, [])

  const handleOpen = useCallback(() => {
    if (isElectron()) {
      window.electronAPI!.openFile().then((result) => {
        if (!result) return
        try {
          const raw = JSON.parse(result.content)
          if (!raw.version || (!Array.isArray(raw.children) && !Array.isArray(raw.pages))) return
          const doc = normalizePenDocument(raw)
          const name = result.filePath.split(/[/\\]/).pop() || 'untitled.op'
          useDocumentStore.getState().loadDocument(doc, name, null, result.filePath)
          requestAnimationFrame(() => zoomToFitContent())
        } catch { /* invalid file */ }
      })
    } else if (supportsFileSystemAccess()) {
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

  const displayName = fileName ?? t('common.untitled')

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
            {layerPanelOpen ? t('topbar.hideLayers') : t('topbar.showLayers')}
          </TooltipContent>
        </Tooltip>

        <div className="w-px h-3.5 bg-border/60 mx-1" />

        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="icon-sm" className="text-muted-foreground" onClick={handleNew}>
              <Plus size={16} strokeWidth={1.5} />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">{t('topbar.new')}</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="icon-sm" className="text-muted-foreground" onClick={handleOpen}>
              <Folder size={15} strokeWidth={1.5} />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">{t('topbar.open')}</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="icon-sm" className="text-muted-foreground" onClick={handleSave}>
              <Save size={15} strokeWidth={1.5} />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">{t('topbar.save')}</TooltipContent>
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
          <TooltipContent side="bottom">{t('topbar.importFigma')}</TooltipContent>
        </Tooltip>
      </div>

      {/* Center section — file name */}
      <div className="flex-1 flex items-center justify-center min-w-0">
        <span className="text-xs text-foreground truncate" suppressHydrationWarning>
          {displayName}
        </span>
        {isDirty && (
          <span className="text-xs text-muted-foreground ml-1.5">
            {t('topbar.edited')}
          </span>
        )}
      </div>

      {/* Right section */}
      <div className="flex items-center gap-0.5 app-region-no-drag electron-win-controls-pad">
        <AgentStatusButton />

        <div className="w-px h-3.5 bg-border/60 mx-1" />

        <LanguageSelector />

        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="icon-sm" className="text-muted-foreground" onClick={toggleTheme}>
              {theme === 'dark' ? <Sun size={15} strokeWidth={1.5} /> : <Moon size={15} strokeWidth={1.5} />}
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">
            {theme === 'dark' ? t('topbar.lightMode') : t('topbar.darkMode')}
          </TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="icon-sm" className="text-muted-foreground" onClick={toggleFullscreen}>
              {isFullscreen ? <Minimize size={15} strokeWidth={1.5} /> : <Maximize size={15} strokeWidth={1.5} />}
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">
            {isFullscreen ? t('topbar.exitFullscreen') : t('topbar.fullscreen')}
          </TooltipContent>
        </Tooltip>
      </div>
    </div>
  )
}
