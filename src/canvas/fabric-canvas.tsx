import { useRef } from 'react'
import { useFabricCanvas } from './use-fabric-canvas'
import { useCanvasGuides } from './use-canvas-guides'
import { useCanvasEvents } from './use-canvas-events'
import { useCanvasViewport } from './use-canvas-viewport'
import { useCanvasSelection } from './use-canvas-selection'
import { useCanvasSync } from './use-canvas-sync'
import { useDimensionLabel } from './use-dimension-label'
import { useFrameLabels } from './use-frame-labels'

export default function FabricCanvas() {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const containerRef = useRef<HTMLDivElement>(null)

  useFabricCanvas(canvasRef, containerRef)
  useCanvasGuides()
  useCanvasEvents()
  useCanvasViewport()
  useCanvasSelection()
  useCanvasSync()
  useDimensionLabel(containerRef)
  useFrameLabels()

  return (
    <div
      ref={containerRef}
      className="flex-1 relative overflow-hidden bg-muted"
    >
      <canvas ref={canvasRef} />
    </div>
  )
}
