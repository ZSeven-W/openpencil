import { useEffect } from 'react'
import { useCanvasStore } from '@/stores/canvas-store'
import type { FabricObjectWithPenId } from './canvas-object-factory'
import { resolveTargetAtDepth, getChildIds } from './selection-context'

const HOVER_COLOR = '#3b82f6'
const HOVER_LINE_WIDTH = 1.5
const CHILD_DASH = [4, 4]

export function useCanvasHover() {
  useEffect(() => {
    const interval = setInterval(() => {
      const canvas = useCanvasStore.getState().fabricCanvas
      if (!canvas) return
      clearInterval(interval)

      // --- Hover tracking via mouse:move ---
      // Always track hoveredId regardless of selection state,
      // so dashed child outlines appear on hover even for selected elements.
      canvas.on('mouse:move', (opt) => {
        const tool = useCanvasStore.getState().activeTool
        if (tool !== 'select') return

        const target = opt.target as FabricObjectWithPenId | null
        const { hoveredId } = useCanvasStore.getState().selection

        if (!target?.penNodeId) {
          if (hoveredId !== null) {
            useCanvasStore.getState().setHoveredId(null)
            canvas.requestRenderAll()
          }
          return
        }

        const resolved = resolveTargetAtDepth(target.penNodeId)

        if (resolved !== hoveredId) {
          useCanvasStore.getState().setHoveredId(resolved)
          canvas.requestRenderAll()
        }
      })

      // Clear hover when mouse leaves the canvas
      canvas.on('mouse:out', () => {
        const { hoveredId } = useCanvasStore.getState().selection
        if (hoveredId !== null) {
          useCanvasStore.getState().setHoveredId(null)
          canvas.requestRenderAll()
        }
      })

      // --- Outline rendering on the lower canvas ---
      canvas.on('after:render', () => {
        const { hoveredId, selectedIds } = useCanvasStore.getState().selection
        if (!hoveredId) return

        const isSelected = selectedIds.includes(hoveredId)

        const el = canvas.lowerCanvasEl
        if (!el) return
        const ctx = el.getContext('2d')
        if (!ctx) return

        const vpt = canvas.viewportTransform
        if (!vpt) return
        const zoom = vpt[0]
        const dpr = el.width / el.offsetWidth

        ctx.save()
        ctx.setTransform(
          vpt[0] * dpr, vpt[1] * dpr,
          vpt[2] * dpr, vpt[3] * dpr,
          vpt[4] * dpr, vpt[5] * dpr,
        )

        // Solid outline on hovered target — skip if already selected
        // (Fabric draws its own selection handles)
        if (!isSelected) {
          drawNodeOutline(ctx, hoveredId, false, zoom)
        }

        // Dashed outlines on direct children — always draw on hover
        const childIds = getChildIds(hoveredId)
        for (const childId of childIds) {
          drawNodeOutline(ctx, childId, true, zoom)
        }

        ctx.restore()
      })

      function drawNodeOutline(
        ctx: CanvasRenderingContext2D,
        nodeId: string,
        dashed: boolean,
        zoom: number,
      ) {
        const objects = canvas!.getObjects() as FabricObjectWithPenId[]
        const obj = objects.find((o) => o.penNodeId === nodeId)
        if (!obj) return

        const left = obj.left ?? 0
        const top = obj.top ?? 0
        const w = (obj.width ?? 0) * (obj.scaleX ?? 1)
        const h = (obj.height ?? 0) * (obj.scaleY ?? 1)
        const angle = obj.angle ?? 0

        ctx.save()

        if (angle !== 0) {
          const cx = left + w / 2
          const cy = top + h / 2
          ctx.translate(cx, cy)
          ctx.rotate((angle * Math.PI) / 180)
          ctx.translate(-cx, -cy)
        }

        ctx.strokeStyle = HOVER_COLOR
        ctx.lineWidth = HOVER_LINE_WIDTH / zoom
        if (dashed) {
          ctx.setLineDash(CHILD_DASH.map((d) => d / zoom))
        } else {
          ctx.setLineDash([])
        }

        ctx.strokeRect(left, top, w, h)
        ctx.restore()
      }
    }, 100)

    return () => clearInterval(interval)
  }, [])
}
