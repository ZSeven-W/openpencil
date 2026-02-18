import { useEffect } from 'react'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import type { FabricObjectWithPenId } from './canvas-object-factory'

const LABEL_FONT_SIZE = 12
const LABEL_OFFSET_Y = 6
const LABEL_COLOR = '#999999'

export function useFrameLabels() {
  useEffect(() => {
    const interval = setInterval(() => {
      const canvas = useCanvasStore.getState().fabricCanvas
      if (!canvas) return
      clearInterval(interval)

      const onAfterRender = () => {
        const el = canvas.lowerCanvasEl
        if (!el) return
        const ctx = el.getContext('2d')
        if (!ctx) return

        const vpt = canvas.viewportTransform
        if (!vpt) return
        const zoom = vpt[0]
        const dpr = el.width / el.offsetWidth

        const store = useDocumentStore.getState()
        // Only top-level document children show labels
        const topIds = new Set(store.document.children.map((c) => c.id))
        const objects = canvas.getObjects() as FabricObjectWithPenId[]

        ctx.save()
        const fontSize = LABEL_FONT_SIZE * dpr
        ctx.font = `500 ${fontSize}px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif`
        ctx.fillStyle = LABEL_COLOR

        for (const obj of objects) {
          if (!obj.penNodeId) continue
          if (!topIds.has(obj.penNodeId)) continue

          const node = store.getNodeById(obj.penNodeId)
          if (!node) continue

          const name = node.name ?? node.type
          const x = ((obj.left ?? 0) * zoom + vpt[4]) * dpr
          const y = ((obj.top ?? 0) * zoom + vpt[5]) * dpr

          ctx.fillText(name, x, y - LABEL_OFFSET_Y * dpr)
        }

        ctx.restore()
      }

      canvas.on('after:render', onAfterRender)

      return () => {
        canvas.off('after:render', onAfterRender)
      }
    }, 100)

    return () => clearInterval(interval)
  }, [])
}
