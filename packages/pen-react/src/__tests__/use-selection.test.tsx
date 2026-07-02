// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import type { ReactNode } from 'react';
import { DesignEngineContext } from '../context';
import { useSelection } from '../hooks/use-selection';

function createMockEngine() {
  const listeners = new Map<string, Set<(...args: any[]) => void>>();
  let selection: string[] = [];

  return {
    getSelection: vi.fn(() => selection),
    on: vi.fn((event: string, cb: (...args: any[]) => void) => {
      if (!listeners.has(event)) listeners.set(event, new Set());
      listeners.get(event)!.add(cb);
      return () => {
        listeners.get(event)?.delete(cb);
      };
    }),
    off: vi.fn(),
    _setSelection(ids: string[]) {
      selection = ids;
      listeners.get('selection:change')?.forEach((cb) => cb(ids));
    },
  };
}

describe('useSelection', () => {
  it('should return current selection from engine', () => {
    const engine = createMockEngine();
    const wrapper = ({ children }: { children: ReactNode }) => (
      <DesignEngineContext.Provider value={engine as any}>{children}</DesignEngineContext.Provider>
    );

    const { result } = renderHook(() => useSelection(), { wrapper });
    expect(result.current).toEqual([]);
  });

  it('should re-render when selection changes', () => {
    const engine = createMockEngine();
    const wrapper = ({ children }: { children: ReactNode }) => (
      <DesignEngineContext.Provider value={engine as any}>{children}</DesignEngineContext.Provider>
    );

    const { result } = renderHook(() => useSelection(), { wrapper });
    expect(result.current).toEqual([]);

    act(() => {
      engine._setSelection(['node-1', 'node-2']);
    });

    expect(result.current).toEqual(['node-1', 'node-2']);
  });
});
