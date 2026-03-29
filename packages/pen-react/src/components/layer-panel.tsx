import { useState, useRef, useCallback, useEffect, memo } from 'react';
import type { PenNode } from '@zseven-w/pen-types';
import { findNodeInTree } from '@zseven-w/pen-core';
import { canBooleanOp, executeBooleanOp, type BooleanOpType } from '@zseven-w/pen-core';
import { useDesignEngine } from '../hooks/use-design-engine.js';
import { useDocument } from '../hooks/use-document.js';
import { useSelection } from '../hooks/use-selection.js';
import { useActivePage } from '../hooks/use-active-page.js';
import { LayerItem } from './layer-item.js';
import type { DropPosition } from './layer-item.js';
import { LayerContextMenu } from './layer-context-menu.js';
import { PageTabs } from './page-tabs.js';

const CONTAINER_TYPES = new Set(['frame', 'group', 'ref']);

const LAYER_MIN_WIDTH = 180;
const LAYER_MAX_WIDTH = 480;
const LAYER_DEFAULT_WIDTH = 224;

interface DragState {
  dragId: string | null;
  overId: string | null;
  dropPosition: DropPosition;
}

function isNodeReusable(node: PenNode, parentReusable: boolean): boolean {
  if (parentReusable) return true;
  return 'reusable' in node && (node as unknown as Record<string, unknown>).reusable === true;
}

function getEffectiveChildren(node: PenNode, allChildren: PenNode[]): PenNode[] | null {
  if (node.type === 'ref') {
    const refNode = node as PenNode & { ref: string };
    const component = findNodeInTree(allChildren, refNode.ref);
    if (
      component &&
      'children' in component &&
      (component as PenNode & { children?: PenNode[] }).children?.length
    ) {
      return (component as PenNode & { children: PenNode[] }).children;
    }
    return null;
  }
  return 'children' in node &&
    (node as PenNode & { children?: PenNode[] }).children &&
    (node as PenNode & { children: PenNode[] }).children.length > 0
    ? (node as PenNode & { children: PenNode[] }).children
    : null;
}

function renderLayerTree(
  nodes: PenNode[],
  depth: number,
  selectedIds: string[],
  handlers: {
    onSelect: (id: string) => void;
    onRename: (id: string, name: string) => void;
    onToggleVisibility: (id: string) => void;
    onToggleLock: (id: string) => void;
    onToggleExpand: (id: string) => void;
    onContextMenu: (e: React.MouseEvent, id: string) => void;
    onDragStart: (id: string) => void;
    onDragOver: (id: string, e: React.PointerEvent) => void;
    onDragEnd: () => void;
  },
  dragOverId: string | null,
  dropPosition: DropPosition,
  collapsedIds: Set<string>,
  allChildren: PenNode[],
  parentReusable = false,
  parentIsInstance = false,
): React.ReactNode[] {
  return nodes.map((node) => {
    const nodeChildren = getEffectiveChildren(node, allChildren);
    const isExpanded = !collapsedIds.has(node.id);
    const isDropTarget = dragOverId === node.id;
    const isInstance = node.type === 'ref' || parentIsInstance;
    const reusable = isNodeReusable(node, parentReusable);

    return (
      <div key={node.id}>
        <LayerItem
          id={node.id}
          name={node.name ?? node.type}
          type={node.type}
          depth={depth}
          selected={selectedIds.includes(node.id)}
          visible={node.visible !== false}
          locked={(node as PenNode & { locked?: boolean }).locked === true}
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
    );
  });
}

function collectCollapsibleNodeIds(
  nodes: PenNode[],
  allChildren: PenNode[],
  result: Set<string> = new Set(),
): Set<string> {
  for (const node of nodes) {
    const nodeChildren = getEffectiveChildren(node, allChildren);
    if (!nodeChildren) continue;
    result.add(node.id);
    collectCollapsibleNodeIds(nodeChildren, allChildren, result);
  }
  return result;
}

