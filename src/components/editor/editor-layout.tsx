import { lazy, Suspense, useState, useCallback, useEffect } from 'react'
import { TooltipProvider } from '@/components/ui/tooltip'
import TopBar from './top-bar'
import Toolbar from './toolbar'
import StatusBar from './status-bar'
import LayerPanel from '@/components/panels/layer-panel'
import PropertyPanel from '@/components/panels/property-panel'
import AIChatPanel, { AIChatMinimizedBar } from '@/components/panels/ai-chat-panel'
import CodePanel from '@/components/panels/code-panel'
import ExportDialog from '@/components/shared/export-dialog'
import SaveDialog from '@/components/shared/save-dialog'
import { useAIStore } from '@/stores/ai-store'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'

const FabricCanvas = lazy(() => import('@/canvas/fabric-canvas'))

export default function EditorLayout() {
  const toggleMinimize = useAIStore((s) => s.toggleMinimize)
  const hasSelection = useCanvasStore((s) => s.selection.activeId !== null)
  const layerPanelOpen = useCanvasStore((s) => s.layerPanelOpen)
  const saveDialogOpen = useDocumentStore((s) => s.saveDialogOpen)
  const closeSaveDialog = useCallback(() => {
    useDocumentStore.getState().setSaveDialogOpen(false)
  }, [])
  const [codePanelOpen, setCodePanelOpen] = useState(false)
  const [exportOpen, setExportOpen] = useState(false)

  const toggleCodePanel = useCallback(() => {
    setCodePanelOpen((prev) => !prev)
  }, [])

  const closeExport = useCallback(() => {
    setExportOpen(false)
  }, [])

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const isMod = e.metaKey || e.ctrlKey

      // Cmd+J: toggle AI panel minimize
      if (isMod && e.key === 'j') {
        e.preventDefault()
        toggleMinimize()
        return
      }

      // Cmd+Shift+C: toggle code panel
      if (isMod && e.shiftKey && e.key.toLowerCase() === 'c') {
        e.preventDefault()
        toggleCodePanel()
        return
      }

      // Cmd+Shift+E: open export
      if (isMod && e.shiftKey && e.key.toLowerCase() === 'e') {
        e.preventDefault()
        setExportOpen((prev) => !prev)
        return
      }
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [toggleMinimize, toggleCodePanel])

  return (
    <TooltipProvider delayDuration={300}>
      <div className="h-screen flex flex-col bg-background">
        <TopBar />
        <div className="flex-1 flex flex-col overflow-hidden">
          <div className="flex-1 flex overflow-hidden">
            {layerPanelOpen && <LayerPanel />}
            <div className="flex-1 flex flex-col min-w-0 relative">
              <Suspense
                fallback={
                  <div className="flex-1 flex items-center justify-center bg-muted text-muted-foreground text-sm">
                    Loading canvas...
                  </div>
                }
              >
                <FabricCanvas />
              </Suspense>
              <Toolbar />

              {/* Bottom bar: minimized AI (left) + zoom controls (right) */}
              <div className="absolute bottom-2 left-2 right-2 z-10 flex items-center justify-between pointer-events-none">
                <div className="pointer-events-auto">
                  <AIChatMinimizedBar />
                </div>
                <div className="pointer-events-auto">
                  <StatusBar />
                </div>
              </div>

              {/* Expanded AI panel (floating, draggable) */}
              <AIChatPanel />
            </div>
            {hasSelection && <PropertyPanel />}
          </div>
          {codePanelOpen && <CodePanel onClose={() => setCodePanelOpen(false)} />}
        </div>
        <ExportDialog open={exportOpen} onClose={closeExport} />
        <SaveDialog open={saveDialogOpen} onClose={closeSaveDialog} />
      </div>
    </TooltipProvider>
  )
}
