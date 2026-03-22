import { Minus, Plus } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { useCanvasStore } from '@/stores/canvas-store'
import { getSkiaEngineRef } from '@/canvas/skia-engine-ref'
import { Button } from '@/components/ui/button'

export default function StatusBar() {
  const { t } = useTranslation()
  const zoom = useCanvasStore((s) => s.viewport.zoom)

  const zoomPercent = Math.round(zoom * 100)

  const applyZoom = (newZoom: number) => {
    const engine = getSkiaEngineRef()
    if (!engine) return
    const rect = engine.getCanvasRect()
    if (!rect) return
    const cx = rect.left + rect.width / 2
    const cy = rect.top + rect.height / 2
    engine.zoomToPoint(cx, cy, newZoom)
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
        aria-label={t('statusbar.zoomOut')}
      >
        <Minus size={14} />
      </Button>
      <button
        onClick={handleZoomReset}
        className="min-w-[48px] h-5 text-[11px] text-muted-foreground hover:text-foreground tabular-nums text-center cursor-pointer bg-transparent border-none"
        aria-label={t('statusbar.resetZoom')}
      >
        {zoomPercent}%
      </button>
      <Button
        variant="ghost"
        size="icon-sm"
        onClick={handleZoomIn}
        aria-label={t('statusbar.zoomIn')}
      >
        <Plus size={14} />
      </Button>
    </div>
  )
}
