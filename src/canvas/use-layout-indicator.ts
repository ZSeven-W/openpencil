import { useEffect } from 'react'
import { useCanvasStore } from '@/stores/canvas-store'
import { activeInsertionIndicator } from './layout-reorder'

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
        if (!activeInsertionIndicator) return

        const el = canvas.lowerCanvasEl
        const ctx = el?.getContext('2d')
        if (!ctx) return

        const vpt = canvas.viewportTransform
        if (!vpt) return
        const zoom = vpt[0]

        ctx.save()
        ctx.transform(vpt[0], vpt[1], vpt[2], vpt[3], vpt[4], vpt[5])

        const { x, y, length, orientation } = activeInsertionIndicator

        // Draw indicator line
        ctx.strokeStyle = '#3B82F6'
        ctx.lineWidth = 2 / zoom
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
        ctx.fillStyle = '#3B82F6'
        const r = 3 / zoom
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

        ctx.restore()
      }

      canvas.on('after:render', onAfterRender)
    }, 100)

    return () => clearInterval(interval)
  }, [])
}
