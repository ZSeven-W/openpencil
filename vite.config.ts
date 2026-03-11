import { defineConfig } from 'vitest/config'
import { devtools } from '@tanstack/devtools-vite'
import { tanstackStart } from '@tanstack/react-start/plugin/vite'
import viteReact from '@vitejs/plugin-react'
import viteTsConfigPaths from 'vite-tsconfig-paths'
import { fileURLToPath, URL } from 'node:url'
import tailwindcss from '@tailwindcss/vite'
import { nitro } from 'nitro/vite'

const isElectronBuild = process.env.BUILD_TARGET === 'electron'

const config = defineConfig({
  test: {
    teardownTimeout: 1000,
  },
  optimizeDeps: {
    include: ['@cyca/react-timeline-editor'],
  },
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
      '@cyca/react-timeline-editor/css': fileURLToPath(
        new URL('./node_modules/@cyca/react-timeline-editor/dist/react-timeline-editor.css', import.meta.url),
      ),
    },
  },
  plugins: [
    devtools(),
    nitro({
      rollupConfig: { external: [/^@sentry\//, 'canvas', 'jsdom', 'cssstyle'] },
      serverDir: './server',
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
})

export default config
