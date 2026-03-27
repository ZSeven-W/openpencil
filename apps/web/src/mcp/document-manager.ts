import { readFile, writeFile, access, unlink } from 'node:fs/promises'
import { constants } from 'node:fs'
import { homedir } from 'node:os'
import { join, resolve } from 'node:path'
import type { PenDocument } from '../types/pen'
import { sanitizeObject } from './utils/sanitize'
import { PORT_FILE_DIR_NAME, PORT_FILE_NAME } from '@/constants/app'

const cache = new Map<string, { doc: PenDocument; mtime: number }>()

/** Special path indicating the MCP should operate on the live Electron canvas. */
export const LIVE_CANVAS_PATH = 'live://canvas'

/** Resolve filePath for MCP tools — defaults to live canvas when omitted. */
export function resolveDocPath(filePath?: string): string {
  if (!filePath || filePath === LIVE_CANVAS_PATH) return LIVE_CANVAS_PATH
  return resolve(filePath)
}

// ---------------------------------------------------------------------------
// Sync URL cache — avoids repeated port file reads + health checks
// ---------------------------------------------------------------------------

let _cachedSyncUrl: string | null = null
let _cachedSyncUrlTime = 0
const SYNC_URL_TTL = 30_000 // 30 seconds

/** Pre-set the sync URL (e.g. from CLI connection discovery). */
export function setSyncUrl(url: string): void {
  _cachedSyncUrl = url
  _cachedSyncUrlTime = Date.now()
}

/** Clear the cached sync URL (e.g. after connection failure). */
export function clearSyncUrl(): void {
  _cachedSyncUrl = null
  _cachedSyncUrlTime = 0
}

const PORT_FILE_PATH = join(homedir(), PORT_FILE_DIR_NAME, PORT_FILE_NAME)
const SYNC_BASE_URLS = ['http://127.0.0.1', 'http://localhost']

async function getReachableSyncUrl(port: number): Promise<string | null> {
  for (const baseUrl of SYNC_BASE_URLS) {
    const url = `${baseUrl}:${port}/api/mcp/server`
    for (let attempt = 0; attempt < 5; attempt++) {
      try {
        const res = await fetch(url, {
          signal: AbortSignal.timeout(500),
        })
        if (res.ok) return `${baseUrl}:${port}`
      } catch {
        // Server may still be starting, or the port file may be stale.
      }
      if (attempt < 4) {
        await new Promise((resolve) => setTimeout(resolve, 200))
      }
    }
  }
  return null
}

type LiveSyncAvailability = 'connected' | 'no-document' | 'unreachable' | 'missing-port-file'

export interface LiveSyncState {
  status: LiveSyncAvailability
  url: string | null
  port: number | null
  message: string
}

async function probeLiveSyncUrl(baseUrl: string): Promise<LiveSyncAvailability> {
  try {
    const docRes = await fetch(`${baseUrl}/api/mcp/document`, {
      signal: AbortSignal.timeout(500),
    })
    if (docRes.ok) return 'connected'
    if (docRes.status === 404) return 'no-document'
  } catch {
    // Ignore and try lighter health probes below.
  }

  try {
    const selectionRes = await fetch(`${baseUrl}/api/mcp/selection`, {
      signal: AbortSignal.timeout(500),
    })
    if (selectionRes.ok) return 'no-document'
  } catch {
    // Ignore and try generic server status.
  }

  try {
    const serverRes = await fetch(`${baseUrl}/api/mcp/server`, {
      signal: AbortSignal.timeout(500),
    })
    if (serverRes.ok) return 'no-document'
  } catch {
    // Ignore.
  }

  return 'unreachable'
}

function buildLiveSyncMessage(status: LiveSyncAvailability, port?: number | null): string {
  const portHint = port ? ` (port ${port})` : ''
  switch (status) {
    case 'connected':
      return `Connected to OpenPencil live canvas${portHint}.`
    case 'no-document':
      return `OpenPencil is running${portHint}, but no live document is loaded in the editor yet. Open the editor page and wait for sync.`
    case 'unreachable':
      return `Found an OpenPencil port file${portHint}, but the live sync server is unreachable. Restart OpenPencil and try again.`
    case 'missing-port-file':
    default:
      return 'No running OpenPencil instance found. Start the Electron app or dev server first.'
  }
}

