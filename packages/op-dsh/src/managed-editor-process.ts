/** Process configuration shared by every DSH-owned editor daemon. */
import { createHash } from 'node:crypto'
import { mkdir } from 'node:fs/promises'
import { join } from 'node:path'
import type { EditorRuntime } from './editor-runtime.js'
import { stateRoot } from './renderer.js'

export async function managedEditorProcessOptions(runtime: EditorRuntime, sourcePath: string, allowOrigin: string): Promise<{ args: string[]; env: NodeJS.ProcessEnv }> {
  // Versioned stores cannot downgrade the standalone application's settings,
  // or each other when two pinned runtimes have different settings schemas.
  const identity = createHash('sha256').update(runtime.openPencilVersion + ':' + runtime.revision).digest('hex').slice(0, 24)
  const configRoot = join(stateRoot(), 'managed-settings', identity)
  await mkdir(configRoot, { recursive: true, mode: 0o700 })
  return {
    args: ['--serve-web', '--managed', '--port', '0', '--file', sourcePath, '--allow-origin', allowOrigin, '--config-root', configRoot],
    env: { ...process.env, OPENPENCIL_WEB_BUNDLE_DIR: runtime.webBundleDir, OPENPENCIL_CANVASKIT_DIR: runtime.canvasKitDir },
  }
}
