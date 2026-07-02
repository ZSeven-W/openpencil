import { useEffect } from 'react';
import { useCanvasStore } from '@/stores/canvas-store';
import { useDocumentStore } from '@/stores/document-store';
import { useHistoryStore } from '@/stores/history-store';

export function useHistoryShortcuts() {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) {
        return;
      }

      const isMod = e.metaKey || e.ctrlKey;

      // Undo: Cmd/Ctrl+Z
      if (isMod && e.key === 'z' && !e.shiftKey) {
        e.preventDefault();
        const currentDoc = useDocumentStore.getState().document;
        const prev = useHistoryStore.getState().undo(currentDoc);
        if (prev) {
          useDocumentStore.getState().applyHistoryState(prev);
        }
        useCanvasStore.getState().clearSelection();
        return;
      }

      // Redo: Cmd/Ctrl+Shift+Z
      if (isMod && e.key === 'z' && e.shiftKey) {
        e.preventDefault();
        const currentDoc = useDocumentStore.getState().document;
        const next = useHistoryStore.getState().redo(currentDoc);
        if (next) {
          useDocumentStore.getState().applyHistoryState(next);
        }
        useCanvasStore.getState().clearSelection();
        return;
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, []);
}
