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
  console.debug('[figma-paste] Figma markers detected, extracting clipboard data...')

  const clipData = extractFigmaClipboardData(html)
  if (!clipData) {
    console.warn('[figma-paste] Failed to extract clipboard data from HTML')
    return false
  }

  console.debug('[figma-paste] Extracted clipboard data, meta:', clipData.meta,
    'buffer size:', clipData.buffer.byteLength, 'bytes')

  const { nodes, warnings } = figmaClipboardToNodes(clipData.buffer)
  console.debug('[figma-paste] Converted', nodes.length, 'nodes, warnings:', warnings)

  if (nodes.length === 0) {
    console.warn('[figma-paste] No convertible nodes found:', warnings)
    return false
  }

  // Center pasted nodes at viewport center
  const bounds = computeBounds(nodes)
  const { cx, cy } = getViewportCenter()
  const offsetX = cx - (bounds.minX + bounds.maxX) / 2
  const offsetY = cy - (bounds.minY + bounds.maxY) / 2

  console.debug('[figma-paste] Bounds:', bounds, 'viewport center:', { cx, cy },
    'offset:', { offsetX, offsetY })

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

  console.debug('[figma-paste] Successfully pasted', newIds.length, 'nodes:', newIds)
  return true
}

/**
 * Try reading Figma data from the system clipboard via Clipboard API.
 * Used as a fallback when the `paste` event might not fire
 * (e.g. when a non-editable element like <canvas> has focus).
 */
export async function tryPasteFigmaFromClipboard(): Promise<boolean> {
  try {
    // Try modern Clipboard API first
    if (navigator.clipboard?.read) {
      console.debug('[figma-paste] Reading clipboard via Clipboard API...')
      const items = await navigator.clipboard.read()
      for (const item of items) {
        if (item.types.includes('text/html')) {
          const blob = await item.getType('text/html')
          const html = await blob.text()
          console.debug('[figma-paste] Got HTML from clipboard, length:', html.length,
            'has figma markers:', isFigmaClipboardHtml(html))
          if (isFigmaClipboardHtml(html)) {
            return processFigmaHtml(html)
          }
        }
      }
      console.debug('[figma-paste] No Figma data found in clipboard items')
    } else {
      console.debug('[figma-paste] Clipboard API not available')
    }
  } catch (err) {
    console.warn('[figma-paste] Clipboard API read failed:', err)
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
      console.debug('[figma-paste] paste event fired, target:', (e.target as HTMLElement)?.tagName)

      // Skip if user is typing in an input/textarea/contentEditable
      const target = e.target as HTMLElement
      if (
        target.tagName === 'INPUT' ||
        target.tagName === 'TEXTAREA' ||
        target.isContentEditable
      ) {
        console.debug('[figma-paste] Skipping — editable element focused')
        return
      }

      const html = e.clipboardData?.getData('text/html')
      console.debug('[figma-paste] clipboard HTML length:', html?.length ?? 0,
        'has figma markers:', html ? isFigmaClipboardHtml(html) : false)

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
