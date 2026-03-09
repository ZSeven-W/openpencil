import { useEffect } from 'react'
import { useCanvasStore } from '@/stores/canvas-store'
import { activeInsertionIndicator, activeContainerHighlight } from './insertion-indicator'
import { INDICATOR_BLUE, INDICATOR_LINE_WIDTH, INDICATOR_DASH, INDICATOR_ENDPOINT_RADIUS } from './canvas-constants'

/**
 * Renders the layout reorder insertion indicator on the canvas overlay
 * using the `after:render` hook — same pattern as use-canvas-guides.ts.
 */
export function useLayoutIndicator() {
  useEffect(() => {
    const interval = setInterval(() => {
      const canvas = useCanvasStore.getState().fabricCanvas
      if (!canvas) return
      clearInterval(interval)

      const onAfterRender = () => {
        if (!activeInsertionIndicator && !activeContainerHighlight) return

        const el = canvas.lowerCanvasEl
        const ctx = el?.getContext('2d')
        if (!ctx) return

        const vpt = canvas.viewportTransform
        if (!vpt) return
        const zoom = vpt[0]

        ctx.save()
        ctx.transform(vpt[0], vpt[1], vpt[2], vpt[3], vpt[4], vpt[5])

        // Draw container highlight (dashed blue rectangle)
        if (activeContainerHighlight) {
          const { x: cx, y: cy, w: cw, h: ch } = activeContainerHighlight
          ctx.strokeStyle = INDICATOR_BLUE
          ctx.lineWidth = INDICATOR_LINE_WIDTH / zoom
          ctx.setLineDash(INDICATOR_DASH.map((d) => d / zoom))
          ctx.strokeRect(cx, cy, cw, ch)
          ctx.setLineDash([])
        }

        // Draw insertion indicator line
        if (activeInsertionIndicator) {
          const { x, y, length, orientation } = activeInsertionIndicator

          ctx.strokeStyle = INDICATOR_BLUE
          ctx.lineWidth = INDICATOR_LINE_WIDTH / zoom
          ctx.setLineDash([])
          ctx.beginPath()
          if (orientation === 'horizontal') {
            ctx.moveTo(x, y)
            ctx.lineTo(x + length, y)
          } else {
            ctx.moveTo(x, y)
            ctx.lineTo(x, y + length)
          }
          ctx.stroke()

          // Small circles at endpoints
          ctx.fillStyle = INDICATOR_BLUE
          const r = INDICATOR_ENDPOINT_RADIUS / zoom
          ctx.beginPath()
          if (orientation === 'horizontal') {
            ctx.arc(x, y, r, 0, Math.PI * 2)
            ctx.moveTo(x + length + r, y)
            ctx.arc(x + length, y, r, 0, Math.PI * 2)
          } else {
            ctx.arc(x, y, r, 0, Math.PI * 2)
            ctx.moveTo(x + r, y + length)
            ctx.arc(x, y + length, r, 0, Math.PI * 2)
          }
          ctx.fill()
        }

        ctx.restore()
      }

      canvas.on('after:render', onAfterRender)
    }, 100)

    return () => clearInterval(interval)
  }, [])
}
