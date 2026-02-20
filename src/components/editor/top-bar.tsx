import { useCallback, useEffect, useState } from 'react'
import {
  PanelLeft,
  FilePlus,
  FolderOpen,
  Save,
  Sun,
  Moon,
  Maximize,
  Minimize,
} from 'lucide-react'
import ClaudeLogo from '@/components/icons/claude-logo'
import OpenAILogo from '@/components/icons/openai-logo'
import OpenCodeLogo from '@/components/icons/opencode-logo'
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
    } else if (supportsFileSystemAccess()) {
      saveDocumentAs(doc, fn ?? 'untitled.pen').then((result) => {
        if (result) {
          useDocumentStore.setState({
            fileName: result.fileName,
            fileHandle: result.handle,
            isDirty: false,
          })
        }
      })
    } else if (fn) {
      downloadDocument(doc, fn)
      store.markClean()
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
    <div className="h-10 bg-card border-b border-border flex items-center px-2 shrink-0 select-none">
      {/* Left section */}
      <div className="flex items-center gap-0.5">
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={toggleLayerPanel}
              className={layerPanelOpen ? 'text-foreground' : 'text-muted-foreground'}
            >
              <PanelLeft size={16} />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">
            {layerPanelOpen ? 'Hide layers' : 'Show layers'}
          </TooltipContent>
        </Tooltip>

        <div className="w-px h-4 bg-border mx-1" />

        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="icon-sm" onClick={handleNew}>
              <FilePlus size={16} />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">New document</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="icon-sm" onClick={handleOpen}>
              <FolderOpen size={16} />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">Open</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="icon-sm" onClick={handleSave}>
              <Save size={16} />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">Save</TooltipContent>
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
      <div className="flex items-center gap-0.5">
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => useAgentSettingsStore.getState().setDialogOpen(true)}
              className="h-7 gap-1.5 text-xs text-muted-foreground hover:text-foreground"
            >
              <ClaudeLogo className="w-4 h-4" />
              <OpenAILogo className="w-4 h-4 -ml-1" />
              <OpenCodeLogo className="w-4 h-4 -ml-1" />
              <span className="hidden sm:inline">Agents & MCP</span>
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">Setup Agents & MCP</TooltipContent>
        </Tooltip>

        <div className="w-px h-4 bg-border mx-1" />

        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="icon-sm" onClick={toggleTheme}>
              {theme === 'dark' ? <Sun size={16} /> : <Moon size={16} />}
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">
            {theme === 'dark' ? 'Light mode' : 'Dark mode'}
          </TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="icon-sm" onClick={toggleFullscreen}>
              {isFullscreen ? <Minimize size={16} /> : <Maximize size={16} />}
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
