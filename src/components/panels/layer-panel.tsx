import { useState, useRef, useCallback } from 'react'
import { useDocumentStore, findNodeInTree } from '@/stores/document-store'
import { useCanvasStore } from '@/stores/canvas-store'
import { setSkipNextDepthResolve } from '@/canvas/use-canvas-selection'
import type { FabricObjectWithPenId } from '@/canvas/canvas-object-factory'
import type { PenNode } from '@/types/pen'
import LayerItem from './layer-item'
import type { DropPosition } from './layer-item'
import LayerContextMenu from './layer-context-menu'

const CONTAINER_TYPES = new Set(['frame', 'group', 'ref'])

interface DragState {
  dragId: string | null
  overId: string | null
  dropPosition: DropPosition
}

function isNodeReusable(node: PenNode, parentReusable: boolean): boolean {
  if (parentReusable) return true
  return 'reusable' in node && node.reusable === true
}

/** Get effective children for a node, resolving RefNode instances. */
function getEffectiveChildren(
  node: PenNode,
  allChildren: PenNode[],
): PenNode[] | null {
  if (node.type === 'ref') {
    const component = findNodeInTree(allChildren, node.ref)
    if (component && 'children' in component && component.children?.length) {
      return component.children
    }
    return null
  }
  return 'children' in node && node.children && node.children.length > 0
    ? node.children
    : null
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
    onToggleExpand: (id: string) => void
    onContextMenu: (e: React.MouseEvent, id: string) => void
    onDragStart: (id: string) => void
    onDragOver: (id: string, e: React.PointerEvent) => void
    onDragEnd: () => void
  },
  dragOverId: string | null,
  dropPosition: DropPosition,
  collapsedIds: Set<string>,
  allChildren: PenNode[],
  parentReusable = false,
  parentIsInstance = false,
) {
  return [...nodes].reverse().map((node) => {
    const nodeChildren = getEffectiveChildren(node, allChildren)
    const isExpanded = !collapsedIds.has(node.id)
    const isDropTarget = dragOverId === node.id
    const isInstance = node.type === 'ref' || parentIsInstance
    const reusable = isNodeReusable(node, parentReusable)

    return (
      <div key={node.id}>
        <LayerItem
          id={node.id}
          name={node.name ?? node.type}
          type={node.type}
          depth={depth}
          selected={selectedIds.includes(node.id)}
          visible={node.visible !== false}
          locked={node.locked === true}
          hasChildren={nodeChildren !== null}
          expanded={isExpanded}
          isReusable={reusable}
          isInstance={isInstance}
          dropPosition={isDropTarget ? dropPosition : null}
          {...handlers}
        />
        {nodeChildren &&
          isExpanded &&
          renderLayerTree(
            nodeChildren,
            depth + 1,
            selectedIds,
            handlers,
            dragOverId,
            dropPosition,
            collapsedIds,
            allChildren,
            reusable,
            isInstance,
          )}
      </div>
    )
  })
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
  const getNodeById = useDocumentStore((s) => s.getNodeById)
  const isDescendantOf = useDocumentStore((s) => s.isDescendantOf)
  const selectedIds = useCanvasStore((s) => s.selection.selectedIds)
  const setSelection = useCanvasStore((s) => s.setSelection)
  const fabricCanvas = useCanvasStore((s) => s.fabricCanvas)

  const [contextMenu, setContextMenu] = useState<{
    x: number
    y: number
    nodeId: string
  } | null>(null)

  const dragRef = useRef<DragState>({
    dragId: null,
    overId: null,
    dropPosition: null,
  })
  const [dragOverId, setDragOverId] = useState<string | null>(null)
  const [dropPosition, setDropPosition] = useState<DropPosition>(null)
  const [collapsedIds, setCollapsedIds] = useState<Set<string>>(new Set())

  const handleToggleExpand = useCallback((id: string) => {
    setCollapsedIds((prev) => {
      const next = new Set(prev)
      if (next.has(id)) {
        next.delete(id)
      } else {
        next.add(id)
      }
      return next
    })
  }, [])

  const handleSelect = useCallback(
    (id: string) => {
      setSelection([id], id)
      if (fabricCanvas) {
        const objects = fabricCanvas.getObjects()
        const target = objects.find(
          (o) => (o as FabricObjectWithPenId).penNodeId === id,
        )
        if (target) {
          setSkipNextDepthResolve()
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

  const handleDragOver = useCallback(
    (id: string, e: React.PointerEvent) => {
      const { dragId } = dragRef.current
      if (!dragId || dragId === id) return

      // Prevent dropping into own descendants
      if (isDescendantOf(id, dragId)) return

      const rect = e.currentTarget.getBoundingClientRect()
      const y = e.clientY - rect.top
      const ratio = y / rect.height
      const targetNode = getNodeById(id)
      const canBeParent = targetNode
        ? CONTAINER_TYPES.has(targetNode.type)
        : false

      let pos: DropPosition
      if (canBeParent) {
        if (ratio < 0.25) pos = 'above'
        else if (ratio > 0.75) pos = 'below'
        else pos = 'inside'
      } else {
        pos = ratio < 0.5 ? 'above' : 'below'
      }

      dragRef.current.overId = id
      dragRef.current.dropPosition = pos
      setDragOverId(id)
      setDropPosition(pos)
    },
    [getNodeById, isDescendantOf],
  )

  const handleDragEnd = useCallback(() => {
    const { dragId, overId, dropPosition: pos } = dragRef.current
    if (dragId && overId && dragId !== overId && pos) {
      const parent = getParentOf(overId)
      const parentId = parent ? parent.id : null
      const siblings = parent
        ? ('children' in parent ? parent.children ?? [] : [])
        : children
      const targetIdx = siblings.findIndex((n) => n.id === overId)

      if (pos === 'inside') {
        moveNode(dragId, overId, 0)
        // Auto-expand the target so the dropped item is visible
        setCollapsedIds((prev) => {
          const next = new Set(prev)
          next.delete(overId)
          return next
        })
      } else if (targetIdx !== -1) {
        const insertIdx = pos === 'above' ? targetIdx : targetIdx + 1
        moveNode(dragId, parentId, insertIdx)
      }
    }
    dragRef.current = { dragId: null, overId: null, dropPosition: null }
    setDragOverId(null)
    setDropPosition(null)
  }, [children, getParentOf, moveNode])

  const makeReusable = useDocumentStore((s) => s.makeReusable)
  const detachComponent = useDocumentStore((s) => s.detachComponent)

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
        case 'make-component':
          makeReusable(nodeId)
          break
        case 'detach-component':
          detachComponent(nodeId)
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
      makeReusable,
      detachComponent,
    ],
  )

  const handlers = {
    onSelect: handleSelect,
    onRename: handleRename,
    onToggleVisibility: toggleVisibility,
    onToggleLock: toggleLock,
    onToggleExpand: handleToggleExpand,
    onContextMenu: handleContextMenu,
    onDragStart: handleDragStart,
    onDragOver: handleDragOver,
    onDragEnd: handleDragEnd,
  }

  return (
    <div className="w-56 bg-card border-r border-border flex flex-col shrink-0">
      <div className="h-8 flex items-center px-3 border-b border-border">
        <span className="text-xs font-medium text-muted-foreground tracking-wider">
          Layers
        </span>
      </div>
      <div className="flex-1 overflow-y-auto py-1 px-1">
        {children.length === 0 ? (
          <p className="text-xs text-muted-foreground text-center mt-4 px-2">
            No layers yet. Use the toolbar to draw shapes.
          </p>
        ) : (
          renderLayerTree(
            children,
            0,
            selectedIds,
            handlers,
            dragOverId,
            dropPosition,
            collapsedIds,
            children,
          )
        )}
      </div>

      {contextMenu && (() => {
        const contextNode = getNodeById(contextMenu.nodeId)
        const isContainer = contextNode
          ? contextNode.type === 'frame' || contextNode.type === 'group' || contextNode.type === 'rectangle'
          : false
        const nodeIsReusable = contextNode
          ? 'reusable' in contextNode && contextNode.reusable === true
          : false
        const nodeIsInstance = contextNode?.type === 'ref'
        return (
          <LayerContextMenu
            x={contextMenu.x}
            y={contextMenu.y}
            nodeId={contextMenu.nodeId}
            canGroup={selectedIds.length >= 2}
            canCreateComponent={isContainer && !nodeIsReusable}
            isReusable={nodeIsReusable}
            isInstance={nodeIsInstance}
            onAction={handleContextAction}
            onClose={() => setContextMenu(null)}
          />
        )
      })()}
    </div>
  )
}
