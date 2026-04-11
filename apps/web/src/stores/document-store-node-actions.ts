import { nanoid } from 'nanoid';
import type { PenDocument, PenNode, GroupNode, RefNode } from '@/types/pen';

import { useHistoryStore } from '@/stores/history-store';
import { useCanvasStore } from '@/stores/canvas-store';
import {
  findNodeInTree,
  findParentInTree,
  removeNodeFromTree,
  updateNodeInTree,
  flattenNodes,
  insertNodeInTree,
  isDescendantOf,
  getNodeBounds,
  findClearX,
  scaleChildrenInPlace,
  rotateChildrenInPlace,
  cloneNodeWithNewIds,
  deepCloneNode,
  getActivePageChildren,
  setActivePageChildren,
  getAllChildren,
} from './document-tree-utils';

type SetState = {
  (partial: Partial<{ document: PenDocument; isDirty: boolean }>): void;
  (
    fn: (state: { document: PenDocument }) => Partial<{ document: PenDocument; isDirty: boolean }>,
  ): void;
};

/** Shortcut: get the active page's children from the current state. */
function _children(s: { document: PenDocument }): PenNode[] {
  return getActivePageChildren(s.document, useCanvasStore.getState().activePageId);
}

/** Shortcut: return a new document with active page's children replaced. */
function _setChildren(doc: PenDocument, children: PenNode[]): PenDocument {
  return setActivePageChildren(doc, useCanvasStore.getState().activePageId, children);
}

/** Push current document to history, then apply mutation and mark dirty. */
function mutateWithHistory(
  get: () => { document: PenDocument },
  set: SetState,
  fn: (doc: PenDocument) => PenDocument,
) {
  useHistoryStore.getState().pushState(get().document);
  set({ document: fn(get().document), isDirty: true });
}

interface NodeActions {
  addNode: (parentId: string | null, node: PenNode, index?: number) => void;
  updateNode: (id: string, updates: Partial<PenNode>) => void;
  removeNode: (id: string) => void;
  moveNode: (id: string, newParentId: string | null, index: number) => void;
  reorderNode: (id: string, direction: 'up' | 'down') => void;
  toggleVisibility: (id: string) => void;
  toggleLock: (id: string) => void;
  duplicateNode: (id: string) => string | null;
  groupNodes: (nodeIds: string[]) => string | null;
  ungroupNode: (groupId: string) => void;
  scaleDescendantsInStore: (parentId: string, scaleX: number, scaleY: number) => void;
  rotateDescendantsInStore: (parentId: string, angleDeltaDeg: number) => void;
  getNodeById: (id: string) => PenNode | undefined;
  getParentOf: (id: string) => PenNode | undefined;
  getFlatNodes: () => PenNode[];
  isDescendantOf: (nodeId: string, ancestorId: string) => boolean;
}

