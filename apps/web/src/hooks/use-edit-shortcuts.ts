import { useEffect } from 'react';
import i18n from '@/i18n';
import { useCanvasStore } from '@/stores/canvas-store';
import { useDocumentStore, getActivePageChildren } from '@/stores/document-store';
import { useHistoryStore } from '@/stores/history-store';
import { canBooleanOp, executeBooleanOp, type BooleanOpType } from '@/utils/boolean-ops';
import { supportsFileSystemAccess, openDocumentFS, openDocument } from '@/utils/file-operations';
import { syncCanvasPositionsToStore, zoomToFitContent } from '@/canvas/skia-engine-ref';

export function useEditShortcuts() {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) {
        return;
      }

      const isMod = e.metaKey || e.ctrlKey;

      // Save: Cmd/Ctrl+S         → save in-place if .op file is known, else save-as
      // Save As: Cmd/Ctrl+Shift+S → always show file picker (never overwrite silently)
      if (isMod && e.key === 's') {
        e.preventDefault();
        try {
          syncCanvasPositionsToStore();
        } catch {
          /* continue */
        }
        useDocumentStore
          .getState()
          .save()
          .catch((err) => console.error('[Save] Failed:', err));
        return;
      }

      // Open: Cmd/Ctrl+O
      if (isMod && e.key === 'o' && !e.shiftKey) {
        e.preventDefault();
        if (useDocumentStore.getState().isDirty) {
          if (!window.confirm(i18n.t('topbar.closeConfirmMessage'))) return;
        }
        if (supportsFileSystemAccess()) {
          openDocumentFS().then((result) => {
            if (result) {
              useDocumentStore.getState().loadDocument(result.doc, result.fileName, result.handle);
              requestAnimationFrame(() => zoomToFitContent());
            }
          });
        } else {
          openDocument().then((result) => {
            if (result) {
              useDocumentStore.getState().loadDocument(result.doc, result.fileName);
              requestAnimationFrame(() => zoomToFitContent());
            }
          });
        }
        return;
      }

      // Group: Cmd/Ctrl+G
      if (isMod && e.key === 'g' && !e.shiftKey) {
        const { selectedIds } = useCanvasStore.getState().selection;
        if (selectedIds.length >= 2) {
          e.preventDefault();
          const groupId = useDocumentStore.getState().groupNodes(selectedIds);
          if (groupId) {
            useCanvasStore.getState().setSelection([groupId], groupId);
          }
        }
        return;
      }

      // Create Component: Cmd/Ctrl+Alt+K
      if (isMod && e.altKey && e.key.toLowerCase() === 'k') {
        const { selectedIds } = useCanvasStore.getState().selection;
        if (selectedIds.length === 1) {
          e.preventDefault();
          useDocumentStore.getState().makeReusable(selectedIds[0]);
        }
        return;
      }

      // Ungroup: Cmd/Ctrl+Shift+G
      if (isMod && e.shiftKey && e.key.toLowerCase() === 'g') {
        const { selectedIds } = useCanvasStore.getState().selection;
        if (selectedIds.length === 1) {
          e.preventDefault();
          const node = useDocumentStore.getState().getNodeById(selectedIds[0]);
          if (node && node.type === 'group' && 'children' in node && node.children) {
            const childIds = node.children.map((c) => c.id);
            useDocumentStore.getState().ungroupNode(selectedIds[0]);
            useCanvasStore.getState().setSelection(childIds, childIds[0] ?? null);
          }
        }
        return;
      }

      // Boolean operations: Cmd/Ctrl+Alt+U (union), Cmd/Ctrl+Alt+S (subtract), Cmd/Ctrl+Alt+I (intersect)
      if (isMod && e.altKey && !e.shiftKey) {
        const booleanOps: Record<string, BooleanOpType> = {
          u: 'union',
          s: 'subtract',
          i: 'intersect',
        };
        const opType = booleanOps[e.key.toLowerCase()];
        if (opType) {
          const { selectedIds } = useCanvasStore.getState().selection;
          const nodes = selectedIds
            .map((id) => useDocumentStore.getState().getNodeById(id))
            .filter((n): n is NonNullable<typeof n> => n != null);
          if (canBooleanOp(nodes)) {
            e.preventDefault();
            const result = executeBooleanOp(nodes, opType);
            if (result) {
              useHistoryStore.getState().pushState(useDocumentStore.getState().document);
              for (const id of selectedIds) {
                useDocumentStore.getState().removeNode(id);
              }
              useDocumentStore.getState().addNode(null, result);
              useCanvasStore.getState().setSelection([result.id], result.id);
            }
          }
          return;
        }
      }

      // Delete / Backspace: remove selected elements
      if (e.key === 'Delete' || e.key === 'Backspace') {
        const { selectedIds } = useCanvasStore.getState().selection;
        if (selectedIds.length > 0) {
          e.preventDefault();
          if (selectedIds.length > 1) {
            useHistoryStore.getState().beginBatch(useDocumentStore.getState().document.children);
          }
          for (const id of selectedIds) {
            useDocumentStore.getState().removeNode(id);
          }
          if (selectedIds.length > 1) {
            useHistoryStore.getState().endBatch(useDocumentStore.getState().document);
          }
          useCanvasStore.getState().clearSelection();
        }
        return;
      }

      // Cmd+A: select all (top-level nodes only, matching manual selection behavior)
      if (isMod && e.key === 'a') {
        e.preventDefault();
        const topLevelNodes = getActivePageChildren(
          useDocumentStore.getState().document,
          useCanvasStore.getState().activePageId,
        );
        const ids = topLevelNodes.map((n) => n.id);
        useCanvasStore.getState().setSelection(ids, ids[0] ?? null);
        return;
      }

      // [ ] : reorder layers
      if (e.key === '[') {
        e.preventDefault();
        const { selectedIds } = useCanvasStore.getState().selection;
        if (selectedIds.length > 1) {
          useHistoryStore.getState().beginBatch(useDocumentStore.getState().document.children);
        }
        for (const id of selectedIds) {
          useDocumentStore.getState().reorderNode(id, 'down');
        }
        if (selectedIds.length > 1) {
          useHistoryStore.getState().endBatch(useDocumentStore.getState().document);
        }
        return;
      }
      if (e.key === ']') {
        e.preventDefault();
        const { selectedIds } = useCanvasStore.getState().selection;
        if (selectedIds.length > 1) {
          useHistoryStore.getState().beginBatch(useDocumentStore.getState().document.children);
        }
        for (const id of selectedIds) {
          useDocumentStore.getState().reorderNode(id, 'up');
        }
        if (selectedIds.length > 1) {
          useHistoryStore.getState().endBatch(useDocumentStore.getState().document);
        }
        return;
      }

      // Arrow keys: nudge
      const nudgeKeys = ['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'];
      if (nudgeKeys.includes(e.key) && !isMod) {
        const { selectedIds } = useCanvasStore.getState().selection;
        if (selectedIds.length === 0) return;
        e.preventDefault();
        if (selectedIds.length > 1) {
          useHistoryStore.getState().beginBatch(useDocumentStore.getState().document.children);
        }
        const amount = e.shiftKey ? 10 : 1;
        for (const id of selectedIds) {
          const node = useDocumentStore.getState().getNodeById(id);
          if (!node) continue;
          const updates: Record<string, number> = {};
          if (e.key === 'ArrowLeft') updates.x = (node.x ?? 0) - amount;
          if (e.key === 'ArrowRight') updates.x = (node.x ?? 0) + amount;
          if (e.key === 'ArrowUp') updates.y = (node.y ?? 0) - amount;
          if (e.key === 'ArrowDown') updates.y = (node.y ?? 0) + amount;
          useDocumentStore.getState().updateNode(id, updates);
        }
        if (selectedIds.length > 1) {
          useHistoryStore.getState().endBatch(useDocumentStore.getState().document);
        }
        return;
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, []);
}
