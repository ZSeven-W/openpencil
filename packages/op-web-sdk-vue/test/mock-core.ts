import { vi } from 'vitest';
// A controllable fake OpViewer for adapter tests. `emit(event)` fires listeners.
export function makeMockViewer() {
  const listeners: Record<string, Set<() => void>> = { load: new Set(), viewportchange: new Set() };
  let vp = { panX: 0, panY: 0, zoom: 1 };
  const viewer = {
    load: vi.fn(),
    get document() { return { id: 'doc', pages: [] }; },
    get pages() { return [{ id: 'p', name: 'P', children: [] }]; },
    get pageCount() { return 1; },
    get activePage() { return 0; },
    get viewport() { return vp; },
    setViewport: vi.fn((v: { panX: number; panY: number; zoom: number }) => { vp = v; }),
    setZoom: vi.fn((z: number) => { vp = { ...vp, zoom: z }; }),
    panTo: vi.fn(), zoomToFit: vi.fn(),
    export: vi.fn(() => new Uint8Array([60, 115, 118, 103, 47, 62])),
    on: vi.fn((e: string, cb: () => void) => { listeners[e].add(cb); return () => listeners[e].delete(cb); }),
    off: vi.fn((e: string, cb: () => void) => { listeners[e].delete(cb); }),
    destroy: vi.fn(),
  };
  const emit = (e: 'load' | 'viewportchange') => listeners[e].forEach((cb) => cb());
  return { viewer, emit };
}
// Shared vi.mock factory: tests call vi.mock('@zseven-w/op-web-sdk', () => coreMock(current))
