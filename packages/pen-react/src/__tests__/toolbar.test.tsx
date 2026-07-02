// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import { DesignEngineContext } from '../context';
import { CoreToolbar } from '../components/core-toolbar';

afterEach(() => cleanup());

function createMockEngine() {
  const listeners = new Map<string, Set<(...args: any[]) => void>>();
  let tool = 'select';

  return {
    getActiveTool: vi.fn(() => tool),
    setActiveTool: vi.fn((t: string) => {
      tool = t;
      listeners.get('tool:change')?.forEach((cb) => cb(t));
    }),
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

describe('CoreToolbar', () => {
  it('should render core tool buttons', () => {
    const engine = createMockEngine();

    render(
      <DesignEngineContext.Provider value={engine as any}>
        <CoreToolbar />
      </DesignEngineContext.Provider>,
    );

    // Should have select, text, frame, hand tool buttons
    expect(screen.getByLabelText(/select/i)).toBeTruthy();
    expect(screen.getByLabelText(/text/i)).toBeTruthy();
  });

  it('should render trailing slot content', () => {
    const engine = createMockEngine();

    render(
      <DesignEngineContext.Provider value={engine as any}>
        <CoreToolbar trailing={<button data-testid="custom">Custom</button>} />
      </DesignEngineContext.Provider>,
    );

    expect(screen.getByTestId('custom')).toBeTruthy();
  });

  it('should highlight active tool', () => {
    const engine = createMockEngine();

    render(
      <DesignEngineContext.Provider value={engine as any}>
        <CoreToolbar />
      </DesignEngineContext.Provider>,
    );

    const selectBtn = screen.getByLabelText(/select/i);
    expect(selectBtn.getAttribute('aria-pressed')).toBe('true');
  });
});