function LayerPanelInner() {
  const engine = useDesignEngine();
  const doc = useDocument();
  const selectedIds = useSelection();
  const { activePageId } = useActivePage();

  const [panelWidth, setPanelWidth] = useState(LAYER_DEFAULT_WIDTH);
  const isDraggingResize = useRef(false);
  const resizeStartX = useRef(0);
  const resizeStartWidth = useRef(0);

  // Compute active page children
  const children: PenNode[] = (() => {
    if (!doc) return [];
    if (doc.pages && doc.pages.length > 0) {
      const page = doc.pages.find((p) => p.id === activePageId) ?? doc.pages[0];
      return page?.children ?? [];
    }
    return (doc as unknown as { children?: PenNode[] }).children ?? [];
  })();

  const handleResizeMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      isDraggingResize.current = true;
      resizeStartX.current = e.clientX;
      resizeStartWidth.current = panelWidth;

      const handleMouseMove = (ev: MouseEvent) => {
        if (!isDraggingResize.current) return;
        const delta = ev.clientX - resizeStartX.current;
        const newWidth = Math.max(
          LAYER_MIN_WIDTH,
          Math.min(LAYER_MAX_WIDTH, resizeStartWidth.current + delta),
        );
        setPanelWidth(newWidth);
      };

      const handleMouseUp = () => {
        isDraggingResize.current = false;
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleMouseUp);
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
      };

      document.body.style.cursor = 'col-resize';
      document.body.style.userSelect = 'none';
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
    },
    [panelWidth],
  );

  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    nodeId: string;
  } | null>(null);

  const dragRef = useRef<DragState>({
    dragId: null,
    overId: null,
    dropPosition: null,
  });
  const [dragOverId, setDragOverId] = useState<string | null>(null);
  const [dropPosition, setDropPosition] = useState<DropPosition>(null);
  const [collapsedIds, setCollapsedIds] = useState<Set<string>>(() =>
    collectCollapsibleNodeIds(children, children),
  );
  const knownCollapsibleIdsRef = useRef<Set<string>>(new Set());

  useEffect(() => {
    const currentCollapsibleIds = collectCollapsibleNodeIds(children, children);
    const known = knownCollapsibleIdsRef.current;

    setCollapsedIds((prev) => {
      const next = new Set<string>();
      for (const id of currentCollapsibleIds) {
        const isNewNode = !known.has(id);
        if (isNewNode || prev.has(id)) {
          next.add(id);
        }
      }
      return next;
    });

    knownCollapsibleIdsRef.current = currentCollapsibleIds;
  }, [children]);

  const scrollContainerRef = useRef<HTMLDivElement>(null);

  // Auto-expand ancestors when selection changes
  useEffect(() => {
    if (selectedIds.length === 0) return;
    const ancestorIds = new Set<string>();
    const eng = engine as any;
    for (const id of selectedIds) {
      let current = eng.getParentOf?.(id) as PenNode | undefined;
      while (current) {
        ancestorIds.add(current.id);
        current = eng.getParentOf?.(current.id) as PenNode | undefined;
      }
    }
    if (ancestorIds.size === 0) return;

    setCollapsedIds((prev) => {
      let changed = false;
      for (const aid of ancestorIds) {
        if (prev.has(aid)) {
          changed = true;
          break;
        }
      }
      if (!changed) return prev;
      const next = new Set(prev);
      for (const aid of ancestorIds) next.delete(aid);
      return next;
    });

    requestAnimationFrame(() => {
      const container = scrollContainerRef.current;
      if (!container) return;
      const el = container.querySelector(`[data-layer-id="${selectedIds[0]}"]`);
      if (el) {
        el.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
      }
    });
  }, [selectedIds, engine]);

  const handleToggleExpand = useCallback((id: string) => {
    setCollapsedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  }, []);

  const handleSelect = useCallback(
    (id: string) => {
      engine.select([id]);
    },
    [engine],
  );

  const handleRename = useCallback(
    (id: string, name: string) => {
      engine.updateNode(id, { name });
    },
    [engine],
  );

  const handleToggleVisibility = useCallback(
    (id: string) => {
      const node = engine.getNodeById(id);
      if (node) engine.updateNode(id, { visible: node.visible === false });
    },
    [engine],
  );

  const handleToggleLock = useCallback(
    (id: string) => {
      const node = engine.getNodeById(id) as PenNode & { locked?: boolean };
      if (node) engine.updateNode(id, { locked: !node.locked } as Partial<PenNode>);
    },
    [engine],
  );

  const handleContextMenu = useCallback(
    (e: React.MouseEvent, id: string) => {
      e.preventDefault();
      setContextMenu({ x: e.clientX, y: e.clientY, nodeId: id });
      handleSelect(id);
    },
    [handleSelect],
  );

  const handleDragStart = useCallback((id: string) => {
    dragRef.current.dragId = id;
  }, []);

  const handleDragOver = useCallback(
    (id: string, e: React.PointerEvent) => {
      const { dragId } = dragRef.current;
      if (!dragId || dragId === id) return;

      if ((engine as any).isDescendantOf?.(id, dragId)) return;

      const rect = e.currentTarget.getBoundingClientRect();
      const y = e.clientY - rect.top;
      const ratio = y / rect.height;
      const targetNode = engine.getNodeById(id);
      const canBeParent = targetNode ? CONTAINER_TYPES.has(targetNode.type) : false;

      let pos: DropPosition;
      if (canBeParent) {
        if (ratio < 0.25) pos = 'above';
        else if (ratio > 0.75) pos = 'below';
        else pos = 'inside';
      } else {
        pos = ratio < 0.5 ? 'above' : 'below';
      }

      dragRef.current.overId = id;
      dragRef.current.dropPosition = pos;
      setDragOverId(id);
      setDropPosition(pos);
    },
    [engine],
  );

  const handleDragEnd = useCallback(() => {
    const { dragId, overId, dropPosition: pos } = dragRef.current;
    if (dragId && overId && dragId !== overId && pos) {
      const parent = (engine as any).getParentOf?.(overId) as PenNode | undefined;
      const parentId = parent ? parent.id : null;
      const siblings = parent
        ? 'children' in parent
          ? ((parent as PenNode & { children?: PenNode[] }).children ?? [])
          : []
        : children;
      const targetIdx = siblings.findIndex((n) => n.id === overId);

      if (pos === 'inside') {
        engine.moveNode?.(dragId, overId, 0);
        setCollapsedIds((prev) => {
          const next = new Set(prev);
          next.delete(overId);
          return next;
        });
      } else if (targetIdx !== -1) {
        const insertIdx = pos === 'above' ? targetIdx : targetIdx + 1;
        engine.moveNode?.(dragId, parentId, insertIdx);
      }
    }
    dragRef.current = { dragId: null, overId: null, dropPosition: null };
    setDragOverId(null);
    setDropPosition(null);
  }, [children, engine]);

  const handleContextAction = useCallback(
    (action: string) => {
      if (!contextMenu) return;
      const { nodeId } = contextMenu;
      switch (action) {
        case 'delete':
          engine.removeNode(nodeId);
          break;
        case 'duplicate':
          engine.duplicateNode?.(nodeId);
          break;
        case 'group':
          if (selectedIds.length >= 2) {
            const newGroupId = engine.groupNodes?.(selectedIds);
            if (newGroupId) {
              engine.select([newGroupId]);
            }
          }
          break;
        case 'lock':
          handleToggleLock(nodeId);
          break;
        case 'hide':
          handleToggleVisibility(nodeId);
          break;
        case 'make-component':
          (engine as any).makeReusable?.(nodeId);
          break;
        case 'detach-component':
          (engine as any).detachComponent?.(nodeId);
          break;
        case 'boolean-union':
        case 'boolean-subtract':
        case 'boolean-intersect': {
          const opType = action.replace('boolean-', '') as BooleanOpType;
          const nodes = selectedIds
            .map((id) => engine.getNodeById(id))
            .filter((n): n is PenNode => n != null);
          if (canBooleanOp(nodes)) {
            const result = executeBooleanOp(nodes, opType);
            if (result) {
              for (const id of selectedIds) engine.removeNode(id);
              engine.addNode(null, result);
              engine.select([result.id]);
            }
          }
          break;
        }
      }
      setContextMenu(null);
    },
    [contextMenu, selectedIds, engine, handleToggleLock, handleToggleVisibility],
  );

  const handlers = {
    onSelect: handleSelect,
    onRename: handleRename,
    onToggleVisibility: handleToggleVisibility,
    onToggleLock: handleToggleLock,
    onToggleExpand: handleToggleExpand,
    onContextMenu: handleContextMenu,
    onDragStart: handleDragStart,
    onDragOver: handleDragOver,
    onDragEnd: handleDragEnd,
  };

  return (
    <div
      className="bg-card border-r border-border flex flex-col shrink-0 relative"
      style={{ width: panelWidth }}
    >
      {/* Resize handle */}
      <div
        className="absolute right-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-primary/30 active:bg-primary/50 z-10"
        onMouseDown={handleResizeMouseDown}
      />
      <PageTabs />
      <div className="h-8 flex items-center px-3 border-b border-border">
        <span className="text-xs font-medium text-muted-foreground tracking-wider">Layers</span>
      </div>
      <div ref={scrollContainerRef} className="flex-1 overflow-y-auto py-1 px-1">
        {children.length === 0 ? (
          <p className="text-xs text-muted-foreground text-center mt-4 px-2">No layers yet</p>
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

      {contextMenu &&
        (() => {
          const contextNode = engine.getNodeById(contextMenu.nodeId);
          const isContainer = contextNode
            ? contextNode.type === 'frame' ||
              contextNode.type === 'group' ||
              contextNode.type === 'rectangle'
            : false;
          const nodeIsReusable = contextNode
            ? 'reusable' in contextNode &&
              (contextNode as unknown as Record<string, unknown>).reusable === true
            : false;
          const nodeIsInstance = contextNode?.type === 'ref';
          const booleanNodes = selectedIds
            .map((id) => engine.getNodeById(id))
            .filter((n): n is PenNode => n != null);
          return (
            <LayerContextMenu
              x={contextMenu.x}
              y={contextMenu.y}
              nodeId={contextMenu.nodeId}
              canGroup={selectedIds.length >= 2}
              canBoolean={canBooleanOp(booleanNodes)}
              canCreateComponent={isContainer && !nodeIsReusable}
              isReusable={nodeIsReusable}
              isInstance={nodeIsInstance ?? false}
              onAction={handleContextAction}
              onClose={() => setContextMenu(null)}
            />
          );
        })()}
    </div>
  );
}

export const LayerPanel = memo(LayerPanelInner);