// ---------------------------------------------------------------------------
// Sync URL discovery
// ---------------------------------------------------------------------------

function isPidAlive(pid: number): boolean {
  try {
    process.kill(pid, 0)
    return true
  } catch (err: unknown) {
    return (err as NodeJS.ErrnoException).code === 'EPERM' // process exists but we lack permission
  }
}

/** Read the port file and return the Nitro sync base URL, or null if unavailable. */
export async function getSyncUrl(): Promise<string | null> {
  // Use cached URL if still fresh
  if (_cachedSyncUrl && Date.now() - _cachedSyncUrlTime < SYNC_URL_TTL) {
    return _cachedSyncUrl
  }
  const state = await getLiveSyncState()
  if ((state.status === 'connected' || state.status === 'no-document') && state.url) {
    _cachedSyncUrl = state.url
    _cachedSyncUrlTime = Date.now()
    return state.url
  }
  return null
}

/** Inspect the current live canvas sync state with a user-facing diagnosis. */
export async function getLiveSyncState(): Promise<LiveSyncState> {
  try {
    const raw = await readFile(PORT_FILE_PATH, 'utf-8')
    const { port, pid } = JSON.parse(raw) as { port: number; pid: number }
    const url = await getReachableSyncUrl(port)
    if (url) {
      const status = await probeLiveSyncUrl(url)
      if (status === 'unreachable') {
        return {
          status,
          url: null,
          port,
          message: buildLiveSyncMessage(status, port),
        }
      }

      return {
        status,
        url,
        port,
        message: buildLiveSyncMessage(status, port),
      }
    }

    if (!isPidAlive(pid)) {
      try {
        await unlink(PORT_FILE_PATH)
      } catch {
        // Ignore cleanup failures for stale port files.
      }
      return {
        status: 'missing-port-file',
        url: null,
        port: null,
        message: buildLiveSyncMessage('missing-port-file'),
      }
    }

    return {
      status: 'unreachable',
      url: null,
      port,
      message: buildLiveSyncMessage('unreachable', port),
    }
  } catch {
    return {
      status: 'missing-port-file',
      url: null,
      port: null,
      message: buildLiveSyncMessage('missing-port-file'),
    }
  }
}

/** Fetch the current document from the live Electron canvas. */
async function fetchLiveDocument(): Promise<PenDocument> {
  // Fast path: use cached sync URL
  const cachedUrl = _cachedSyncUrl && Date.now() - _cachedSyncUrlTime < SYNC_URL_TTL
    ? _cachedSyncUrl : null

  if (cachedUrl) {
    try {
      const res = await fetch(`${cachedUrl}/api/mcp/document`)
      if (res.ok) {
        const data = (await res.json()) as { document: PenDocument }
        return data.document
      }
    } catch {
      // Cache stale — fall through to full discovery
      clearSyncUrl()
    }
  }

  const sync = await getLiveSyncState()
  if (!sync.url || sync.status !== 'connected') {
    throw new Error(sync.message)
  }
  _cachedSyncUrl = sync.url
  _cachedSyncUrlTime = Date.now()
  const res = await fetch(`${sync.url}/api/mcp/document`)
  if (!res.ok) {
    const body = await res.json().catch(() => ({} as Record<string, unknown>))
    throw new Error(
      (body as { error?: string }).error ?? `Failed to fetch live document: ${res.status}`,
    )
  }
  const data = (await res.json()) as { document: PenDocument }
  return data.document
}

/** Push document to the live Electron canvas. Fails silently if unavailable. */
async function pushLiveDocument(doc: PenDocument): Promise<void> {
  // Fast path: use cached sync URL
  const cachedUrl = _cachedSyncUrl && Date.now() - _cachedSyncUrlTime < SYNC_URL_TTL
    ? _cachedSyncUrl : null
  const syncUrl = cachedUrl ?? await getSyncUrl()
  if (!syncUrl) return
  try {
    await fetch(`${syncUrl}/api/mcp/document`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ document: doc }),
    })
  } catch {
    // Network error — Electron might have quit between check and request
    clearSyncUrl()
  }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/** Validate that a parsed object looks like a PenDocument. */
