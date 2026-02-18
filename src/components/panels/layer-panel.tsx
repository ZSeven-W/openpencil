import { useState, useRef, useCallback } from 'react'
import { useDocumentStore } from '@/stores/document-store'
import { useCanvasStore } from '@/stores/canvas-store'
import type { FabricObjectWithPenId } from '@/canvas/canvas-object-factory'
import type { PenNode } from '@/types/pen'
import LayerItem from './layer-item'
import LayerContextMenu from './layer-context-menu'

interface DragState {
  dragId: string | null
  overId: string | null
}

function renderLayerTree(
  nodes: PenNode[],
  depth: number,
  selectedIds: string[],
  handlers: {
    onSelect: (id: string) => void
    onRename: (id: string, name: string) => void
    onToggleVisibility: (id: string) => void
    onToggleLock: (id: string) => void
    onContextMenu: (e: React.MouseEvent, id: string) => void
    onDragStart: (id: string) => void
    onDragOver: (id: string) => void
    onDragEnd: () => void
  },
  dragOverId: string | null,
) {
  return [...nodes].reverse().map((node) => (
    <div key={node.id} className="group">
      <div
        className={
          dragOverId === node.id
            ? 'border-t-2 border-blue-500'
            : 'border-t-2 border-transparent'
        }
      >
        <LayerItem
          id={node.id}
          name={node.name ?? node.type}
          type={node.type}
          depth={depth}
          selected={selectedIds.includes(node.id)}
          visible={node.visible !== false}
          locked={node.locked === true}
          {...handlers}
        />
      </div>
      {'children' in node &&
        node.children &&
        node.children.length > 0 &&
        renderLayerTree(
          node.children,
          depth + 1,
          selectedIds,
          handlers,
          dragOverId,
        )}
    </div>
  ))
}

export default function LayerPanel() {
  const children = useDocumentStore((s) => s.document.children)
  const updateNode = useDocumentStore((s) => s.updateNode)
  const removeNode = useDocumentStore((s) => s.removeNode)
  const duplicateNode = useDocumentStore((s) => s.duplicateNode)
  const toggleVisibility = useDocumentStore((s) => s.toggleVisibility)
  const toggleLock = useDocumentStore((s) => s.toggleLock)
  const groupNodes = useDocumentStore((s) => s.groupNodes)
  const moveNode = useDocumentStore((s) => s.moveNode)
  const getParentOf = useDocumentStore((s) => s.getParentOf)
  const selectedIds = useCanvasStore((s) => s.selection.selectedIds)
  const setSelection = useCanvasStore((s) => s.setSelection)
  const fabricCanvas = useCanvasStore((s) => s.fabricCanvas)

  const [contextMenu, setContextMenu] = useState<{
    x: number
    y: number
    nodeId: string
  } | null>(null)

  const dragRef = useRef<DragState>({ dragId: null, overId: null })
  const [dragOverId, setDragOverId] = useState<string | null>(null)

  const handleSelect = useCallback(
    (id: string) => {
      setSelection([id], id)
      if (fabricCanvas) {
        const objects = fabricCanvas.getObjects()
        const target = objects.find(
          (o) => (o as FabricObjectWithPenId).penNodeId === id,
        )
        if (target) {
          fabricCanvas.setActiveObject(target)
          fabricCanvas.requestRenderAll()
        }
      }
    },
    [fabricCanvas, setSelection],
  )

  const handleRename = useCallback(
    (id: string, name: string) => {
      updateNode(id, { name })
    },
    [updateNode],
  )

  const handleContextMenu = useCallback(
    (e: React.MouseEvent, id: string) => {
      e.preventDefault()
      setContextMenu({ x: e.clientX, y: e.clientY, nodeId: id })
      handleSelect(id)
    },
    [handleSelect],
  )

  const handleDragStart = useCallback((id: string) => {
    dragRef.current.dragId = id
  }, [])

  const handleDragOver = useCallback((id: string) => {
    if (dragRef.current.dragId && dragRef.current.dragId !== id) {
      dragRef.current.overId = id
      setDragOverId(id)
    }
  }, [])

  const handleDragEnd = useCallback(() => {
    const { dragId, overId } = dragRef.current
    if (dragId && overId && dragId !== overId) {
      const parent = getParentOf(overId)
      const parentId = parent ? parent.id : null
      const siblings = parent
        ? ('children' in parent ? parent.children ?? [] : [])
        : children
      const targetIdx = siblings.findIndex((n) => n.id === overId)
      if (targetIdx !== -1) {
        moveNode(dragId, parentId, targetIdx)
      }
    }
    dragRef.current = { dragId: null, overId: null }
    setDragOverId(null)
  }, [children, getParentOf, moveNode])

  const handleContextAction = useCallback(
    (action: string) => {
      if (!contextMenu) return
      const { nodeId } = contextMenu
      switch (action) {
        case 'delete':
          removeNode(nodeId)
          break
        case 'duplicate':
          duplicateNode(nodeId)
          break
        case 'group':
          if (selectedIds.length >= 2) {
            const newGroupId = groupNodes(selectedIds)
            if (newGroupId) {
              setSelection([newGroupId], newGroupId)
            }
          }
          break
        case 'lock':
          toggleLock(nodeId)
          break
        case 'hide':
          toggleVisibility(nodeId)
          break
      }
      setContextMenu(null)
    },
    [
      contextMenu,
      selectedIds,
      removeNode,
      duplicateNode,
      groupNodes,
      toggleLock,
      toggleVisibility,
      setSelection,
    ],
  )

  const handlers = {
    onSelect: handleSelect,
    onRename: handleRename,
    onToggleVisibility: toggleVisibility,
    onToggleLock: toggleLock,
    onContextMenu: handleContextMenu,
    onDragStart: handleDragStart,
    onDragOver: handleDragOver,
    onDragEnd: handleDragEnd,
  }

  return (
    <div className="w-56 bg-card border-r border-border flex flex-col shrink-0">
      <div className="h-8 flex items-center px-3 border-b border-border">
        <span className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
          Layers
        </span>
      </div>
      <div className="flex-1 overflow-y-auto py-1 px-1">
        {children.length === 0 ? (
          <p className="text-xs text-muted-foreground text-center mt-4 px-2">
            No layers yet. Use the toolbar to draw shapes.
          </p>
        ) : (
          renderLayerTree(children, 0, selectedIds, handlers, dragOverId)
        )}
      </div>

      {contextMenu && (
        <LayerContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          nodeId={contextMenu.nodeId}
          canGroup={selectedIds.length >= 2}
          onAction={handleContextAction}
          onClose={() => setContextMenu(null)}
        />
      )}
    </div>
  )
}
