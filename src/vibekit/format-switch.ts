/**
 * Format switching — resize root frame and reflow content.
 *
 * Uses existing auto-layout engine: fill_container children reflow
 * automatically via computeLayoutPositions(). Only fixed-size nodes
 * need proportional scaling.
 */

import type { PenNode } from '@/types/pen'
import type { FormatPreset } from './format-presets'
import { useDocumentStore } from '@/stores/document-store'
import { useHistoryStore } from '@/stores/history-store'
import { useCanvasStore } from '@/stores/canvas-store'
import { forcePageResync } from '@/canvas/canvas-sync-utils'

/**
 * Switch the active format, resizing all page root frames and scaling
 * fixed-size children proportionally.
 */
export function switchFormat(newFormat: FormatPreset): void {
  const { activeFormat } = useCanvasStore.getState()
  const doc = useDocumentStore.getState().document
  const { startBatch, endBatch } = useHistoryStore.getState()
  const { updateNode } = useDocumentStore.getState()

  const oldW = activeFormat?.width ?? 1080
  const oldH = activeFormat?.height ?? 1350
  const newW = newFormat.width
  const newH = newFormat.height

  if (oldW === newW && oldH === newH) {
    useCanvasStore.getState().setActiveFormat(newFormat)
    return
  }

  const scaleX = newW / oldW
  const scaleY = newH / oldH

  startBatch(doc)

  // Resize all root frames across pages
  const pages = doc.pages ?? []
  for (const page of pages) {
    for (const child of page.children) {
      if (child.type === 'frame') {
        updateNode(child.id, { width: newW, height: newH })
        // Scale fixed-size descendants proportionally
        if ('children' in child && child.children) {
          walkAndScale(child.children, scaleX, scaleY, updateNode)
        }
      }
    }
  }

  // Also handle single-page fallback (children without pages)
  if (!doc.pages || doc.pages.length === 0) {
    for (const child of doc.children) {
      if (child.type === 'frame') {
        updateNode(child.id, { width: newW, height: newH })
        if ('children' in child && child.children) {
          walkAndScale(child.children, scaleX, scaleY, updateNode)
        }
      }
    }
  }

  endBatch()

  useCanvasStore.getState().setActiveFormat(newFormat)
  forcePageResync()
}

/**
 * Walk a node tree and scale fixed-size nodes proportionally.
 * Nodes with fill_container or fit_content sizing are skipped —
 * the layout engine handles those automatically.
 */
function walkAndScale(
  nodes: PenNode[],
  scaleX: number,
  scaleY: number,
  updateNode: (id: string, partial: Partial<PenNode>) => void,
): void {
  for (const node of nodes) {
    const updates: Record<string, unknown> = {}
    let hasUpdates = false

    // Scale position
    if (typeof node.x === 'number') {
      updates.x = Math.round(node.x * scaleX)
      hasUpdates = true
    }
    if (typeof node.y === 'number') {
      updates.y = Math.round(node.y * scaleY)
      hasUpdates = true
    }

    // Scale width/height only for fixed numeric sizes
    const nodeAny = node as unknown as Record<string, unknown>
    if ('width' in node && typeof nodeAny.width === 'number') {
      updates.width = Math.round(nodeAny.width as number * scaleX)
      hasUpdates = true
    }
    if ('height' in node && typeof nodeAny.height === 'number') {
      updates.height = Math.round(nodeAny.height as number * scaleY)
      hasUpdates = true
    }

    // Scale font size proportionally (use average scale)
    if (node.type === 'text' && typeof node.fontSize === 'number') {
      const avgScale = (scaleX + scaleY) / 2
      updates.fontSize = Math.round(node.fontSize * avgScale)
      hasUpdates = true
    }

    if (hasUpdates) {
      updateNode(node.id, updates as Partial<PenNode>)
    }

    // Recurse into children
    if ('children' in node && node.children) {
      walkAndScale(node.children, scaleX, scaleY, updateNode)
    }
  }
}
