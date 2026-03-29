import { useEffect } from 'react';
import { useCanvasStore } from '@/stores/canvas-store';
import type { ToolType } from '@/types/canvas';

const TOOL_KEYS: Record<string, ToolType> = {
  v: 'select',
  f: 'frame',
  r: 'rectangle',
  o: 'ellipse',
  y: 'polygon',
  l: 'line',
  t: 'text',
  p: 'path',
  h: 'hand',
};

export function useToolShortcuts() {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) {
        return;
      }

      const isMod = e.metaKey || e.ctrlKey;

      // Tool shortcuts (single key, no modifier)
      if (!isMod && !e.shiftKey && !e.altKey) {
        const tool = TOOL_KEYS[e.key.toLowerCase()];
        if (tool) {
          e.preventDefault();
          useCanvasStore.getState().setActiveTool(tool);
          return;
        }
      }

      // Escape: 1) clear selection, 2) exit frame, 3) switch to select tool
      if (e.key === 'Escape') {
        e.preventDefault();
        const { selectedIds, enteredFrameId } = useCanvasStore.getState().selection;

        if (selectedIds.length > 0) {
          useCanvasStore.getState().clearSelection();
        } else if (enteredFrameId) {
          useCanvasStore.getState().exitFrame();
        } else {
          useCanvasStore.getState().setActiveTool('select');
        }
        return;
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, []);
}
