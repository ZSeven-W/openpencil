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

/** Shortcut：从当前状态获取活动页面的子级。 */
function _children(s: { document: PenDocument }): PenNode[] {
  return getActivePageChildren(s.document, useCanvasStore.getState().activePageId);
}

/** Shortcut：返回一个新文档，其中活动页面的子级已替换。 */
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
      // Only 容器类型（框架、组、矩形）可以重复使用
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

      // Case 1：Detach 可重用组件（删除可重用标志）
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

      // Case 2：Detach 实例（RefNode -> 独立节点树）
      if (node.type === 'ref') {
        const component = findNodeInTree(allNodes, node.ref);
        if (!component) return;

        useHistoryStore.getState().pushState(state.document);

        // Apply 在克隆 IDs 之前覆盖组件的副本
        const source = structuredClone(component);
        // Apply 顶级视觉覆盖（填充、描边等）
        const topOverrides = node.descendants?.[node.ref];
        if (topOverrides) {
          Object.assign(source, topOverrides);
        }
        // Apply 子级覆盖
        if (node.descendants && 'children' in source && source.children) {
          source.children = source.children.map((child: PenNode) => {
            const override = node.descendants?.[child.id];
            return override ? ({ ...child, ...override } as PenNode) : child;
          });
        }

        // Clone 与新 IDs
        const detached = cloneNodeWithNewIds(source);
        // Apply 所有直接实例属性（位置、大小、元）
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

        // Replace 具有分离树的 RefNode
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
