import { useEffect } from 'react'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import type { FabricObjectWithPenId } from './canvas-object-factory'
import { resolveTargetAtDepth, getChildIds } from './selection-context'
import { COMPONENT_COLOR, INSTANCE_COLOR } from './canvas-constants'
import type { PenNode } from '@/types/pen'

const HOVER_COLOR = '#3b82f6'
const HOVER_LINE_WIDTH = 1.5
const CHILD_DASH = [4, 4]

function collectReusableIds(nodes: PenNode[], result: Set<string>) {
  for (const node of nodes) {
    if ('reusable' in node && node.reusable === true) result.add(node.id)
    if ('children' in node && node.children) collectReusableIds(node.children, result)
  }
}

function collectInstanceIds(nodes: PenNode[], result: Set<string>) {
  for (const node of nodes) {
    if (node.type === 'ref') result.add(node.id)
    if ('children' in node && node.children) collectInstanceIds(node.children, result)
  }
}

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

        const docChildren = useDocumentStore.getState().document.children
        const reusableIds = new Set<string>()
        const instanceIds = new Set<string>()
        collectReusableIds(docChildren, reusableIds)
        collectInstanceIds(docChildren, instanceIds)

        ctx.save()
        ctx.setTransform(
          vpt[0] * dpr, vpt[1] * dpr,
          vpt[2] * dpr, vpt[3] * dpr,
          vpt[4] * dpr, vpt[5] * dpr,
        )

        // Solid outline on hovered target — skip if already selected
        // (Fabric draws its own selection handles)
        if (!isSelected) {
          drawNodeOutline(ctx, hoveredId, false, zoom, reusableIds, instanceIds)
        }

        // Dashed outlines on direct children — always draw on hover
        const childIds = getChildIds(hoveredId)
        for (const childId of childIds) {
          drawNodeOutline(ctx, childId, true, zoom, reusableIds, instanceIds)
        }

        ctx.restore()
      })

      function drawNodeOutline(
        ctx: CanvasRenderingContext2D,
        nodeId: string,
        dashed: boolean,
        zoom: number,
        reusableIds: Set<string>,
        instanceIds: Set<string>,
      ) {
        const objects = canvas!.getObjects() as FabricObjectWithPenId[]
        const obj = objects.find((o) => o.penNodeId === nodeId)
        if (!obj) return

        const left = obj.left ?? 0
        const top = obj.top ?? 0
        const w = (obj.width ?? 0) * (obj.scaleX ?? 1)
        const h = (obj.height ?? 0) * (obj.scaleY ?? 1)
        const angle = obj.angle ?? 0

        const isReusable = reusableIds.has(nodeId)
        const isInstance = instanceIds.has(nodeId)

        ctx.save()

        if (angle !== 0) {
          const cx = left + w / 2
          const cy = top + h / 2
          ctx.translate(cx, cy)
          ctx.rotate((angle * Math.PI) / 180)
          ctx.translate(-cx, -cy)
        }

        ctx.strokeStyle = isReusable
          ? COMPONENT_COLOR
          : isInstance
            ? INSTANCE_COLOR
            : HOVER_COLOR
        ctx.lineWidth = HOVER_LINE_WIDTH / zoom
        if (isInstance) {
          ctx.setLineDash(CHILD_DASH.map((d) => d / zoom))
        } else if (dashed) {
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
