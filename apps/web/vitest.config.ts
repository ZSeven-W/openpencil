import { defineConfig } from 'vitest/config';
import { fileURLToPath, URL } from 'node:url';
import viteTsConfigPaths from 'vite-tsconfig-paths';

export default defineConfig({
  test: {
    teardownTimeout: 1000,
    fileParallelism: false,
    environment: 'node',
    environmentMatchGlobs: [
      ['src/**/*.test.tsx', 'jsdom'],
      ['../../packages/pen-react/src/**/*.test.tsx', 'jsdom'],
    ],
    include: [
      'src/**/*.test.{ts,tsx}',
      'server/**/*.test.ts',
      '../../packages/*/src/**/*.test.{ts,tsx}',
      '../desktop/git/__tests__/**/*.test.ts',
      '../desktop/file-system/__tests__/**/*.test.ts',
      '../desktop/cloud/__tests__/**/*.test.ts',
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
    viteTsConfigPaths({
      projects: ['./tsconfig.json'],
    }),
  ],
});
