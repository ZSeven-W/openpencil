import { useEffect } from 'react';
import { useCanvasStore } from '@/stores/canvas-store';
import { useDocumentStore } from '@/stores/document-store';
import { cloneNodesWithNewIds } from '@/utils/node-clone';
import { tryPasteFigmaFromClipboard } from '@/hooks/use-figma-paste';
import {
  findNodeInTree,
  findParentInTree,
  getActivePageChildren,
} from '@/stores/document-tree-utils';
import type { PenNode } from '@zseven-w/pen-types';

// Container 类型（在钢笔类型中扩展 ContainerProps）——只有这些可以容纳孩子。
function canHoldChildren(node: PenNode): boolean {
  return node.type === 'frame' || node.type === 'group' || node.type === 'rectangle';
}

export function useClipboardShortcuts() {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) {
        return;
      }

      const isMod = e.metaKey || e.ctrlKey;

      // Copy: Cmd/Ctrl+c
      if (isMod && e.key === 'c' && !e.shiftKey) {
        const { selectedIds } = useCanvasStore.getState().selection;
        if (selectedIds.length > 0) {
          e.preventDefault();
          const nodes = selectedIds
            .map((id) => useDocumentStore.getState().getNodeById(id))
            .filter((n): n is NonNullable<typeof n> => n != null);
          useCanvasStore.getState().setClipboard(structuredClone(nodes));
        }
        return;
      }

      // Cut: Cmd/Ctrl+x
      if (isMod && e.key === 'x' && !e.shiftKey) {
        const { selectedIds } = useCanvasStore.getState().selection;
        if (selectedIds.length > 0) {
          e.preventDefault();
          const nodes = selectedIds
            .map((id) => useDocumentStore.getState().getNodeById(id))
            .filter((n): n is NonNullable<typeof n> => n != null);
          useCanvasStore.getState().setClipboard(structuredClone(nodes));
          for (const id of selectedIds) {
            useDocumentStore.getState().removeNode(id);
          }
          useCanvasStore.getState().clearSelection();
        }
        return;
      }

      // Paste: Cmd/Ctrl+v
      if (isMod && e.key === 'v' && !e.shiftKey) {
        const canvasState = useCanvasStore.getState();
        const { clipboard } = canvasState;
        if (clipboard.length > 0) {
          e.preventDefault();

          // Anchor 粘贴到活动选择： - If 所选节点是一个容器，粘贴到其中（
          // 作为最后一个子节点）。 - Otherwise 粘贴为同级节点，位于所选节点之后。 - Falls 当未选择任何内容时返回根目录。
          const anchorId = canvasState.selection.selectedIds[0];
          const docState = useDocumentStore.getState();
          const children = getActivePageChildren(docState.document, canvasState.activePageId);

          let parentId: string | null = null;
          let insertIndex: number | undefined;
          if (anchorId) {
            const anchor = findNodeInTree(children, anchorId);
            if (anchor && canHoldChildren(anchor)) {
              // Paste 在所选容器内
              parentId = anchor.id;
              insertIndex = undefined; // 追加到末尾
            } else {
              // Paste 作为所选节点的兄弟节点
              const parent = findParentInTree(children, anchorId);
              parentId = parent ? parent.id : null;
              const siblings = parent && 'children' in parent ? (parent.children ?? []) : children;
              const idx = siblings.findIndex((n) => n.id === anchorId);
              if (idx >= 0) insertIndex = idx + 1;
            }
          }

          const newIds: string[] = [];
          for (const original of clipboard) {
            // Pasting 可重用组件创建实例 (RefNode)
            if ('reusable' in original && original.reusable) {
              const component = useDocumentStore.getState().getNodeById(original.id);
              if (component && 'reusable' in component && component.reusable) {
                const newId = useDocumentStore.getState().duplicateNode(original.id);
                if (newId) {
                  newIds.push(newId);
                  continue;
                }
              }
            }
            // Regular 粘贴不可重复使用的节点
            const [cloned] = cloneNodesWithNewIds([original], { offset: 10 });
            useDocumentStore.getState().addNode(parentId, cloned, insertIndex);
            newIds.push(cloned.id);
            if (insertIndex !== undefined) insertIndex += 1;
          }
          useCanvasStore.getState().setSelection(newIds, newIds[0] ?? null);
        } else {
          // Internal 剪贴板为空 — 尝试从系统剪贴板读取 Figma 数据。当不可编辑元素（画布）具有焦点时，The 本机 `paste`
          // 事件可能不会触发，因此我们还通过 Clipboard API 读取作为后备。
          e.preventDefault();
          tryPasteFigmaFromClipboard();
        }
        return;
      }

      // Duplicate: Cmd/Ctrl+d
      if (isMod && e.key === 'd') {
        const { selectedIds } = useCanvasStore.getState().selection;
        if (selectedIds.length > 0) {
          e.preventDefault();
          const newIds: string[] = [];
          for (const id of selectedIds) {
            const newId = useDocumentStore.getState().duplicateNode(id);
            if (newId) newIds.push(newId);
          }
          if (newIds.length > 0) {
            useCanvasStore.getState().setSelection(newIds, newIds[0]);
          }
        }
        return;
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, []);
}
