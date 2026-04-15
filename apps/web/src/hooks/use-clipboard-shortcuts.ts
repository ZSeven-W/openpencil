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

// Container types (extend ContainerProps in pen-types) — only these can hold children.
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

      // Copy: Cmd/Ctrl+C
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

      // Cut: Cmd/Ctrl+X
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

      // Paste: Cmd/Ctrl+V
      if (isMod && e.key === 'v' && !e.shiftKey) {
        const canvasState = useCanvasStore.getState();
        const { clipboard } = canvasState;
        if (clipboard.length > 0) {
          e.preventDefault();

          // Anchor paste to the active selection:
          //  - If the selected node is a container, paste inside it (as last child).
          //  - Otherwise paste as a sibling, right after the selected node.
          //  - Falls back to root when nothing is selected.
          const anchorId = canvasState.selection.selectedIds[0];
          const docState = useDocumentStore.getState();
          const children = getActivePageChildren(docState.document, canvasState.activePageId);

          let parentId: string | null = null;
          let insertIndex: number | undefined;
          if (anchorId) {
            const anchor = findNodeInTree(children, anchorId);
            if (anchor && canHoldChildren(anchor)) {
              // Paste inside the selected container
              parentId = anchor.id;
              insertIndex = undefined; // append to end
            } else {
              // Paste as sibling of the selected node
              const parent = findParentInTree(children, anchorId);
              parentId = parent ? parent.id : null;
              const siblings = parent && 'children' in parent ? (parent.children ?? []) : children;
              const idx = siblings.findIndex((n) => n.id === anchorId);
              if (idx >= 0) insertIndex = idx + 1;
            }
          }

          const newIds: string[] = [];
          for (const original of clipboard) {
            // Pasting a reusable component creates an instance (RefNode)
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
            // Regular paste for non-reusable nodes
            const [cloned] = cloneNodesWithNewIds([original], { offset: 10 });
            useDocumentStore.getState().addNode(parentId, cloned, insertIndex);
            newIds.push(cloned.id);
            if (insertIndex !== undefined) insertIndex += 1;
          }
          useCanvasStore.getState().setSelection(newIds, newIds[0] ?? null);
        } else {
          // Internal clipboard empty — try reading Figma data from system clipboard.
          // The native `paste` event may not fire when a non-editable element (canvas)
          // has focus, so we also read via the Clipboard API as a fallback.
          e.preventDefault();
          tryPasteFigmaFromClipboard();
        }
        return;
      }

      // Duplicate: Cmd/Ctrl+D
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
