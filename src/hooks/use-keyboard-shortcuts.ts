import { useEffect } from 'react'
import { ActiveSelection } from 'fabric'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import { useHistoryStore } from '@/stores/history-store'
import { cloneNodesWithNewIds } from '@/utils/node-clone'
import {
  supportsFileSystemAccess,
  writeToFileHandle,
  saveDocumentAs,
  downloadDocument,
  openDocumentFS,
  openDocument,
} from '@/utils/file-operations'
import { syncCanvasPositionsToStore } from '@/canvas/use-canvas-sync'
import type { FabricObjectWithPenId } from '@/canvas/canvas-object-factory'
import { zoomToFitContent } from '@/canvas/use-fabric-canvas'
import { isPenToolActive, penToolKeyDown } from '@/canvas/pen-tool'
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

      // During pen tool drawing, handle Enter/Escape/Backspace specially
      if (isPenToolActive()) {
        const canvas = useCanvasStore.getState().fabricCanvas
        if (canvas && penToolKeyDown(canvas, e.key)) {
          e.preventDefault()
          return
        }
      }

      const isMod = e.metaKey || e.ctrlKey

      // Undo: Cmd/Ctrl+Z
      if (isMod && e.key === 'z' && !e.shiftKey) {
        e.preventDefault()
        const currentDoc = useDocumentStore.getState().document
        const prev = useHistoryStore.getState().undo(currentDoc)
        if (prev) {
          useDocumentStore.getState().applyHistoryState(prev)
        }
        return
      }

      // Redo: Cmd/Ctrl+Shift+Z
      if (isMod && e.key === 'z' && e.shiftKey) {
        e.preventDefault()
        const currentDoc = useDocumentStore.getState().document
        const next = useHistoryStore.getState().redo(currentDoc)
        if (next) {
          useDocumentStore.getState().applyHistoryState(next)
        }
        return
      }

      // Copy: Cmd/Ctrl+C
      if (isMod && e.key === 'c' && !e.shiftKey) {
        const { selectedIds } = useCanvasStore.getState().selection
        if (selectedIds.length > 0) {
          e.preventDefault()
          const nodes = selectedIds
            .map((id) => useDocumentStore.getState().getNodeById(id))
            .filter((n): n is NonNullable<typeof n> => n != null)
          useCanvasStore.getState().setClipboard(structuredClone(nodes))
        }
        return
      }

      // Cut: Cmd/Ctrl+X
      if (isMod && e.key === 'x' && !e.shiftKey) {
        const { selectedIds } = useCanvasStore.getState().selection
        if (selectedIds.length > 0) {
          e.preventDefault()
          const nodes = selectedIds
            .map((id) => useDocumentStore.getState().getNodeById(id))
            .filter((n): n is NonNullable<typeof n> => n != null)
          useCanvasStore.getState().setClipboard(structuredClone(nodes))
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

      // Paste: Cmd/Ctrl+V
      if (isMod && e.key === 'v' && !e.shiftKey) {
        const { clipboard } = useCanvasStore.getState()
        if (clipboard.length > 0) {
          e.preventDefault()
          const cloned = cloneNodesWithNewIds(clipboard, 10)
          const newIds: string[] = []
          for (const node of cloned) {
            useDocumentStore.getState().addNode(null, node)
            newIds.push(node.id)
          }
          useCanvasStore.getState().setSelection(newIds, newIds[0] ?? null)
        }
        return
      }

      // Duplicate: Cmd/Ctrl+D
      if (isMod && e.key === 'd') {
        const { selectedIds } = useCanvasStore.getState().selection
        if (selectedIds.length > 0) {
          e.preventDefault()
          const nodes = selectedIds
            .map((id) => useDocumentStore.getState().getNodeById(id))
            .filter((n): n is NonNullable<typeof n> => n != null)
          const cloned = cloneNodesWithNewIds(nodes, 10)
          const newIds: string[] = []
          for (const node of cloned) {
            useDocumentStore.getState().addNode(null, node)
            newIds.push(node.id)
          }
          useCanvasStore.getState().setSelection(newIds, newIds[0] ?? null)
        }
        return
      }

      // Save: Cmd/Ctrl+S
      if (isMod && e.key === 's' && !e.shiftKey) {
        e.preventDefault()
        // Force-sync all Fabric object positions to the store before serializing
        syncCanvasPositionsToStore()
        const store = useDocumentStore.getState()
        const { document: doc, fileName, fileHandle } = store

        if (fileHandle) {
          writeToFileHandle(fileHandle, doc).then(() => store.markClean())
        } else if (supportsFileSystemAccess()) {
          saveDocumentAs(doc, fileName ?? 'untitled.pen').then((result) => {
            if (result) {
              useDocumentStore.setState({
                fileName: result.fileName,
                fileHandle: result.handle,
                isDirty: false,
              })
            }
          })
        } else if (fileName) {
          downloadDocument(doc, fileName)
          store.markClean()
        } else {
          store.setSaveDialogOpen(true)
        }
        return
      }

      // Open: Cmd/Ctrl+O
      if (isMod && e.key === 'o' && !e.shiftKey) {
        e.preventDefault()
        if (supportsFileSystemAccess()) {
          openDocumentFS().then((result) => {
            if (result) {
              useDocumentStore
                .getState()
                .loadDocument(result.doc, result.fileName, result.handle)
              requestAnimationFrame(() => zoomToFitContent())
            }
          })
        } else {
          openDocument().then((result) => {
            if (result) {
              useDocumentStore
                .getState()
                .loadDocument(result.doc, result.fileName)
              requestAnimationFrame(() => zoomToFitContent())
            }
          })
        }
        return
      }

      // Group: Cmd/Ctrl+G
      if (isMod && e.key === 'g' && !e.shiftKey) {
        const { selectedIds } = useCanvasStore.getState().selection
        if (selectedIds.length >= 2) {
          e.preventDefault()
          const groupId = useDocumentStore.getState().groupNodes(selectedIds)
          if (groupId) {
            useCanvasStore.getState().setSelection([groupId], groupId)
          }
        }
        return
      }

      // Ungroup: Cmd/Ctrl+Shift+G
      if (isMod && e.shiftKey && e.key.toLowerCase() === 'g') {
        const { selectedIds } = useCanvasStore.getState().selection
        if (selectedIds.length === 1) {
          e.preventDefault()
          const node = useDocumentStore.getState().getNodeById(selectedIds[0])
          if (node && node.type === 'group' && 'children' in node && node.children) {
            const childIds = node.children.map((c) => c.id)
            useDocumentStore.getState().ungroupNode(selectedIds[0])
            useCanvasStore.getState().setSelection(childIds, childIds[0] ?? null)
          }
        }
        return
      }

      // Tool shortcuts (single key, no modifier)
      if (!isMod && !e.shiftKey && !e.altKey) {
        const tool = TOOL_KEYS[e.key.toLowerCase()]
        if (tool) {
          e.preventDefault()
          useCanvasStore.getState().setActiveTool(tool)
          return
        }
      }

      // Escape: 1) clear selection, 2) exit frame, 3) switch to select tool
      if (e.key === 'Escape') {
        e.preventDefault()
        const { selectedIds, enteredFrameId } = useCanvasStore.getState().selection
        const canvas = useCanvasStore.getState().fabricCanvas

        if (selectedIds.length > 0) {
          // Step 1: clear current selection
          useCanvasStore.getState().clearSelection()
          if (canvas) {
            canvas.discardActiveObject()
            canvas.requestRenderAll()
          }
        } else if (enteredFrameId) {
          // Step 2: exit entered frame
          useCanvasStore.getState().exitFrame()
          if (canvas) {
            canvas.discardActiveObject()
            canvas.requestRenderAll()
          }
        } else {
          // Step 3: switch to select tool
          useCanvasStore.getState().setActiveTool('select')
          if (canvas) {
            canvas.discardActiveObject()
            canvas.requestRenderAll()
          }
        }
        return
      }

      // Delete / Backspace: remove selected elements
      if (e.key === 'Delete' || e.key === 'Backspace') {
        const { selectedIds } = useCanvasStore.getState().selection
        if (selectedIds.length > 0) {
          e.preventDefault()
          if (selectedIds.length > 1) {
            useHistoryStore
              .getState()
              .beginBatch(
                useDocumentStore.getState().document.children,
              )
          }
          for (const id of selectedIds) {
            useDocumentStore.getState().removeNode(id)
          }
          if (selectedIds.length > 1) {
            useHistoryStore
              .getState()
              .endBatch(useDocumentStore.getState().document)
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

      // Cmd+A: select all (top-level nodes only, matching manual selection behavior)
      if (isMod && e.key === 'a') {
        e.preventDefault()
        const topLevelNodes = useDocumentStore.getState().document.children
        const ids = topLevelNodes.map((n) => n.id)
        useCanvasStore.getState().setSelection(ids, ids[0] ?? null)
        const canvas = useCanvasStore.getState().fabricCanvas
        if (canvas) {
          const topLevelSet = new Set(ids)
          const objects = (
            canvas.getObjects() as FabricObjectWithPenId[]
          ).filter((obj) => obj.penNodeId && topLevelSet.has(obj.penNodeId))
          if (objects.length === 1) {
            canvas.setActiveObject(objects[0])
            canvas.requestRenderAll()
          } else if (objects.length > 1) {
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
        if (selectedIds.length > 1) {
          useHistoryStore
            .getState()
            .beginBatch(
              useDocumentStore.getState().document.children,
            )
        }
        for (const id of selectedIds) {
          useDocumentStore.getState().reorderNode(id, 'down')
        }
        if (selectedIds.length > 1) {
          useHistoryStore
            .getState()
            .endBatch(useDocumentStore.getState().document)
        }
        return
      }
      if (e.key === ']') {
        e.preventDefault()
        const { selectedIds } = useCanvasStore.getState().selection
        if (selectedIds.length > 1) {
          useHistoryStore
            .getState()
            .beginBatch(
              useDocumentStore.getState().document.children,
            )
        }
        for (const id of selectedIds) {
          useDocumentStore.getState().reorderNode(id, 'up')
        }
        if (selectedIds.length > 1) {
          useHistoryStore
            .getState()
            .endBatch(useDocumentStore.getState().document)
        }
        return
      }

      // Arrow keys: nudge
      const nudgeKeys = ['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight']
      if (nudgeKeys.includes(e.key) && !isMod) {
        const { selectedIds } = useCanvasStore.getState().selection
        if (selectedIds.length === 0) return
        e.preventDefault()
        if (selectedIds.length > 1) {
          useHistoryStore
            .getState()
            .beginBatch(
              useDocumentStore.getState().document.children,
            )
        }
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
        if (selectedIds.length > 1) {
          useHistoryStore
            .getState()
            .endBatch(useDocumentStore.getState().document)
        }
        return
      }
    }

    document.addEventListener('keydown', handleKeyDown)
    return () => document.removeEventListener('keydown', handleKeyDown)
  }, [])
}
