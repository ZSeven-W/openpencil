/**
 * Screenshot capture utilities for design validation.
 * Supports both full root frame and per-node screenshots.
 */

import { useCanvasStore } from '@/stores/canvas-store'
import { DEFAULT_FRAME_ID, useDocumentStore } from '@/stores/document-store'
import type { FabricObjectWithPenId } from '@/canvas/canvas-object-factory'

/**
 * Capture a screenshot of a specific node and all its descendants.
 * Returns a base64 PNG data URL, or null if the node can't be rendered.
 */
export function captureNodeScreenshot(nodeId: string): string | null {
  const canvas = useCanvasStore.getState().fabricCanvas
  if (!canvas) return null

  const store = useDocumentStore.getState()
  if (!store.getNodeById(nodeId)) return null

  const allFlat = store.getFlatNodes()
  const descendantIds = new Set<string>()
  for (const node of allFlat) {
    if (node.id !== nodeId && store.isDescendantOf(node.id, nodeId)) {
      descendantIds.add(node.id)
    }
  }

  const allObjects = canvas.getObjects() as FabricObjectWithPenId[]
  const rootObj = allObjects.find((obj) => obj.penNodeId === nodeId)
  if (!rootObj) return null

  const originX = rootObj.left ?? 0
  const originY = rootObj.top ?? 0
  const w = (rootObj.width ?? 0) * (rootObj.scaleX ?? 1)
  const h = (rootObj.height ?? 0) * (rootObj.scaleY ?? 1)

  if (w <= 0 || h <= 0) return null

  const allIds = new Set(descendantIds)
  allIds.add(nodeId)

  const layerObjects = allObjects.filter(
    (obj) => obj.penNodeId && allIds.has(obj.penNodeId),
  )

  const offscreen = document.createElement('canvas')
  offscreen.width = Math.ceil(w)
  offscreen.height = Math.ceil(h)
  const ctx = offscreen.getContext('2d')
  if (!ctx) return null

  ctx.translate(-originX, -originY)

  for (const obj of layerObjects) {
    obj.render(ctx)
  }

  return offscreen.toDataURL('image/png')
}

/**
 * Capture a screenshot of the entire root frame.
 */
export function captureRootFrameScreenshot(): string | null {
  return captureNodeScreenshot(DEFAULT_FRAME_ID)
}
