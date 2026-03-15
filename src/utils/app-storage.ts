/**
 * Cross-environment storage abstraction.
 *
 * In Electron the Nitro server starts on a random port each launch, which
 * changes the origin and wipes `localStorage`. This module provides a
 * synchronous `getItem` / `setItem` / `removeItem` API that:
 *
 * - **Electron**: reads from an in-memory cache that was pre-loaded from a
 *   JSON preferences file via IPC at startup. Writes update the cache
 *   immediately (synchronous) and persist to disk asynchronously.
 * - **Web**: delegates directly to `localStorage`.
 *
 * Call `initAppStorage()` once at app startup (before any store hydration)
 * and `await` the result. In web mode it resolves instantly.
 */

/** In-memory cache for Electron mode. */
let cache: Record<string, string> | null = null
let initPromise: Promise<void> | null = null

/** Whether we are running inside Electron with the IPC bridge available. */
function isElectron(): boolean {
  return typeof window !== 'undefined' && !!window.electronAPI?.getPreferences
}

/**
 * Initialise the storage layer. Must be called (and awaited) before any
 * store hydration so that the cache is populated. Idempotent — multiple
 * calls return the same promise.
 */
export async function initAppStorage(): Promise<void> {
  if (typeof window === 'undefined') return
  if (!isElectron()) return
  if (initPromise) return initPromise
  initPromise = (async () => {
    try {
      const prefs: Record<string, string> =
        await window.electronAPI!.getPreferences()
      cache = prefs ?? {}
    } catch {
      cache = {}
    }
  })()
  return initPromise
}

/** Synchronous get — reads from cache (Electron) or localStorage (web). */
export function getItem(key: string): string | null {
  if (cache !== null) {
    return cache[key] ?? null
  }
  if (typeof window === 'undefined') return null
  try {
    return localStorage.getItem(key)
  } catch {
    return null
  }
}

/** Synchronous set — updates cache + fires async IPC write in Electron. */
export function setItem(key: string, value: string): void {
  if (cache !== null) {
    cache[key] = value
    window.electronAPI
      ?.setPreference(key, value)
      ?.catch(() => {})
    return
  }
  try {
    localStorage.setItem(key, value)
  } catch {
    // quota exceeded or private mode
  }
}

/** Synchronous remove — updates cache + fires async IPC write in Electron. */
export function removeItem(key: string): void {
  if (cache !== null) {
    delete cache[key]
    window.electronAPI
      ?.removePreference(key)
      ?.catch(() => {})
    return
  }
  try {
    localStorage.removeItem(key)
  } catch {
    // ignore
  }
}

/**
 * Convenience re-export so stores can do:
 *   import { appStorage } from '@/utils/app-storage'
 *   appStorage.getItem(...)
 */
export const appStorage = { getItem, setItem, removeItem }
