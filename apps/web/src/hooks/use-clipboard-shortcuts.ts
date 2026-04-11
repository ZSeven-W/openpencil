import { useEffect } from 'react';
import { useCanvasStore } from '@/stores/canvas-store';
import { useDocumentStore } from '@/stores/document-store';
import { cloneNodesWithNewIds } from '@/utils/node-clone';
import { tryPasteFigmaFromClipboard } from '@/hooks/use-figma-paste';

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
        const { clipboard } = useCanvasStore.getState();
        if (clipboard.length > 0) {
          e.preventDefault();
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
            useDocumentStore.getState().addNode(null, cloned);
            newIds.push(cloned.id);
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
