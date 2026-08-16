import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MockViewer } from './mock-wasm.js';
let current: MockViewer;
vi.mock('../src/wasm.js', () => ({
  ensureWasm: vi.fn(async () => {}),
  WasmViewer: vi.fn(() => {
    current = new MockViewer();
    return current;
  }),
  _export: vi.fn(),
}));
import { createViewer } from '../src/index.js';
import { normalizeViewerWheelDelta, viewerWheelInput } from '../src/wheel.js';

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
    vi.spyOn(canvas, 'getBoundingClientRect').mockReturnValue({
      x: 10,
      y: 20,
      left: 10,
      top: 20,
      right: 1210,
      bottom: 820,
      width: 1200,
      height: 800,
      toJSON: () => ({}),
    });
    await createViewer({ canvas });
    canvas.dispatchEvent(
      new WheelEvent('wheel', {
        deltaX: 5,
        deltaY: -10,
        ctrlKey: true,
        clientX: 210,
        clientY: 320,
        bubbles: true,
      }),
    );
    expect(current.forward_wheel).toHaveBeenCalledWith(5, -40, true, 200, 300);
  });
  it('normalizes line and page wheel units before forwarding', () => {
    expect(normalizeViewerWheelDelta(3, WheelEvent.DOM_DELTA_LINE, 800)).toBe(120);
    expect(normalizeViewerWheelDelta(1, WheelEvent.DOM_DELTA_PAGE, 800)).toBe(800);
    expect(normalizeViewerWheelDelta(Number.NaN, WheelEvent.DOM_DELTA_PIXEL, 800)).toBe(0);
    expect(normalizeViewerWheelDelta(Number.MAX_VALUE, WheelEvent.DOM_DELTA_LINE, 800)).toBe(0);
  });
  it('accelerates modified zoom without accelerating ordinary pan', () => {
    expect(
      viewerWheelInput(
        {
          deltaX: 2,
          deltaY: 3,
          deltaMode: WheelEvent.DOM_DELTA_LINE,
          ctrlKey: false,
          metaKey: false,
          altKey: false,
        },
        1200,
        800,
      ),
    ).toEqual({ deltaX: 80, deltaY: 120, zoom: false });
    expect(
      viewerWheelInput(
        {
          deltaX: 0,
          deltaY: 5,
          deltaMode: WheelEvent.DOM_DELTA_PIXEL,
          ctrlKey: true,
          metaKey: false,
          altKey: false,
        },
        1200,
        800,
      ),
    ).toEqual({ deltaX: 0, deltaY: 20, zoom: true });
    expect(
      viewerWheelInput(
        {
          deltaX: 0,
          deltaY: 1,
          deltaMode: WheelEvent.DOM_DELTA_PAGE,
          ctrlKey: false,
          metaKey: true,
          altKey: false,
        },
        1200,
        800,
      ),
    ).toEqual({ deltaX: 0, deltaY: 175, zoom: true });
  });
});
