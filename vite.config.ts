import { defineConfig } from 'vitest/config'
import { devtools } from '@tanstack/devtools-vite'
import { tanstackStart } from '@tanstack/react-start/plugin/vite'
import viteReact from '@vitejs/plugin-react'
import viteTsConfigPaths from 'vite-tsconfig-paths'
import { fileURLToPath, URL } from 'node:url'
import tailwindcss from '@tailwindcss/vite'
import { nitro } from 'nitro/vite'
import type { Plugin } from 'vite'

/**
 * Vite plugin to fix @cyca/react-timeline-editor CJS interop.
 * The library inlines react-compiler-runtime and react/jsx-runtime as CJS modules
 * that call __require("react"), which fails in Vite's ESM dev mode.
 * This plugin rewrites those calls to use the already-imported React.
 */
function timelineEditorCjsFix(): Plugin {
  return {
    name: 'timeline-editor-cjs-fix',
    enforce: 'pre',
    transform(code, id) {
      if (!id.includes('react-timeline-editor')) return
      if (!code.includes('__require("react")')) return
      // The library already does `import * as React$1 from "react"` at top level
      return code.replace(/__require\("react"\)/g, 'React$1')
    },
  }
}

const isElectronBuild = process.env.BUILD_TARGET === 'electron'

const config = defineConfig({
  test: {
    teardownTimeout: 1000,
  },
  optimizeDeps: {
    exclude: ['@cyca/react-timeline-editor'],
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
    timelineEditorCjsFix(),
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
