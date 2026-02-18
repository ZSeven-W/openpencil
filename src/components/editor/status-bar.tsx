import { Minus, Plus } from 'lucide-react'
import { useCanvasStore } from '@/stores/canvas-store'
import { MIN_ZOOM, MAX_ZOOM } from '@/canvas/canvas-constants'
import { Button } from '@/components/ui/button'

export default function StatusBar() {
  const zoom = useCanvasStore((s) => s.viewport.zoom)
  const fabricCanvas = useCanvasStore((s) => s.fabricCanvas)

  const zoomPercent = Math.round(zoom * 100)

  const applyZoom = (newZoom: number) => {
    const canvas = fabricCanvas
    if (!canvas) return

    const clamped = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, newZoom))
    const center = canvas.getCenterPoint()
    canvas.zoomToPoint(center, clamped)

    const vpt = canvas.viewportTransform
    if (vpt) {
      useCanvasStore.getState().setZoom(clamped)
      useCanvasStore.getState().setPan(vpt[4], vpt[5])
    }
    canvas.requestRenderAll()
  }

  const handleZoomOut = () => applyZoom(zoom / 1.2)
  const handleZoomIn = () => applyZoom(zoom * 1.2)
  const handleZoomReset = () => applyZoom(1)

  return (
    <div className="h-7 bg-card border border-border rounded-lg flex items-center px-1 gap-0.5 shadow-lg">
      <Button
        variant="ghost"
        size="icon-sm"
        onClick={handleZoomOut}
        aria-label="Zoom out"
      >
        <Minus size={14} />
      </Button>
      <button
        onClick={handleZoomReset}
        className="min-w-[48px] h-5 text-[11px] text-muted-foreground hover:text-foreground tabular-nums text-center cursor-pointer bg-transparent border-none"
        aria-label="Reset zoom"
      >
        {zoomPercent}%
      </button>
      <Button
        variant="ghost"
        size="icon-sm"
        onClick={handleZoomIn}
        aria-label="Zoom in"
      >
        <Plus size={14} />
      </Button>
    </div>
  )
}
