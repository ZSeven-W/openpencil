// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { DesignEngineContext } from '../context';
import { PropertyPanel } from '../components/property-panel';

function createMockEngine(selectedNode: any = null, selection: string[] = []) {
  const listeners = new Map<string, Set<(...args: any[]) => void>>();
  // Stable references required so useSyncExternalStore doesn't infinite-loop
  const doc = { id: 'doc', pages: [], children: [] };

  return {
    getDocument: vi.fn(() => doc),
    getSelection: vi.fn(() => selection),
    getNodeById: vi.fn(() => selectedNode),
    getActivePage: vi.fn(() => ''),
    updateNode: vi.fn(),
    canUndo: false,
    canRedo: false,
    undo: vi.fn(),
    redo: vi.fn(),
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

describe('PropertyPanel', () => {
  it('should render nothing when no node is selected', () => {
    const engine = createMockEngine();

    const { container } = render(
      <DesignEngineContext.Provider value={engine as any}>
        <PropertyPanel />
      </DesignEngineContext.Provider>,
    );

    expect(container.innerHTML).toBe('');
  });

  it('should show SizeSection for a rectangle node', () => {
    const rectNode = {
      id: 'rect-1',
      type: 'rectangle',
      name: 'Test Rect',
      x: 10,
      y: 20,
      width: 100,
      height: 50,
    };
    const engine = createMockEngine(rectNode, ['rect-1']);

    render(
      <DesignEngineContext.Provider value={engine as any}>
        <PropertyPanel />
      </DesignEngineContext.Provider>,
    );

    // SizeSection should render position labels
    expect(screen.getByText('Position')).toBeTruthy();
  });

  it('should show TextSection for a text node', () => {
    const textNode = {
      id: 'text-1',
      type: 'text',
      name: 'Test Text',
      x: 0,
      y: 0,
      content: 'Hello',
      fontSize: 16,
      fontFamily: 'Inter',
      fontWeight: 400,
    };
    const engine = createMockEngine(textNode, ['text-1']);

    render(
      <DesignEngineContext.Provider value={engine as any}>
        <PropertyPanel />
      </DesignEngineContext.Provider>,
    );

    // TextSection should render font-related controls
    expect(screen.getByText('Typography')).toBeTruthy();
  });
});
