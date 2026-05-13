import { beforeEach, describe, expect, it } from 'vitest';

import { useCanvasStore } from '@/stores/canvas-store';

function resetSelection() {
  useCanvasStore.setState({
    selection: {
      ...useCanvasStore.getState().selection,
      selectedIds: [],
      activeId: null,
      hoveredId: null,
    },
  });
}

describe('canvas-store selection', () => {
  beforeEach(() => {
    resetSelection();
  });

  it('does not notify subscribers when setting the same empty selection', () => {
    const previousSelection = useCanvasStore.getState().selection;
    let calls = 0;
    const unsubscribe = useCanvasStore.subscribe(() => {
      calls += 1;
    });

    useCanvasStore.getState().setSelection([], null);

    unsubscribe();
    expect(calls).toBe(0);
    expect(useCanvasStore.getState().selection).toBe(previousSelection);
  });

  it('does not notify subscribers when selecting the same layer again', () => {
    useCanvasStore.getState().setSelection(['node-1'], 'node-1');
    const previousSelection = useCanvasStore.getState().selection;
    let calls = 0;
    const unsubscribe = useCanvasStore.subscribe(() => {
      calls += 1;
    });

    useCanvasStore.getState().setSelection(['node-1'], 'node-1');

    unsubscribe();
    expect(calls).toBe(0);
    expect(useCanvasStore.getState().selection).toBe(previousSelection);
  });

  it('uses the last selected id as active when activeId is omitted', () => {
    useCanvasStore.getState().setSelection(['node-1', 'node-2'], null);

    expect(useCanvasStore.getState().selection.activeId).toBe('node-2');
  });

  it('keeps explicit activeId when it belongs to the selection', () => {
    useCanvasStore.getState().setSelection(['node-1', 'node-2'], 'node-1');

    expect(useCanvasStore.getState().selection.activeId).toBe('node-1');
  });

  it('does not notify subscribers when clearing an already empty selection', () => {
    const previousSelection = useCanvasStore.getState().selection;
    let calls = 0;
    const unsubscribe = useCanvasStore.subscribe(() => {
      calls += 1;
    });

    useCanvasStore.getState().clearSelection();

    unsubscribe();
    expect(calls).toBe(0);
    expect(useCanvasStore.getState().selection).toBe(previousSelection);
  });
});
