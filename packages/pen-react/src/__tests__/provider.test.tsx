// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { render, renderHook } from '@testing-library/react';
import type { ReactNode } from 'react';
import { DesignProvider } from '../provider';
import { useDesignEngine } from '../hooks/use-design-engine';

// Mock DesignEngine
vi.mock('@zseven-w/pen-engine', () => {
  const listeners = new Map<string, Set<(...args: any[]) => void>>();
  let doc = { id: 'default', name: 'Untitled', children: [], pages: [] };

  class MockDesignEngine {
    on(event: string, cb: (...args: any[]) => void) {
      if (!listeners.has(event)) listeners.set(event, new Set());
      listeners.get(event)!.add(cb);
      return () => {
        listeners.get(event)?.delete(cb);
      };
    }
    off(event: string, cb: (...args: any[]) => void) {
      listeners.get(event)?.delete(cb);
    }
    loadDocument(d: any) {
      doc = d;
      listeners.get('document:change')?.forEach((fn) => fn(d));
    }
    getDocument() {
      return doc;
    }
    createDocument() {
      return { id: 'new', name: 'New', children: [], pages: [] };
    }
    dispose() {}
  }

  return { DesignEngine: MockDesignEngine };
});

describe('DesignProvider', () => {
  it('should provide engine to children', () => {
    const wrapper = ({ children }: { children: ReactNode }) => (
      <DesignProvider>{children}</DesignProvider>
    );

    const { result } = renderHook(() => useDesignEngine(), { wrapper });
    expect(result.current).toBeDefined();
    expect(typeof result.current.getDocument).toBe('function');
  });

  it('should create engine once and reuse across re-renders', () => {
    let engineRef1: any;
    let engineRef2: any;

    function Capture({ onCapture }: { onCapture: (e: any) => void }) {
      const engine = useDesignEngine();
      onCapture(engine);
      return null;
    }

    const { rerender } = render(
      <DesignProvider>
        <Capture
          onCapture={(e) => {
            engineRef1 = e;
          }}
        />
      </DesignProvider>,
    );

    rerender(
      <DesignProvider>
        <Capture
          onCapture={(e) => {
            engineRef2 = e;
          }}
        />
      </DesignProvider>,
    );

    expect(engineRef1).toBe(engineRef2);
  });

  it('should load initialDocument on first render', () => {
    const initDoc = { id: 'init', name: 'Init', children: [], pages: [] };
    let capturedEngine: any;

    function Capture() {
      capturedEngine = useDesignEngine();
      return null;
    }

    render(
      <DesignProvider initialDocument={initDoc as any}>
        <Capture />
      </DesignProvider>,
    );

    expect(capturedEngine.getDocument()).toMatchObject({ id: 'init', name: 'Init' });
  });
});
