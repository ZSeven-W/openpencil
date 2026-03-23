/** Port file discovery and app health check. */

import { readFile } from 'node:fs/promises'
import { join } from 'node:path'
import { homedir } from 'node:os'

const PORT_FILE_DIR = '.openpencil'
const PORT_FILE_NAME = '.port'
const PORT_FILE_PATH = join(homedir(), PORT_FILE_DIR, PORT_FILE_NAME)

function isPidAlive(pid: number): boolean {
  try {
    process.kill(pid, 0)
    return true
  } catch (err: unknown) {
    return (err as NodeJS.ErrnoException).code === 'EPERM'
  }
}

export interface AppInfo {
  port: number
  pid: number
  timestamp: number
  url: string
}

/** Read port file and return app info, or null if no running instance. */
export async function getAppInfo(): Promise<AppInfo | null> {
  try {
    const raw = await readFile(PORT_FILE_PATH, 'utf-8')
    const { port, pid, timestamp } = JSON.parse(raw) as {
      port: number
      pid: number
      timestamp: number
    }
    if (!isPidAlive(pid)) return null
    return { port, pid, timestamp, url: `http://127.0.0.1:${port}` }
  } catch {
    return null
  }
}

/** Get app URL or throw if not running. */
export async function requireApp(): Promise<string> {
  const info = await getAppInfo()
  if (!info) {
    throw new Error(
      'No running OpenPencil instance found. Run `openpencil start` first.',
    )
  }
  return info.url
}

/** Quick check if app is running. */
export async function isAppRunning(): Promise<boolean> {
  return (await getAppInfo()) !== null
}
