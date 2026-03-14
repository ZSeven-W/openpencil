import { useState, useRef, useCallback, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { useDocumentStore, findNodeInTree, getActivePageChildren } from '@/stores/document-store'
import { useCanvasStore } from '@/stores/canvas-store'
import type { PenNode } from '@/types/pen'
import { useHistoryStore } from '@/stores/history-store'
import { canBooleanOp, executeBooleanOp, type BooleanOpType } from '@/utils/boolean-ops'
import LayerItem from './layer-item'
import type { DropPosition } from './layer-item'
import LayerContextMenu from './layer-context-menu'
import PageTabs from '@/components/editor/page-tabs'

const CONTAINER_TYPES = new Set(['frame', 'group', 'ref'])

const LAYER_MIN_WIDTH = 180
const LAYER_MAX_WIDTH = 480
const LAYER_DEFAULT_WIDTH = 224 // w-56

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
  return nodes.map((node) => {
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

function collectCollapsibleNodeIds(
  nodes: PenNode[],
  allChildren: PenNode[],
  result: Set<string> = new Set(),
): Set<string> {
  for (const node of nodes) {
    const nodeChildren = getEffectiveChildren(node, allChildren)
    if (!nodeChildren) continue
    result.add(node.id)
    collectCollapsibleNodeIds(nodeChildren, allChildren, result)
  }
  return result
}

export default function LayerPanel() {
  const { t } = useTranslation()
  const [panelWidth, setPanelWidth] = useState(LAYER_DEFAULT_WIDTH)
  const isDraggingResize = useRef(false)
  const resizeStartX = useRef(0)
  const resizeStartWidth = useRef(0)

  const handleResizeMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault()
    isDraggingResize.current = true
    resizeStartX.current = e.clientX
    resizeStartWidth.current = panelWidth

    const handleMouseMove = (ev: MouseEvent) => {
      if (!isDraggingResize.current) return
      const delta = ev.clientX - resizeStartX.current
      const newWidth = Math.max(LAYER_MIN_WIDTH, Math.min(LAYER_MAX_WIDTH, resizeStartWidth.current + delta))
      setPanelWidth(newWidth)
    }

    const handleMouseUp = () => {
      isDraggingResize.current = false
      document.removeEventListener('mousemove', handleMouseMove)
      document.removeEventListener('mouseup', handleMouseUp)
      document.body.style.cursor = ''
      document.body.style.userSelect = ''
    }

    document.body.style.cursor = 'col-resize'
    document.body.style.userSelect = 'none'
    document.addEventListener('mousemove', handleMouseMove)
    document.addEventListener('mouseup', handleMouseUp)
  }, [panelWidth])

  const activePageId = useCanvasStore((s) => s.activePageId)
  const children = useDocumentStore((s) => getActivePageChildren(s.document, activePageId))
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
  const [collapsedIds, setCollapsedIds] = useState<Set<string>>(
    () => collectCollapsibleNodeIds(children, children),
  )
  const knownCollapsibleIdsRef = useRef<Set<string>>(new Set())

  useEffect(() => {
    const currentCollapsibleIds = collectCollapsibleNodeIds(children, children)
    const known = knownCollapsibleIdsRef.current

    setCollapsedIds((prev) => {
      const next = new Set<string>()
      for (const id of currentCollapsibleIds) {
        const isNewNode = !known.has(id)
        if (isNewNode || prev.has(id)) {
          next.add(id)
        }
      }
      return next
    })

    knownCollapsibleIdsRef.current = currentCollapsibleIds
  }, [children])

  const scrollContainerRef = useRef<HTMLDivElement>(null)

  // Auto-expand ancestors when selection changes (e.g. child selected on canvas)
  useEffect(() => {
    if (selectedIds.length === 0) return
    const ancestorIds = new Set<string>()
    for (const id of selectedIds) {
      let current = getParentOf(id)
      while (current) {
        ancestorIds.add(current.id)
        current = getParentOf(current.id)
      }
    }
    if (ancestorIds.size === 0) return

    setCollapsedIds((prev) => {
      let changed = false
      for (const aid of ancestorIds) {
        if (prev.has(aid)) { changed = true; break }
      }
      if (!changed) return prev
      const next = new Set(prev)
      for (const aid of ancestorIds) next.delete(aid)
      return next
    })

    // Scroll the selected item into view after DOM updates
    requestAnimationFrame(() => {
      const container = scrollContainerRef.current
      if (!container) return
      const el = container.querySelector(`[data-layer-id="${selectedIds[0]}"]`)
      if (el) {
        el.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
      }
    })
  }, [selectedIds, getParentOf])

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
    },
    [setSelection],
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
        case 'boolean-union':
        case 'boolean-subtract':
        case 'boolean-intersect': {
          const opType = action.replace('boolean-', '') as BooleanOpType
          const nodes = selectedIds
            .map((id) => getNodeById(id))
            .filter((n): n is PenNode => n != null)
          if (canBooleanOp(nodes)) {
            const result = executeBooleanOp(nodes, opType)
            if (result) {
              useHistoryStore.getState().pushState(useDocumentStore.getState().document)
              for (const id of selectedIds) removeNode(id)
              useDocumentStore.getState().addNode(null, result)
              setSelection([result.id], result.id)
            }
          }
          break
        }
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
      getNodeById,
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
    <div className="bg-card border-r border-border flex flex-col shrink-0 relative" style={{ width: panelWidth }}>
      {/* Resize handle */}
      <div
        className="absolute right-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-primary/30 active:bg-primary/50 z-10"
        onMouseDown={handleResizeMouseDown}
      />
      <PageTabs />
      <div className="h-8 flex items-center px-3 border-b border-border">
        <span className="text-xs font-medium text-muted-foreground tracking-wider">
          {t('layers.title')}
        </span>
      </div>
      <div ref={scrollContainerRef} className="flex-1 overflow-y-auto py-1 px-1">
        {children.length === 0 ? (
          <p className="text-xs text-muted-foreground text-center mt-4 px-2">
            {t('layers.empty')}
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
        const booleanNodes = selectedIds
          .map((id) => getNodeById(id))
          .filter((n): n is PenNode => n != null)
        return (
          <LayerContextMenu
            x={contextMenu.x}
            y={contextMenu.y}
            nodeId={contextMenu.nodeId}
            canGroup={selectedIds.length >= 2}
            canBoolean={canBooleanOp(booleanNodes)}
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
