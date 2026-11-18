/** Page-local live selection store shared by the sidebar and conversation. */

export interface OpenPencilNodeSelection {
  id: string
  type?: string
  name?: string
  x?: number
  y?: number
  width?: number
  height?: number
}

export interface OpenPencilLiveSelection {
  sourcePath: string
  activePageId: string
  selectedIds: string[]
  nodes: OpenPencilNodeSelection[]
  updatedAt: number
}

export interface OpenPencilSelectionStoreSnapshot {
  revision: number
  selection?: OpenPencilLiveSelection
}

const stores = new Map<string, OpenPencilSelectionStoreSnapshot>()
const listeners = new Map<string, Set<() => void>>()
const EMPTY_SELECTION_STORE_SNAPSHOT: OpenPencilSelectionStoreSnapshot = Object.freeze({ revision: 0 })

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function finite(value: unknown): number | undefined {
  return typeof value === 'number' && Number.isFinite(value) ? value : undefined
}

function nodeOf(value: unknown): OpenPencilNodeSelection | undefined {
  if (!isRecord(value) || typeof value.id !== 'string' || value.id.length === 0) return undefined
  return {
    id: value.id,
    ...(typeof value.type === 'string' && value.type.length > 0 ? { type: value.type } : {}),
    ...(typeof value.name === 'string' && value.name.length > 0 ? { name: value.name } : {}),
    ...(finite(value.x) === undefined ? {} : { x: finite(value.x) }),
    ...(finite(value.y) === undefined ? {} : { y: finite(value.y) }),
    ...(finite(value.width) === undefined ? {} : { width: finite(value.width) }),
    ...(finite(value.height) === undefined ? {} : { height: finite(value.height) }),
  }
}

export function liveSelectionOf(value: unknown): OpenPencilLiveSelection | undefined {
  if (!isRecord(value) || typeof value.sourcePath !== 'string' || value.sourcePath.length === 0) return undefined
  return {
    sourcePath: value.sourcePath,
    activePageId: typeof value.activePageId === 'string' ? value.activePageId : '',
    selectedIds: Array.isArray(value.selectedIds)
      ? value.selectedIds.filter((id): id is string => typeof id === 'string' && id.length > 0)
      : [],
    nodes: Array.isArray(value.nodes)
      ? value.nodes.map(nodeOf).filter((node): node is OpenPencilNodeSelection => node !== undefined)
      : [],
    updatedAt: finite(value.updatedAt) ?? Date.now(),
  }
}

function sameSelection(a: OpenPencilLiveSelection | undefined, b: OpenPencilLiveSelection | undefined): boolean {
  if (a === undefined || b === undefined) return a === b
  return a.sourcePath === b.sourcePath
    && a.activePageId === b.activePageId
    && JSON.stringify(a.selectedIds) === JSON.stringify(b.selectedIds)
    && JSON.stringify(a.nodes) === JSON.stringify(b.nodes)
}

export function publishOpenPencilSelection(sessionId: string, selection: OpenPencilLiveSelection): void {
  const current = stores.get(sessionId) ?? { revision: 0 }
  if (sameSelection(current.selection, selection)) return
  stores.set(sessionId, { revision: current.revision + 1, selection })
  for (const listener of listeners.get(sessionId) ?? []) listener()
}

export function clearOpenPencilSelection(sessionId: string, sourcePath?: string): void {
  const current = stores.get(sessionId)
  if (current === undefined) return
  if (current.selection === undefined) return
  if (sourcePath !== undefined && current.selection.sourcePath !== sourcePath) return
  stores.set(sessionId, { revision: current.revision + 1 })
  for (const listener of listeners.get(sessionId) ?? []) listener()
}

export function subscribeOpenPencilSelection(sessionId: string, listener: () => void): () => void {
  let scoped = listeners.get(sessionId)
  if (scoped === undefined) {
    scoped = new Set()
    listeners.set(sessionId, scoped)
  }
  scoped.add(listener)
  return () => {
    scoped?.delete(listener)
    if (scoped?.size === 0) listeners.delete(sessionId)
  }
}

export function getOpenPencilSelectionSnapshot(sessionId: string): OpenPencilSelectionStoreSnapshot {
  // useSyncExternalStore requires referentially stable snapshots while the
  // underlying state is unchanged. Returning a new empty object here causes
  // React to enter an update loop before the first selection poll completes.
  return stores.get(sessionId) ?? EMPTY_SELECTION_STORE_SNAPSHOT
}
