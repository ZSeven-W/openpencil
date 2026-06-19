import { describe, it, expect, vi } from 'vitest';

// Mock the virtual wasm module (resolved to test/wasm-stub.ts by vitest.config.ts).
// vi.mock hoists before imports, so the spy is in place when ensureWasm loads.
vi.mock('virtual:op_web_sdk_wasm', () => {
  const init = vi.fn(async () => ({}));
  return { default: init, Viewer: class {}, _export: vi.fn(() => new Uint8Array()) };
});

import { ensureWasm } from '../src/wasm.js';
import initGlue from 'virtual:op_web_sdk_wasm';

describe('ensureWasm', () => {
  it('calls the wasm init exactly once across two calls', async () => {
    await ensureWasm();
    await ensureWasm();
    expect(initGlue).toHaveBeenCalledTimes(1);
  });
});
