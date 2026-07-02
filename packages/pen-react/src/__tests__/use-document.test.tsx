// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import type { ReactNode } from 'react';
import { DesignEngineContext } from '../context';
import { useDocument } from '../hooks/use-document';

function createMockEngine() {
  const listeners = new Map<string, Set<(...args: any[]) => void>>();
  let document = { id: 'doc-1', name: 'Test', children: [], pages: [] };

  return {
    getDocument: vi.fn(() => document),
    on: vi.fn((event: string, cb: (...args: any[]) => void) => {
      if (!listeners.has(event)) listeners.set(event, new Set());
      listeners.get(event)!.add(cb);
      return () => {
        listeners.get(event)?.delete(cb);
      };
    }),
    off: vi.fn(),
    // Test helper: simulate document change
    _setDocument(doc: any) {
      document = doc;
      listeners.get('document:change')?.forEach((cb) => cb(doc));
    },
  };
}

describe('useDocument', () => {
  it('should return current document from engine', () => {
    const engine = createMockEngine();
    const wrapper = ({ children }: { children: ReactNode }) => (
      <DesignEngineContext.Provider value={engine as any}>{children}</DesignEngineContext.Provider>
    );

    const { result } = renderHook(() => useDocument(), { wrapper });
    expect(result.current).toEqual(engine.getDocument());
  });

  it('should re-render when document changes', () => {
    const engine = createMockEngine();
    const wrapper = ({ children }: { children: ReactNode }) => (
      <DesignEngineContext.Provider value={engine as any}>{children}</DesignEngineContext.Provider>
    );

    const { result } = renderHook(() => useDocument(), { wrapper });
    const firstDoc = result.current;

    const newDoc = { id: 'doc-2', name: 'Updated', children: [], pages: [] };
    act(() => {
      engine._setDocument(newDoc);
    });

    expect(result.current).toBe(newDoc);
    expect(result.current).not.toBe(firstDoc);
  });

  it('should subscribe to document:change event', () => {
    const engine = createMockEngine();
    const wrapper = ({ children }: { children: ReactNode }) => (
      <DesignEngineContext.Provider value={engine as any}>{children}</DesignEngineContext.Provider>
    );

    renderHook(() => useDocument(), { wrapper });
    expect(engine.on).toHaveBeenCalledWith('document:change', expect.any(Function));
  });
});
