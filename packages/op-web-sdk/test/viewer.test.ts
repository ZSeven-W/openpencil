import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MockViewer } from './mock-wasm.js';
let current: MockViewer;
vi.mock('../src/wasm.js', () => ({
  ensureWasm: vi.fn(async () => {}),
  WasmViewer: vi.fn(() => { current = new MockViewer(); return current; }),
  _export: vi.fn(),
}));
import { createViewer } from '../src/index.js';

function makeCanvas() { return document.createElement('canvas'); }

describe('createViewer + snapshots', () => {
  beforeEach(() => vi.clearAllMocks());
  it('loads the doc and exposes page count', async () => {
    const v = await createViewer({ canvas: makeCanvas(), doc: '{"version":"1.0","pages":[{"id":"p","name":"P","children":[]}]}' });
    expect(current.load_str).toHaveBeenCalled();
    expect(v.pageCount).toBe(1);
    expect(v.activePage).toBe(0);
    expect(Array.isArray(v.pages)).toBe(true);
  });
});
