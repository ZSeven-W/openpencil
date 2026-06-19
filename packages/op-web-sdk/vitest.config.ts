import { defineConfig, type Plugin } from 'vitest/config';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = fileURLToPath(new URL('.', import.meta.url));

// Absolute path to the committed test stub.
const wasmStubPath = path.resolve(__dirname, 'test/wasm-stub.ts');

// The virtual module specifier used in src/wasm.ts and test files.
const VIRTUAL_WASM_ID = 'virtual:op_web_sdk_wasm';

/**
 * Vite plugin that resolves the virtual wasm specifier to the committed test
 * stub during tests, so import-analysis succeeds on a fresh clone with no
 * wasm/ dir present.  The real bundle is resolved differently at build time
 * (by tsup/externals or by sync-wasm.sh populating wasm/).
 */
function wasmStubPlugin(): Plugin {
  return {
    name: 'wasm-stub',
    enforce: 'pre',
    resolveId(id) {
      if (id === VIRTUAL_WASM_ID) return wasmStubPath;
      return null;
    },
  };
}

export default defineConfig({
  plugins: [wasmStubPlugin()],
  test: { environment: 'jsdom', globals: true },
});
