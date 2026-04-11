import type { PenDocument, PenNode } from '@/types/pen';

import { useHistoryStore } from '@/stores/history-store';
import { useCanvasStore } from '@/stores/canvas-store';
import {
  findNodeInTree,
  findParentInTree,
  removeNodeFromTree,
  updateNodeInTree,
  insertNodeInTree,
  cloneNodeWithNewIds,
  getActivePageChildren,
  setActivePageChildren,
  getAllChildren,
} from './document-tree-utils';

/** Shortcut: get the active page's children from the current state. */
function _children(s: { document: PenDocument }): PenNode[] {
  return getActivePageChildren(s.document, useCanvasStore.getState().activePageId);
}

/** Shortcut: return a new document with active page's children replaced. */
function _setChildren(doc: PenDocument, children: PenNode[]): PenDocument {
  return setActivePageChildren(doc, useCanvasStore.getState().activePageId, children);
}

interface ComponentActions {
  makeReusable: (nodeId: string) => void;
  detachComponent: (nodeId: string) => string | undefined;
}

type SetState = {
  (partial: Partial<{ document: PenDocument; isDirty: boolean }>): void;
  (
    fn: (state: { document: PenDocument }) => Partial<{ document: PenDocument; isDirty: boolean }>,
  ): void;
};

export function createComponentActions(
  set: SetState,
  get: () => { document: PenDocument },
): ComponentActions {
  return {
    makeReusable: (nodeId) => {
      const state = get();
      const children = _children(state);
      const node = findNodeInTree(children, nodeId);
      if (!node) return;
      // Only container types (frame, group, rectangle) can be made reusable
      if (node.type !== 'frame' && node.type !== 'group' && node.type !== 'rectangle') return;
      if ('reusable' in node && node.reusable) return;
      useHistoryStore.getState().pushState(state.document);
      set((s) => ({
        document: _setChildren(
          s.document,
          updateNodeInTree(_children(s), nodeId, {
            reusable: true,
          } as Partial<PenNode>),
        ),
        isDirty: true,
      }));
    },

    detachComponent: (nodeId) => {
      const state = get();
      const children = _children(state);
      const allNodes = getAllChildren(state.document);
      const node = findNodeInTree(children, nodeId);
      if (!node) return;

      // Case 1: Detach a reusable component (remove reusable flag)
      if ('reusable' in node && node.reusable) {
        useHistoryStore.getState().pushState(state.document);
        set((s) => ({
          document: _setChildren(
            s.document,
            updateNodeInTree(_children(s), nodeId, {
              reusable: undefined,
            } as Partial<PenNode>),
          ),
          isDirty: true,
        }));
        return nodeId;
      }

      // Case 2: Detach an instance (RefNode -> independent node tree)
      if (node.type === 'ref') {
        const component = findNodeInTree(allNodes, node.ref);
        if (!component) return;

        useHistoryStore.getState().pushState(state.document);

        // Apply overrides to a copy of the component before cloning IDs
        const source = structuredClone(component);
        // Apply top-level visual overrides (fill, stroke, etc.)
        const topOverrides = node.descendants?.[node.ref];
        if (topOverrides) {
          Object.assign(source, topOverrides);
        }
        // Apply child-level overrides
        if (node.descendants && 'children' in source && source.children) {
          source.children = source.children.map((child: PenNode) => {
            const override = node.descendants?.[child.id];
            return override ? ({ ...child, ...override } as PenNode) : child;
          });
        }

        // Clone with new IDs
        const detached = cloneNodeWithNewIds(source);
        // Apply all direct instance properties (position, size, meta)
        const detachedRecord = detached as unknown as Record<string, unknown>;
        for (const [key, val] of Object.entries(node)) {
          if (
            key === 'type' ||
            key === 'ref' ||
            key === 'descendants' ||
            key === 'children' ||
            key === 'id'
          )
            continue;
          if (val !== undefined) {
            detachedRecord[key] = val;
          }
        }
        if (!detached.name) detached.name = source.name;
        delete detachedRecord.reusable;

        // Replace the RefNode with the detached tree
        const parent = findParentInTree(children, nodeId);
        const parentId = parent ? parent.id : null;
        const siblings = parent ? ('children' in parent ? (parent.children ?? []) : []) : children;
        const idx = siblings.findIndex((n) => n.id === nodeId);

        let newChildren = removeNodeFromTree(children, nodeId);
        newChildren = insertNodeInTree(newChildren, parentId, detached, idx >= 0 ? idx : undefined);

        set({
          document: _setChildren(state.document, newChildren),
          isDirty: true,
        });
        return detached.id;
      }
    },
  };
}
