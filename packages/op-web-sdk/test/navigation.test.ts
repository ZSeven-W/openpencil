import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MockViewer } from './mock-wasm.js';
let current: MockViewer;
vi.mock('../src/wasm.js', () => ({
  ensureWasm: vi.fn(async () => {}),
  WasmViewer: vi.fn(() => { current = new MockViewer(); return current; }),
  _export: vi.fn(),
}));
import { createViewer } from '../src/index.js';

describe('navigation', () => {
  beforeEach(() => vi.clearAllMocks());
  it('setZoom keeps pan and changes zoom; viewport reflects it', async () => {
    const v = await createViewer({ canvas: document.createElement('canvas') });
    v.setViewport({ panX: 3, panY: 4, zoom: 1 });
    v.setZoom(2);
    expect(current.set_viewport).toHaveBeenLastCalledWith(3, 4, 2);
    expect(v.viewport).toEqual({ panX: 3, panY: 4, zoom: 2 });
  });
  it('forwards wheel events to the wasm viewer', async () => {
    const canvas = document.createElement('canvas');
    await createViewer({ canvas });
    canvas.dispatchEvent(new WheelEvent('wheel', { deltaX: 5, deltaY: -10, ctrlKey: true, bubbles: true }));
    expect(current.forward_wheel).toHaveBeenCalled();
  });
});
