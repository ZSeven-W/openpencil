import { useEffect } from 'react'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore, getActivePageChildren } from '@/stores/document-store'
import { COMPONENT_COLOR, INSTANCE_COLOR } from './canvas-constants'
import type { FabricObjectWithPenId } from './canvas-object-factory'
import type { PenNode } from '@/types/pen'

const LABEL_FONT_SIZE = 12
const LABEL_OFFSET_Y = 6
const LABEL_COLOR = '#999999'

/** Collect IDs of all nodes with reusable: true in the tree. */
function collectReusableIds(nodes: PenNode[], result: Set<string>) {
  for (const node of nodes) {
    if ('reusable' in node && node.reusable === true) {
      result.add(node.id)
    }
    if ('children' in node && node.children) {
      collectReusableIds(node.children, result)
    }
  }
}

/** Collect IDs of all RefNodes (instances) in the tree. */
function collectInstanceIds(nodes: PenNode[], result: Set<string>) {
  for (const node of nodes) {
    if (node.type === 'ref') {
      result.add(node.id)
    }
    if ('children' in node && node.children) {
      collectInstanceIds(node.children, result)
    }
  }
}

export function useFrameLabels() {
  useEffect(() => {
    let detach: (() => void) | null = null

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
        if (!Number.isFinite(zoom) || zoom <= 0) return
        if (!el.offsetWidth) return
        const dpr = el.width / el.offsetWidth

        const store = useDocumentStore.getState()
        const pageChildren = getActivePageChildren(store.document, useCanvasStore.getState().activePageId)
        // Top-level frame nodes show labels
        const topFrameIds = new Set(
          pageChildren
            .filter((c) => c.type === 'frame')
            .map((c) => c.id),
        )
        // Reusable component IDs (at any depth)
        const reusableIds = new Set<string>()
        collectReusableIds(pageChildren, reusableIds)
        // Instance (RefNode) IDs
        const instanceIds = new Set<string>()
        collectInstanceIds(pageChildren, instanceIds)

        const objects = canvas.getObjects() as FabricObjectWithPenId[]

        ctx.save()
        ctx.setTransform(
          vpt[0] * dpr, vpt[1] * dpr,
          vpt[2] * dpr, vpt[3] * dpr,
          vpt[4] * dpr, vpt[5] * dpr,
        )
        ctx.font = `500 ${LABEL_FONT_SIZE / zoom}px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif`
        ctx.textBaseline = 'bottom'

        for (const obj of objects) {
          if (!obj.penNodeId) continue

          const isTopFrame = topFrameIds.has(obj.penNodeId)
          const isReusable = reusableIds.has(obj.penNodeId)
          const isInstance = instanceIds.has(obj.penNodeId)

          if (!isTopFrame && !isReusable && !isInstance) continue

          const node = store.getNodeById(obj.penNodeId)
          if (!node) continue

          const name = node.name ?? node.type
          const corners = obj.getCoords()
          const x = Math.min(...corners.map((p) => p.x))
          const y = Math.min(...corners.map((p) => p.y))

          ctx.fillStyle = isReusable
            ? COMPONENT_COLOR
            : isInstance
              ? INSTANCE_COLOR
              : LABEL_COLOR
          ctx.fillText(name, x, y - LABEL_OFFSET_Y / zoom)
        }

        ctx.restore()
      }

      canvas.on('after:render', onAfterRender)
      detach = () => {
        canvas.off('after:render', onAfterRender)
      }
    }, 100)

    return () => {
      clearInterval(interval)
      detach?.()
    }
  }, [])
}