export function createNodeActions(
  set: SetState,
  get: () => { document: PenDocument },
): NodeActions {
  return {
    addNode: (parentId, node, index) => {
      mutateWithHistory(get, set, (doc) =>
        _setChildren(
          doc,
          // Default to index 0 (prepend) so new items appear at the top of
          // the layer panel = frontmost on canvas. Callers can pass an
          // explicit index to override.
          insertNodeInTree(_children({ document: doc }), parentId, node, index ?? 0),
        ),
      );
    },

    updateNode: (id, updates) => {
      mutateWithHistory(get, set, (doc) =>
        _setChildren(doc, updateNodeInTree(_children({ document: doc }), id, updates)),
      );
    },

    removeNode: (id) => {
      mutateWithHistory(get, set, (doc) =>
        _setChildren(doc, removeNodeFromTree(_children({ document: doc }), id)),
      );
    },

    moveNode: (id, newParentId, index) => {
      const state = get();
      const children = _children(state);
      const node = findNodeInTree(children, id);
      if (!node) return;
      const withoutNode = removeNodeFromTree(children, id);
      const withNode = insertNodeInTree(withoutNode, newParentId, deepCloneNode(node), index);
      mutateWithHistory(get, set, () => _setChildren(state.document, withNode));
    },

    reorderNode: (id, direction) => {
      const state = get();
      const children = _children(state);
      const parent = findParentInTree(children, id);
      const siblings = parent ? ('children' in parent ? (parent.children ?? []) : []) : children;
      const idx = siblings.findIndex((n) => n.id === id);
      if (idx === -1) return;
      const newIdx =
        direction === 'up' ? Math.max(0, idx - 1) : Math.min(siblings.length - 1, idx + 1);
      if (newIdx === idx) return;
      const newSiblings = [...siblings];
      const [removed] = newSiblings.splice(idx, 1);
      newSiblings.splice(newIdx, 0, removed);

      if (parent && 'children' in parent) {
        mutateWithHistory(get, set, (doc) =>
          _setChildren(
            doc,
            updateNodeInTree(_children({ document: doc }), parent.id, {
              children: newSiblings,
            } as Partial<PenNode>),
          ),
        );
      } else {
        mutateWithHistory(get, set, (doc) => _setChildren(doc, newSiblings));
      }
    },

    toggleVisibility: (id) => {
      const node = findNodeInTree(_children(get()), id);
      if (!node) return;
      const currentVisible = node.visible !== false;
      mutateWithHistory(get, set, (doc) =>
        _setChildren(
          doc,
          updateNodeInTree(_children({ document: doc }), id, {
            visible: !currentVisible,
          } as Partial<PenNode>),
        ),
      );
    },

    toggleLock: (id) => {
      const node = findNodeInTree(_children(get()), id);
      if (!node) return;
      const currentLocked = node.locked === true;
      mutateWithHistory(get, set, (doc) =>
        _setChildren(
          doc,
          updateNodeInTree(_children({ document: doc }), id, {
            locked: !currentLocked,
          } as Partial<PenNode>),
        ),
      );
    },

    duplicateNode: (id) => {
      const state = get();
      const children = _children(state);
      const allNodes = getAllChildren(state.document);
      const node = findNodeInTree(children, id);
      if (!node) return null;

      // Duplicating a reusable component creates an instance (RefNode)
      if ('reusable' in node && node.reusable === true) {
        const bounds = getNodeBounds(node, allNodes);
        const parent = findParentInTree(children, id);
        const parentId = parent ? parent.id : null;
        const siblings = parent ? ('children' in parent ? (parent.children ?? []) : []) : children;
        const idx = siblings.findIndex((n) => n.id === id);

        const clearX = findClearX(bounds.x, bounds.w, bounds.y, bounds.h, siblings, id, allNodes);

        const refNode: RefNode = {
          id: nanoid(),
          type: 'ref',
          ref: node.id,
          name: node.name ?? node.type,
          x: clearX,
          y: bounds.y,
        };

        mutateWithHistory(get, set, (doc) =>
          _setChildren(
            doc,
            insertNodeInTree(_children({ document: doc }), parentId, refNode as PenNode, idx),
          ),
        );
        return refNode.id;
      }

      // Regular duplication for non-reusable nodes
      const clone = cloneNodeWithNewIds(node);
      clone.name = (clone.name ?? clone.type) + ' copy';

      const parent = findParentInTree(children, id);
      const parentId = parent ? parent.id : null;
      const siblings = parent ? ('children' in parent ? (parent.children ?? []) : []) : children;
      const idx = siblings.findIndex((n) => n.id === id);

      const bounds = getNodeBounds(node, allNodes);
      clone.x = findClearX(bounds.x, bounds.w, bounds.y, bounds.h, siblings, id, allNodes);
      clone.y = bounds.y;

      mutateWithHistory(get, set, (doc) =>
        _setChildren(doc, insertNodeInTree(_children({ document: doc }), parentId, clone, idx)),
      );
      return clone.id;
    },

    groupNodes: (nodeIds) => {
      if (nodeIds.length < 2) return null;
      const state = get();
      const children = _children(state);
      const nodes = nodeIds.map((id) => findNodeInTree(children, id)).filter(Boolean) as PenNode[];
      if (nodes.length < 2) return null;

      // Compute bounding box
      let minX = Infinity,
        minY = Infinity,
        maxX = -Infinity,
        maxY = -Infinity;
      for (const n of nodes) {
        const nx = n.x ?? 0;
        const ny = n.y ?? 0;
        const nw = 'width' in n && typeof n.width === 'number' ? n.width : 0;
        const nh = 'height' in n && typeof n.height === 'number' ? n.height : 0;
        minX = Math.min(minX, nx);
        minY = Math.min(minY, ny);
        maxX = Math.max(maxX, nx + nw);
        maxY = Math.max(maxY, ny + nh);
      }

      // Make children relative to group
      const groupChildren = nodes.map((n) => ({
        ...n,
        x: (n.x ?? 0) - minX,
        y: (n.y ?? 0) - minY,
      })) as PenNode[];

      const groupId = nanoid();
      const group: GroupNode = {
        id: groupId,
        type: 'group',
        name: 'Group',
        x: minX,
        y: minY,
        width: maxX - minX,
        height: maxY - minY,
        children: groupChildren,
      };

      // Find insertion position (position of first selected node)
      const firstParent = findParentInTree(children, nodeIds[0]);
      const parentId = firstParent ? firstParent.id : null;
      const siblings = firstParent
        ? 'children' in firstParent
          ? (firstParent.children ?? [])
          : []
        : children;
      const firstIdx = siblings.findIndex((n) => nodeIds.includes(n.id));

      // Remove all selected nodes, then insert group at first node's position
      let newChildren = children;
      for (const id of nodeIds) {
        newChildren = removeNodeFromTree(newChildren, id);
      }
      newChildren = insertNodeInTree(newChildren, parentId, group, firstIdx);

      mutateWithHistory(get, set, () => _setChildren(state.document, newChildren));
      return groupId;
    },

    ungroupNode: (groupId) => {
      const state = get();
      const children = _children(state);
      const group = findNodeInTree(children, groupId);
      if (!group || group.type !== 'group') return;
      if (!('children' in group) || !group.children) return;

      const parent = findParentInTree(children, groupId);
      const parentId = parent ? parent.id : null;
      const siblings = parent ? ('children' in parent ? (parent.children ?? []) : []) : children;
      const groupIdx = siblings.findIndex((n) => n.id === groupId);

      // Adjust children coordinates to parent space
      const groupX = group.x ?? 0;
      const groupY = group.y ?? 0;
      const adjustedChildren = group.children.map((child) => ({
        ...child,
        x: (child.x ?? 0) + groupX,
        y: (child.y ?? 0) + groupY,
      })) as PenNode[];

      // Remove group, then insert children at group's position (in reverse to maintain order)
      let newChildren = removeNodeFromTree(children, groupId);
      for (let i = adjustedChildren.length - 1; i >= 0; i--) {
        newChildren = insertNodeInTree(newChildren, parentId, adjustedChildren[i], groupIdx);
      }

      mutateWithHistory(get, set, () => _setChildren(state.document, newChildren));
    },

    scaleDescendantsInStore: (parentId, scaleX, scaleY) => {
      if (scaleX === 1 && scaleY === 1) return;
      const state = get();
      const children = _children(state);
      const parent = findNodeInTree(children, parentId);
      if (!parent || !('children' in parent) || !parent.children) return;

      const scaledChildren = scaleChildrenInPlace(parent.children, scaleX, scaleY);
      mutateWithHistory(get, set, (doc) =>
        _setChildren(
          doc,
          updateNodeInTree(_children({ document: doc }), parentId, {
            children: scaledChildren,
          } as Partial<PenNode>),
        ),
      );
    },

    rotateDescendantsInStore: (parentId, angleDeltaDeg) => {
      if (angleDeltaDeg === 0) return;
      const state = get();
      const children = _children(state);
      const parent = findNodeInTree(children, parentId);
      if (!parent || !('children' in parent) || !parent.children) return;

      const rotatedChildren = rotateChildrenInPlace(parent.children, angleDeltaDeg);
      mutateWithHistory(get, set, (doc) =>
        _setChildren(
          doc,
          updateNodeInTree(_children({ document: doc }), parentId, {
            children: rotatedChildren,
          } as Partial<PenNode>),
        ),
      );
    },

    getNodeById: (id) => findNodeInTree(_children(get()), id),

    getParentOf: (id) => findParentInTree(_children(get()), id),

    getFlatNodes: () => flattenNodes(_children(get())),

    isDescendantOf: (nodeId, ancestorId) => isDescendantOf(_children(get()), nodeId, ancestorId),
  };
}
