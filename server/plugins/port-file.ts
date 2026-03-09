/**
 * Nitro plugin — writes ~/.openpencil/.port on server startup so the MCP
 * server can discover the running instance (dev server or Electron).
 *
 * In Electron production mode the main process also writes this file,
 * but this plugin ensures the dev server (`bun --bun run dev`) is
 * discoverable too.
 */

import { writeFile, mkdir, unlink } from 'node:fs/promises'
import { join } from 'node:path'
import { homedir } from 'node:os'
const PORT_FILE_DIR = join(homedir(), '.openpencil')
const PORT_FILE_PATH = join(PORT_FILE_DIR, '.port')

async function writePortFile(port: number): Promise<void> {
  try {
    await mkdir(PORT_FILE_DIR, { recursive: true })
    await writeFile(
      PORT_FILE_PATH,
      JSON.stringify({ port, pid: process.pid, timestamp: Date.now() }),
      'utf-8',
    )
  } catch {
    // Non-critical — MCP sync will fall back to file I/O
  }
}

async function cleanupPortFile(): Promise<void> {
  try {
    await unlink(PORT_FILE_PATH)
  } catch {
    // Ignore if already removed
  }
}

export default () => {
  const port = parseInt(process.env.PORT || '3000', 10)
  writePortFile(port)

  const cleanup = () => {
    cleanupPortFile()
  }
  process.on('beforeExit', cleanup)
  process.on('SIGINT', cleanup)
  process.on('SIGTERM', cleanup)
}
