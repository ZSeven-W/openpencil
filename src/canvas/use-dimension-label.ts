import { useEffect, useRef, type RefObject } from 'react'
import * as fabric from 'fabric'
import { useCanvasStore } from '@/stores/canvas-store'
import { nodeRenderInfo } from './use-canvas-sync'
import type { FabricObjectWithPenId } from './canvas-object-factory'

export function useDimensionLabel(
  containerRef: RefObject<HTMLDivElement | null>,
) {
  const labelRef = useRef<HTMLDivElement | null>(null)

  useEffect(() => {
    const container = containerRef.current
    if (!container) return

    // Create the label element
    const label = document.createElement('div')
    label.style.cssText = `
      position: absolute;
      pointer-events: none;
      z-index: 5;
      background: #0d99ff;
      color: #fff;
      font-size: 11px;
      font-weight: 500;
      font-family: -apple-system, BlinkMacSystemFont, sans-serif;
      padding: 2px 8px;
      border-radius: 4px;
      white-space: nowrap;
      display: none;
    `
    container.appendChild(label)
    labelRef.current = label

    const update = () => {
      const canvas = useCanvasStore.getState().fabricCanvas
      if (!canvas || !label) return

      const active = canvas.getActiveObject()
      if (!active) {
        label.style.display = 'none'
        return
      }

      const bound = active.getBoundingRect()

      // Detect active interaction (dragging or scaling)
      const transform = (canvas as unknown as { _currentTransform?: { action?: string } })
        ._currentTransform
      const action = transform?.action

      if (!action) {
        // No active interaction — hide the label
        label.style.display = 'none'
        return
      }

      if (action === 'drag') {
        // Show position in scene coordinates during drag
        const vpt = canvas.viewportTransform
        const zoom = vpt[0]
        const panX = vpt[4]
        const panY = vpt[5]

        if (active instanceof fabric.ActiveSelection) {
          const sceneX = Math.round(((bound.left - panX) / zoom))
          const sceneY = Math.round(((bound.top - panY) / zoom))
          label.textContent = `X: ${sceneX}  Y: ${sceneY}`
        } else {
          const obj = active as FabricObjectWithPenId
          const info = obj.penNodeId ? nodeRenderInfo.get(obj.penNodeId) : null
          const offsetX = info?.parentOffsetX ?? 0
          const offsetY = info?.parentOffsetY ?? 0
          const relX = Math.round((active.left ?? 0) - offsetX)
          const relY = Math.round((active.top ?? 0) - offsetY)
          label.textContent = `X: ${relX}  Y: ${relY}`
        }
      } else {
        // Show dimensions during scale/resize
        const w = Math.round(active.getScaledWidth())
        const h = Math.round(active.getScaledHeight())
        label.textContent = `${w} \u00d7 ${h}`
      }

      label.style.display = 'block'
      label.style.left = `${bound.left + bound.width / 2}px`
      label.style.top = `${bound.top + bound.height + 8}px`
      label.style.transform = 'translateX(-50%)'
    }

    // Poll for canvas availability
    const interval = setInterval(() => {
      const canvas = useCanvasStore.getState().fabricCanvas
      if (!canvas) return
      clearInterval(interval)

      canvas.on('after:render', update)
    }, 100)

    return () => {
      clearInterval(interval)
      const canvas = useCanvasStore.getState().fabricCanvas
      if (canvas) {
        canvas.off('after:render', update)
      }
      label.remove()
      labelRef.current = null
    }
  }, [containerRef])
}
