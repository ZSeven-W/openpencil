import { useEffect } from 'react'
import { useCanvasStore } from '@/stores/canvas-store'
import type { FabricObjectWithPenId } from './canvas-object-factory'

const FRAME_OVERLAY_COLOR = '#3b82f6'
const FRAME_OVERLAY_DASH = [6, 4]
const FRAME_OVERLAY_WIDTH = 2

/**
 * Renders a dashed border around the currently entered frame
 * to indicate the active selection context.
 * Draws on the lower canvas (same pattern as guides / frame labels).
 */
export function useEnteredFrameOverlay() {
  useEffect(() => {
    const interval = setInterval(() => {
      const canvas = useCanvasStore.getState().fabricCanvas
      if (!canvas) return
      clearInterval(interval)

      canvas.on('after:render', () => {
        const { enteredFrameId } = useCanvasStore.getState().selection
        if (!enteredFrameId) return

        const objects = canvas.getObjects() as FabricObjectWithPenId[]
        const frameObj = objects.find((o) => o.penNodeId === enteredFrameId)
        if (!frameObj) return

        const el = canvas.lowerCanvasEl
        if (!el) return
        const ctx = el.getContext('2d')
        if (!ctx) return

        const vpt = canvas.viewportTransform
        if (!vpt) return
        const zoom = vpt[0]
        const dpr = el.width / el.offsetWidth

        const left = frameObj.left ?? 0
        const top = frameObj.top ?? 0
        const w = (frameObj.width ?? 0) * (frameObj.scaleX ?? 1)
        const h = (frameObj.height ?? 0) * (frameObj.scaleY ?? 1)
        const angle = frameObj.angle ?? 0

        ctx.save()
        ctx.setTransform(
          vpt[0] * dpr, vpt[1] * dpr,
          vpt[2] * dpr, vpt[3] * dpr,
          vpt[4] * dpr, vpt[5] * dpr,
        )

        if (angle !== 0) {
          const cx = left + w / 2
          const cy = top + h / 2
          ctx.translate(cx, cy)
          ctx.rotate((angle * Math.PI) / 180)
          ctx.translate(-cx, -cy)
        }

        ctx.strokeStyle = FRAME_OVERLAY_COLOR
        ctx.lineWidth = FRAME_OVERLAY_WIDTH / zoom
        ctx.setLineDash(FRAME_OVERLAY_DASH.map((d) => d / zoom))
        ctx.strokeRect(left, top, w, h)

        ctx.restore()
      })
    }, 100)

    return () => clearInterval(interval)
  }, [])
}
