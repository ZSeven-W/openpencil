import { useEffect } from 'react'
import { ActiveSelection } from 'fabric'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import type { ToolType } from '@/types/canvas'

const TOOL_KEYS: Record<string, ToolType> = {
  v: 'select',
  f: 'frame',
  r: 'rectangle',
  o: 'ellipse',
  l: 'line',
  t: 'text',
  p: 'path',
  h: 'hand',
}

export function useKeyboardShortcuts() {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Skip if user is typing in an input
      const target = e.target as HTMLElement
      if (
        target.tagName === 'INPUT' ||
        target.tagName === 'TEXTAREA' ||
        target.isContentEditable
      ) {
        return
      }

      const isMod = e.metaKey || e.ctrlKey

      // Tool shortcuts (single key, no modifier)
      if (!isMod && !e.shiftKey && !e.altKey) {
        const tool = TOOL_KEYS[e.key.toLowerCase()]
        if (tool) {
          e.preventDefault()
          useCanvasStore.getState().setActiveTool(tool)
          return
        }
      }

      // Escape: deselect all
      if (e.key === 'Escape') {
        e.preventDefault()
        useCanvasStore.getState().clearSelection()
        useCanvasStore.getState().setActiveTool('select')
        const canvas = useCanvasStore.getState().fabricCanvas
        if (canvas) {
          canvas.discardActiveObject()
          canvas.requestRenderAll()
        }
        return
      }

      // Delete / Backspace: remove selected elements
      if (e.key === 'Delete' || e.key === 'Backspace') {
        const { selectedIds } = useCanvasStore.getState().selection
        if (selectedIds.length > 0) {
          e.preventDefault()
          for (const id of selectedIds) {
            useDocumentStore.getState().removeNode(id)
          }
          useCanvasStore.getState().clearSelection()
          const canvas = useCanvasStore.getState().fabricCanvas
          if (canvas) {
            canvas.discardActiveObject()
            canvas.requestRenderAll()
          }
        }
        return
      }

      // Cmd+A: select all
      if (isMod && e.key === 'a') {
        e.preventDefault()
        const allNodes = useDocumentStore.getState().getFlatNodes()
        const ids = allNodes.map((n) => n.id)
        useCanvasStore.getState().setSelection(ids, ids[0] ?? null)
        const canvas = useCanvasStore.getState().fabricCanvas
        if (canvas) {
          const objects = canvas.getObjects()
          if (objects.length > 0) {
            const sel = new ActiveSelection(objects, { canvas })
            canvas.setActiveObject(sel)
            canvas.requestRenderAll()
          }
        }
        return
      }

      // [ ] : reorder layers
      if (e.key === '[') {
        e.preventDefault()
        const { selectedIds } = useCanvasStore.getState().selection
        for (const id of selectedIds) {
          useDocumentStore.getState().reorderNode(id, 'down')
        }
        return
      }
      if (e.key === ']') {
        e.preventDefault()
        const { selectedIds } = useCanvasStore.getState().selection
        for (const id of selectedIds) {
          useDocumentStore.getState().reorderNode(id, 'up')
        }
        return
      }

      // Arrow keys: nudge
      const nudgeKeys = ['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight']
      if (nudgeKeys.includes(e.key) && !isMod) {
        const { selectedIds } = useCanvasStore.getState().selection
        if (selectedIds.length === 0) return
        e.preventDefault()
        const amount = e.shiftKey ? 10 : 1
        for (const id of selectedIds) {
          const node = useDocumentStore.getState().getNodeById(id)
          if (!node) continue
          const updates: Record<string, number> = {}
          if (e.key === 'ArrowLeft') updates.x = (node.x ?? 0) - amount
          if (e.key === 'ArrowRight') updates.x = (node.x ?? 0) + amount
          if (e.key === 'ArrowUp') updates.y = (node.y ?? 0) - amount
          if (e.key === 'ArrowDown') updates.y = (node.y ?? 0) + amount
          useDocumentStore.getState().updateNode(id, updates)
        }
        return
      }
    }

    document.addEventListener('keydown', handleKeyDown)
    return () => document.removeEventListener('keydown', handleKeyDown)
  }, [])
}
