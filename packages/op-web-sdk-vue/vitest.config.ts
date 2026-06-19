import { defineConfig } from 'vitest/config';
import vue from '@vitejs/plugin-vue';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
const here = fileURLToPath(new URL('.', import.meta.url));
export default defineConfig({
  plugins: [vue()],
  test: { environment: 'jsdom', globals: true },
  resolve: { alias: {
    '@zseven-w/op-web-sdk': resolve(here, '../op-web-sdk/src/index.ts'),
    'virtual:op_web_sdk_wasm': resolve(here, '../op-web-sdk/test/wasm-stub.ts'),
  } },
});
