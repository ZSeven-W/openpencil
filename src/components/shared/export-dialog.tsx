import { useState, useRef, useEffect } from 'react'
import { X } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { useCanvasStore } from '@/stores/canvas-store'
import { exportToPNG, exportToSVG } from '@/utils/export'

interface ExportDialogProps {
  open: boolean
  onClose: () => void
}

export default function ExportDialog({ open, onClose }: ExportDialogProps) {
  const [format, setFormat] = useState<'png' | 'svg'>('png')
  const [scale, setScale] = useState(2)
  const [selectedOnly, setSelectedOnly] = useState(false)
  const fabricCanvas = useCanvasStore((s) => s.fabricCanvas)
  const hasSelection = useCanvasStore(
    (s) => s.selection.selectedIds.length > 0,
  )
  const dialogRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    document.addEventListener('keydown', handleKeyDown)
    return () => document.removeEventListener('keydown', handleKeyDown)
  }, [open, onClose])

  if (!open) return null

  const handleExport = () => {
    if (!fabricCanvas) return
    if (format === 'png') {
      exportToPNG(fabricCanvas, {
        multiplier: scale,
        selectedOnly,
      })
    } else {
      exportToSVG(fabricCanvas, { selectedOnly })
    }
    onClose()
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-background/80" onClick={onClose} />
      <div
        ref={dialogRef}
        className="relative bg-card rounded-lg border border-border p-4 w-72 shadow-xl"
      >
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-sm font-medium text-foreground">Export</h3>
          <Button variant="ghost" size="icon-sm" onClick={onClose}>
            <X size={14} />
          </Button>
        </div>

        {/* Format */}
        <div className="mb-3">
          <label className="text-xs text-muted-foreground block mb-1">Format</label>
          <div className="flex gap-2">
            {(['png', 'svg'] as const).map((f) => (
              <button
                key={f}
                type="button"
                onClick={() => setFormat(f)}
                className={cn(
                  'flex-1 text-xs py-1.5 rounded transition-colors',
                  format === f
                    ? 'bg-primary text-primary-foreground'
                    : 'bg-secondary text-secondary-foreground hover:bg-secondary/80',
                )}
              >
                {f.toUpperCase()}
              </button>
            ))}
          </div>
        </div>

        {/* Scale (PNG only) */}
        {format === 'png' && (
          <div className="mb-3">
            <label className="text-xs text-muted-foreground block mb-1">Scale</label>
            <div className="flex gap-2">
              {[1, 2, 3].map((s) => (
                <button
                  key={s}
                  type="button"
                  onClick={() => setScale(s)}
                  className={cn(
                    'flex-1 text-xs py-1.5 rounded transition-colors',
                    scale === s
                      ? 'bg-primary text-primary-foreground'
                      : 'bg-secondary text-secondary-foreground hover:bg-secondary/80',
                  )}
                >
                  {s}x
                </button>
              ))}
            </div>
          </div>
        )}

        {/* Selected only */}
        {hasSelection && (
          <label className="flex items-center gap-2 mb-4 cursor-pointer">
            <input
              type="checkbox"
              checked={selectedOnly}
              onChange={(e) => setSelectedOnly(e.target.checked)}
              className="rounded border-input bg-secondary text-primary focus:ring-ring focus:ring-offset-0"
            />
            <span className="text-xs text-foreground">Export selected only</span>
          </label>
        )}

        {/* Export button */}
        <Button
          onClick={handleExport}
          disabled={!fabricCanvas}
          className="w-full"
          size="sm"
        >
          Export {format.toUpperCase()}
        </Button>
      </div>
    </div>
  )
}
