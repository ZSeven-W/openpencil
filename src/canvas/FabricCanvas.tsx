import { useRef } from 'react'
import { useFabricCanvas } from './use-fabric-canvas'
import { useCanvasEvents } from './use-canvas-events'
import { useCanvasViewport } from './use-canvas-viewport'
import { useCanvasSelection } from './use-canvas-selection'
import { useCanvasSync } from './use-canvas-sync'

export default function FabricCanvas() {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const containerRef = useRef<HTMLDivElement>(null)

  useFabricCanvas(canvasRef, containerRef)
  useCanvasEvents()
  useCanvasViewport()
  useCanvasSelection()
  useCanvasSync()

  return (
    <div
      ref={containerRef}
      className="flex-1 relative overflow-hidden bg-neutral-100"
    >
      <canvas ref={canvasRef} />
    </div>
  )
}