function validate(doc: unknown): doc is PenDocument {
  if (!doc || typeof doc !== 'object') return false
  const d = doc as Record<string, unknown>
  // Accept docs with children array or pages array
  return typeof d.version === 'string' && (Array.isArray(d.children) || Array.isArray(d.pages))
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/** Read and parse a .op / .pen file, returning a PenDocument. Uses cache. */
export async function openDocument(filePath: string): Promise<PenDocument> {
  // Live canvas mode: always re-fetch from running Electron/dev server
  // to pick up user edits made in the UI since the last MCP call.
  if (filePath === LIVE_CANVAS_PATH) {
    const doc = await fetchLiveDocument()
    cache.set(LIVE_CANVAS_PATH, { doc, mtime: Date.now() })
    return doc
  }

  const cached = cache.get(filePath)
  if (cached) return cached.doc

  await access(filePath, constants.R_OK)
  const text = await readFile(filePath, 'utf-8')
  const raw = JSON.parse(text)
  const sanitized = sanitizeObject(raw)
  if (!validate(sanitized)) {
    throw new Error(`Invalid document format: ${filePath}`)
  }
  cache.set(filePath, { doc: sanitized, mtime: Date.now() })
  return sanitized
}

/** Create a new empty document (not saved to disk yet). */
export function createEmptyDocument(): PenDocument {
  return {
    version: '1.0.0',
    children: [],
  }
}

/** Write a PenDocument to disk and update cache. Also pushes to live canvas if available. */
export async function saveDocument(
  filePath: string,
  doc: PenDocument,
): Promise<void> {
  if (filePath === LIVE_CANVAS_PATH) {
    // Live canvas mode: push to Electron, no disk write
    cache.set(LIVE_CANVAS_PATH, { doc, mtime: Date.now() })
    await pushLiveDocument(doc)
    return
  }

  // File-based: write to disk (no indentation to minimize file size)
  const json = JSON.stringify(doc)
  await writeFile(filePath, json, 'utf-8')
  cache.set(filePath, { doc, mtime: Date.now() })

  // Also push to live canvas (dual-write so canvas updates even for file-based MCP use)
  await pushLiveDocument(doc)
}

/** Get document from cache (for tools that operate on the active doc). */
export function getCachedDocument(
  filePath: string,
): PenDocument | undefined {
  return cache.get(filePath)?.doc
}

/** Update the cached document in-memory (call saveDocument to persist). */
export function setCachedDocument(
  filePath: string,
  doc: PenDocument,
): void {
  cache.set(filePath, { doc, mtime: Date.now() })
}

/** Check if a file exists. */
export async function fileExists(filePath: string): Promise<boolean> {
  try {
    await access(filePath, constants.F_OK)
    return true
  } catch {
    return false
  }
}

/** Fetch the current selection from the live Electron canvas. */
export async function fetchLiveSelection(): Promise<{ selectedIds: string[]; activePageId: string | null }> {
  // Fast path: use cached sync URL
  const cachedUrl = _cachedSyncUrl && Date.now() - _cachedSyncUrlTime < SYNC_URL_TTL
    ? _cachedSyncUrl : null

  if (cachedUrl) {
    try {
      const res = await fetch(`${cachedUrl}/api/mcp/selection`)
      if (res.ok) return (await res.json()) as { selectedIds: string[]; activePageId: string | null }
    } catch {
      clearSyncUrl()
    }
  }

  const sync = await getLiveSyncState()
  if (!sync.url) {
    throw new Error(sync.message)
  }
  if (sync.status === 'no-document') {
    return { selectedIds: [], activePageId: null }
  }
  _cachedSyncUrl = sync.url
  _cachedSyncUrlTime = Date.now()
  try {
    const res = await fetch(`${sync.url}/api/mcp/selection`)
    if (!res.ok) return { selectedIds: [], activePageId: null }
    return (await res.json()) as { selectedIds: string[]; activePageId: string | null }
  } catch {
    throw new Error(buildLiveSyncMessage('unreachable', sync.port))
  }
}

export { probeLiveSyncUrl, buildLiveSyncMessage }

/** Invalidate cache for a file. */
export function invalidateCache(filePath: string): void {
  cache.delete(filePath)
}
