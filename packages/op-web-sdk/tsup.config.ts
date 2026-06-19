import { defineConfig } from 'tsup';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  entry: ['src/index.ts'],
  format: ['esm', 'cjs'],
  dts: true,
  clean: true,
  sourcemap: true,
  // The wasm glue is imported via the virtual specifier `virtual:op_web_sdk_wasm`
  // so vitest (resolveId plugin) and tsc (tsconfig paths) resolve it to a stub
  // without the real bundle present. For the production build, alias it to the
  // real wasm-bindgen glue at `wasm/op_web_sdk.js`, which `sync-wasm.sh`
  // populates before `bun run build`. (Build is deferred-verification: it needs
  // tsup installed and the synced bundle on disk.)
  esbuildOptions(options) {
    options.alias = {
      ...options.alias,
      'virtual:op_web_sdk_wasm': resolve(here, 'wasm/op_web_sdk.js'),
    };
  },
});
