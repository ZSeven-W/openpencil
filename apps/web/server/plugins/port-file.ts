/**
 * Nitro plugin — writes ~/.openpencil/.port on server startup so the MCP
 * server can discover the running instance (dev server or Electron).
 *
 * In Electron production mode the main process also writes this file,
 * but this plugin ensures the dev server (`bun --bun run dev`) is
 * discoverable too.
 */

import { writeFile, mkdir, unlink, readFile } from 'node:fs/promises'
import { randomUUID } from 'node:crypto'
import { join } from 'node:path'
import { homedir } from 'node:os'
const PORT_FILE_DIR = join(homedir(), '.openpencil')
const PORT_FILE_PATH = join(PORT_FILE_DIR, '.port')
const PORT_FILE_TOKEN = randomUUID()

function getOwnerPid(): number {
  return process.ppid > 1 ? process.ppid : process.pid
}

async function writePortFile(port: number): Promise<void> {
  try {
    await mkdir(PORT_FILE_DIR, { recursive: true })
    await writeFile(
      PORT_FILE_PATH,
      JSON.stringify({
        port,
        pid: getOwnerPid(),
        writerPid: process.pid,
        token: PORT_FILE_TOKEN,
        timestamp: Date.now(),
      }),
      'utf-8',
    )
  } catch {
    // Non-critical — MCP sync will fall back to file I/O
  }
}

async function cleanupPortFile(): Promise<void> {
  try {
    const raw = await readFile(PORT_FILE_PATH, 'utf-8')
    const current = JSON.parse(raw) as { token?: string }
    if (current.token !== PORT_FILE_TOKEN) return
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
  process.on('SIGINT', cleanup)
  process.on('SIGTERM', cleanup)
}
