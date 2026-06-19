import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MockViewer } from './mock-wasm.js';
let current: MockViewer;
vi.mock('../src/wasm.js', () => ({
  ensureWasm: vi.fn(async () => {}), WasmViewer: vi.fn(() => { current = new MockViewer(); return current; }), _export: vi.fn(),
}));
import { createViewer } from '../src/index.js';

describe('events', () => {
  beforeEach(() => vi.clearAllMocks());
  it('fires viewportchange on setZoom and stops after off', async () => {
    const v = await createViewer({ canvas: document.createElement('canvas') });
    const cb = vi.fn();
    const off = v.on('viewportchange', cb);
    v.setZoom(2);
    expect(cb).toHaveBeenCalledTimes(1);
    off(); v.setZoom(3);
    expect(cb).toHaveBeenCalledTimes(1);
  });
});
