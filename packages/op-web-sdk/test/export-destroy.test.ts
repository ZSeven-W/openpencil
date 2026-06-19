import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MockViewer, mockExport } from './mock-wasm.js';
let current: MockViewer;
vi.mock('../src/wasm.js', () => ({
  ensureWasm: vi.fn(async () => {}), WasmViewer: vi.fn(() => { current = new MockViewer(); return current; }), _export: mockExport,
}));
import { createViewer } from '../src/index.js';

describe('export + destroy', () => {
  beforeEach(() => vi.clearAllMocks());
  it('exports svg bytes and rejects png', async () => {
    const v = await createViewer({ canvas: document.createElement('canvas') });
    expect(v.export({ format: 'svg' })).toBeInstanceOf(Uint8Array);
    expect(() => v.export({ format: 'png' as 'svg' })).toThrow();
  });
  it('destroy detaches, frees, and removes the wheel listener', async () => {
    const canvas = document.createElement('canvas');
    const v = await createViewer({ canvas });
    v.destroy();
    expect(current.detach).toHaveBeenCalled();
    expect(current.free).toHaveBeenCalled();
    canvas.dispatchEvent(new WheelEvent('wheel', { bubbles: true }));
    expect(current.forward_wheel).not.toHaveBeenCalled();
  });
  it('calling a method after destroy throws the use-after-free error', async () => {
    const v = await createViewer({ canvas: document.createElement('canvas') });
    v.destroy();
    expect(() => v.setZoom(2)).toThrow('op-web-sdk: viewer has been destroyed');
  });
  it('double-destroy is a no-op and does not call free twice', async () => {
    const v = await createViewer({ canvas: document.createElement('canvas') });
    v.destroy();
    expect(() => v.destroy()).not.toThrow();
    expect(current.free).toHaveBeenCalledTimes(1);
  });
});
