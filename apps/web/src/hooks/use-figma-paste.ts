import { useEffect } from 'react'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import { useHistoryStore } from '@/stores/history-store'
import { getCanvasSize } from '@/canvas/skia-engine-ref'
import {
  isFigmaClipboardHtml,
  extractFigmaClipboardData,
  figmaClipboardToNodes,
} from '@/services/figma/figma-clipboard'
import type { PenNode } from '@/types/pen'

/**
 * Compute the bounding box of a set of PenNodes.
 */
function computeBounds(nodes: PenNode[]): { minX: number; minY: number; maxX: number; maxY: number } {
  let minX = Infinity
  let minY = Infinity
  let maxX = -Infinity
  let maxY = -Infinity

  for (const node of nodes) {
    const x = node.x ?? 0
    const y = node.y ?? 0

    let right: number
    let bottom: number
    if (node.type === 'line') {
      right = Math.max(x, node.x2 ?? x)
      bottom = Math.max(y, node.y2 ?? y)
    } else {
      const w = 'width' in node && typeof node.width === 'number' ? node.width : 100
      const h = 'height' in node && typeof node.height === 'number' ? node.height : 100
      right = x + w
      bottom = y + h
    }

    minX = Math.min(minX, x)
    minY = Math.min(minY, y)
    maxX = Math.max(maxX, right)
    maxY = Math.max(maxY, bottom)
  }

  return { minX, minY, maxX, maxY }
}

/**
 * Get the viewport center in scene coordinates using the Skia canvas viewport.
 */
function getViewportCenter(): { cx: number; cy: number } {
  const { viewport } = useCanvasStore.getState()
  const { width, height } = getCanvasSize()
  const cx = (-viewport.panX + width / 2) / viewport.zoom
  const cy = (-viewport.panY + height / 2) / viewport.zoom
  return { cx, cy }
}

/**
 * Process Figma HTML clipboard data — extract, decode, and add to canvas.
 * Returns true if Figma nodes were pasted.
 */
function processFigmaHtml(html: string): boolean {
  const clipData = extractFigmaClipboardData(html)
  if (!clipData) return false

  const { nodes } = figmaClipboardToNodes(clipData.buffer, html)
  if (nodes.length === 0) return false

  // Center pasted nodes at viewport center
  const bounds = computeBounds(nodes)
  const { cx, cy } = getViewportCenter()
  const offsetX = cx - (bounds.minX + bounds.maxX) / 2
  const offsetY = cy - (bounds.minY + bounds.maxY) / 2

  for (const node of nodes) {
    node.x = (node.x ?? 0) + offsetX
    node.y = (node.y ?? 0) + offsetY
  }

  // Batch all insertions into a single undo step
  const doc = useDocumentStore.getState().document
  useHistoryStore.getState().startBatch(doc)

  const newIds: string[] = []
  for (const node of nodes) {
    useDocumentStore.getState().addNode(null, node)
    newIds.push(node.id)
  }

  useHistoryStore.getState().endBatch(useDocumentStore.getState().document)

  // Select the pasted nodes
  useCanvasStore.getState().setSelection(newIds, newIds[0] ?? null)
  return true
}

/**
 * Try reading Figma data from the system clipboard via Clipboard API.
 * Used as a fallback when the `paste` event might not fire
 * (e.g. when a non-editable element like <canvas> has focus).
 */
export async function tryPasteFigmaFromClipboard(): Promise<boolean> {
  try {
    if (navigator.clipboard?.read) {
      const items = await navigator.clipboard.read()
      for (const item of items) {
        if (item.types.includes('text/html')) {
          const blob = await item.getType('text/html')
          const html = await blob.text()
          if (isFigmaClipboardHtml(html)) {
            return processFigmaHtml(html)
          }
        }
      }
    }
  } catch {
    // Clipboard API may not be available or permission denied
  }
  return false
}

/**
 * Listens for browser `paste` events to detect Figma clipboard data.
 * Also provides `tryPasteFigmaFromClipboard()` for use from the keydown
 * handler as a fallback when the paste event might not fire.
 */
export function useFigmaPaste() {
  useEffect(() => {
    const handlePaste = (e: ClipboardEvent) => {
      // Skip if user is typing in an input/textarea/contentEditable
      const target = e.target as HTMLElement
      if (
        target.tagName === 'INPUT' ||
        target.tagName === 'TEXTAREA' ||
        target.isContentEditable
      ) {
        return
      }

      const html = e.clipboardData?.getData('text/html')
      if (!html || !isFigmaClipboardHtml(html)) return

      e.preventDefault()

      try {
        processFigmaHtml(html)
      } catch (err) {
        console.error('[figma-paste] Failed to paste Figma clipboard data:', err)
      }
    }

    document.addEventListener('paste', handlePaste)
    return () => document.removeEventListener('paste', handlePaste)
  }, [])
}
