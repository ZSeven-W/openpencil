import { useEffect } from 'react'
import { useCanvasStore } from '@/stores/canvas-store'
import { activeGuides, clearGuides } from './guide-utils'
import { GUIDE_COLOR, GUIDE_DASH } from './canvas-constants'

/**
 * Renders smart guide lines on the canvas overlay after Fabric's render pass.
 * Guide positions are calculated in use-canvas-events.ts via calculateAndSnap().
 */
export function useCanvasGuides() {
  useEffect(() => {
    const interval = setInterval(() => {
      const canvas = useCanvasStore.getState().fabricCanvas
      if (!canvas) return
      clearInterval(interval)

      // Draw guide lines after Fabric renders (on the lower canvas)
      const onAfterRender = () => {
        if (activeGuides.length === 0) return

        const el = canvas.lowerCanvasEl
        if (!el) return
        const ctx = el.getContext('2d')
        if (!ctx) return

        const vpt = canvas.viewportTransform
        if (!vpt) return
        const zoom = vpt[0] // uniform zoom assumed

        ctx.save()
        // Apply viewport transform so we draw in scene coordinates
        ctx.transform(vpt[0], vpt[1], vpt[2], vpt[3], vpt[4], vpt[5])

        ctx.strokeStyle = GUIDE_COLOR
        ctx.lineWidth = 1 / zoom
        ctx.setLineDash(GUIDE_DASH.map((v) => v / zoom))

        for (const guide of activeGuides) {
          ctx.beginPath()
          if (guide.orientation === 'vertical') {
            ctx.moveTo(guide.position, guide.start)
            ctx.lineTo(guide.position, guide.end)
          } else {
            ctx.moveTo(guide.start, guide.position)
            ctx.lineTo(guide.end, guide.position)
          }
          ctx.stroke()
        }

        ctx.restore()
      }

      // Clear guides when the user stops interacting
      const onMouseUp = () => {
        if (activeGuides.length > 0) {
          clearGuides()
          canvas.requestRenderAll()
        }
      }

      const onModified = () => {
        if (activeGuides.length > 0) {
          clearGuides()
          canvas.requestRenderAll()
        }
      }

      canvas.on('after:render', onAfterRender)
      canvas.on('mouse:up', onMouseUp)
      canvas.on('object:modified', onModified)

      return () => {
        canvas.off('after:render', onAfterRender)
        canvas.off('mouse:up', onMouseUp)
        canvas.off('object:modified', onModified)
      }
    }, 100)

    return () => clearInterval(interval)
  }, [])
}
