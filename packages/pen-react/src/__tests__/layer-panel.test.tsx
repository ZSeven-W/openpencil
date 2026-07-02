// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { DesignEngineContext } from '../context';
import { LayerPanel } from '../components/layer-panel';

function createMockEngine() {
  const listeners = new Map<string, Set<(...args: any[]) => void>>();

  // Stable references required so useSyncExternalStore doesn't infinite-loop
  const doc = {
    id: 'doc',
    pages: [
      {
        id: 'page-1',
        name: 'Page 1',
        children: [
          {
            id: 'rect-1',
            type: 'rectangle',
            name: 'Header BG',
            x: 0,
            y: 0,
            width: 400,
            height: 60,
          },
          { id: 'text-1', type: 'text', name: 'Title', x: 20, y: 10, content: 'Hello' },
        ],
      },
    ],
    children: [],
  };
  const selection: string[] = [];

  return {
    getDocument: vi.fn(() => doc),
    getSelection: vi.fn(() => selection),
    getActivePage: vi.fn(() => 'page-1'),
    select: vi.fn(),
    canUndo: false,
    canRedo: false,
    undo: vi.fn(),
    redo: vi.fn(),
    getNodeById: vi.fn((id: string) => {
      if (id === 'rect-1')
        return {
          id: 'rect-1',
          type: 'rectangle',
          name: 'Header BG',
          x: 0,
          y: 0,
          width: 400,
          height: 60,
        };
      if (id === 'text-1')
        return { id: 'text-1', type: 'text', name: 'Title', x: 20, y: 10, content: 'Hello' };
      return undefined;
    }),
    on: vi.fn((event: string, cb: (...args: any[]) => void) => {
      if (!listeners.has(event)) listeners.set(event, new Set());
      listeners.get(event)!.add(cb);
      return () => {
        listeners.get(event)?.delete(cb);
      };
    }),
    off: vi.fn(),
  };
}

describe('LayerPanel', () => {
  it('should render layer items for active page children', () => {
    const engine = createMockEngine();

    render(
      <DesignEngineContext.Provider value={engine as any}>
        <LayerPanel />
      </DesignEngineContext.Provider>,
    );

    expect(screen.getByText('Header BG')).toBeTruthy();
    expect(screen.getByText('Title')).toBeTruthy();
  });
});
