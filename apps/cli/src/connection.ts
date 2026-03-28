/** Port file discovery and app health check. */

import { readFile, unlink } from 'node:fs/promises'
import { join } from 'node:path'
import { homedir } from 'node:os'

const PORT_FILE_DIR = '.openpencil'
const PORT_FILE_NAME = '.port'
const PORT_FILE_PATH = join(homedir(), PORT_FILE_DIR, PORT_FILE_NAME)
const APP_BASE_URLS = ['http://127.0.0.1', 'http://localhost']

async function getReachableAppUrl(port: number): Promise<string | null> {
  for (const baseUrl of APP_BASE_URLS) {
    const url = `${baseUrl}:${port}/api/mcp/server`
    for (let attempt = 0; attempt < 5; attempt++) {
      try {
        const res = await fetch(url, {
          signal: AbortSignal.timeout(500),
        })
        if (res.ok) return `${baseUrl}:${port}`
      } catch {
        // App may still be starting, or the port file may be stale.
      }
      if (attempt < 4) {
        await new Promise((resolve) => setTimeout(resolve, 200))
      }
    }
  }
  return null
}

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
    const url = await getReachableAppUrl(port)
    if (url) {
      return { port, pid, timestamp, url }
    }
    if (!isPidAlive(pid)) {
      try {
        await unlink(PORT_FILE_PATH)
      } catch {
        // Ignore stale port file cleanup failures.
      }
      return null
    }
    return null
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
