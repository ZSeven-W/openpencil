import { useEffect, useRef, type RefObject } from 'react'
import * as fabric from 'fabric'
import { useCanvasStore } from '@/stores/canvas-store'
import { CANVAS_BACKGROUND } from './canvas-constants'

export function useFabricCanvas(
  canvasRef: RefObject<HTMLCanvasElement | null>,
  containerRef: RefObject<HTMLDivElement | null>,
) {
  const initialized = useRef(false)

  useEffect(() => {
    const el = canvasRef.current
    const container = containerRef.current
    if (!el || !container || initialized.current) return

    initialized.current = true

    const canvas = new fabric.Canvas(el, {
      width: container.clientWidth,
      height: container.clientHeight,
      backgroundColor: CANVAS_BACKGROUND,
      selection: true,
      preserveObjectStacking: true,
      stopContextMenu: true,
      fireRightClick: true,
    })

    // Selection styling
    canvas.selectionColor = 'rgba(59, 130, 246, 0.1)'
    canvas.selectionBorderColor = '#3b82f6'
    canvas.selectionLineWidth = 1

    useCanvasStore.getState().setFabricCanvas(canvas)
    canvas.requestRenderAll()

    // Resize observer
    const resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const { width, height } = entry.contentRect
        canvas.setDimensions({ width, height })
        canvas.requestRenderAll()
      }
    })
    resizeObserver.observe(container)

    return () => {
      resizeObserver.disconnect()
      useCanvasStore.getState().setFabricCanvas(null)
      canvas.dispose()
      initialized.current = false
    }
  }, [canvasRef, containerRef])
}
