import { defineConfig } from 'vitest/config';
import { devtools } from '@tanstack/devtools-vite';
import { tanstackStart } from '@tanstack/react-start/plugin/vite';
import viteReact from '@vitejs/plugin-react';
import viteTsConfigPaths from 'vite-tsconfig-paths';
import { fileURLToPath, URL } from 'node:url';
import tailwindcss from '@tailwindcss/vite';
import { nitro } from 'nitro/vite';
import { copyFileSync, mkdirSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { vitePluginSkills } from '../../packages/pen-ai-skills/vite-plugin-skills';

const isElectronBuild = process.env.BUILD_TARGET === 'electron';

// Copy CanvasKit WASM files to public directory for runtime loading
function copyCanvasKitWasm() {
  const wasmDir = resolve('public/canvaskit');
  if (!existsSync(wasmDir)) mkdirSync(wasmDir, { recursive: true });
  const ckDir = resolve('../../node_modules/canvaskit-wasm/bin');
  const files = ['canvaskit.wasm'];
  for (const file of files) {
    const src = resolve(ckDir, file);
    const dest = resolve(wasmDir, file);
    if (existsSync(src) && !existsSync(dest)) {
      copyFileSync(src, dest);
    }
  }
}
copyCanvasKitWasm();

const config = defineConfig({
  test: {
    teardownTimeout: 1000,
    include: [
      'src/**/*.test.{ts,tsx}',
      'server/**/*.test.ts',
      '../../packages/*/src/**/*.test.{ts,tsx}',
      '../desktop/git/__tests__/**/*.test.ts',
    ],
    setupFiles: ['./src/__tests__/setup-react.ts'],
  },
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  ssr: {
    external: ['@zseven-w/agent-native'],
  },
  assetsInclude: ['**/*.wasm'],
  plugins: [
    vitePluginSkills(fileURLToPath(new URL('../../packages/pen-ai-skills', import.meta.url))),
    ...(process.env.NODE_ENV === 'production' ? [] : [devtools()]),
    nitro({
      rollupConfig: {
        external: [
          /^@sentry\//,
          'canvas',
          'jsdom',
          'cssstyle',
          'canvaskit-wasm',
          '@zseven-w/agent-native',
        ],
      },
      serverDir: './server',
      output: { dir: '../../out/web' },
      ...(isElectronBuild ? { preset: 'node-server' } : {}),
    }),
    // this is the plugin that enables path aliases
    viteTsConfigPaths({
      projects: ['./tsconfig.json'],
    }),
    tailwindcss(),
    tanstackStart(),
    viteReact(),
  ],
});

export default config;
