/**
 * In-memory sync state for MCP <-> Renderer real-time communication.
 * Shared across Nitro API endpoints: GET/POST /api/mcp/document, GET /api/mcp/events.
 */

import type { PenDocument } from '../../src/types/pen'

let currentDocument: PenDocument | null = null
let documentVersion = 0
let currentSelection: string[] = []
let currentActivePageId: string | null = null

interface SSEWriter {
  push(data: string): void
}

interface SSEClient {
  id: string
  writer: SSEWriter
}

const clients = new Map<string, SSEClient>()

export function getSyncDocument(): { doc: PenDocument | null; version: number } {
  return { doc: currentDocument, version: documentVersion }
}

export function setSyncDocument(doc: PenDocument, sourceClientId?: string): number {
  currentDocument = doc
  documentVersion++
  broadcast({ type: 'document:update', version: documentVersion, document: doc }, sourceClientId)
  return documentVersion
}

export function getSyncSelection(): { selectedIds: string[]; activePageId: string | null } {
  return { selectedIds: currentSelection, activePageId: currentActivePageId }
}

export function clearSyncState(): void {
  currentDocument = null
  documentVersion = 0
  currentSelection = []
  currentActivePageId = null
}

export function setSyncSelection(selectedIds: string[], activePageId?: string | null): void {
  currentSelection = selectedIds
  if (activePageId !== undefined) currentActivePageId = activePageId
}

export function registerSSEClient(id: string, writer: SSEWriter): void {
  clients.set(id, { id, writer })
}

export function unregisterSSEClient(id: string): void {
  clients.delete(id)
}

function broadcast(payload: Record<string, unknown>, excludeClientId?: string): void {
  const data = JSON.stringify(payload)
  for (const [id, client] of clients) {
    if (id === excludeClientId) continue
    try {
      client.writer.push(data)
    } catch {
      clients.delete(id)
    }
  }
}
