/**
 * In-memory sync state for MCP ↔ Renderer real-time communication.
 * Shared across Nitro API endpoints: GET/POST /api/mcp/document, GET /api/mcp/events.
 */

import type { PenDocument } from '../../src/types/pen'
import type { ServerResponse } from 'node:http'

let currentDocument: PenDocument | null = null
let documentVersion = 0
let currentSelection: string[] = []
let currentActivePageId: string | null = null

interface SSEClient {
  id: string
  res: ServerResponse
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

export function setSyncSelection(selectedIds: string[], activePageId?: string | null): void {
  currentSelection = selectedIds
  if (activePageId !== undefined) currentActivePageId = activePageId
}

export function registerSSEClient(id: string, res: ServerResponse): void {
  clients.set(id, { id, res })
}

export function unregisterSSEClient(id: string): void {
  clients.delete(id)
}

function broadcast(payload: Record<string, unknown>, excludeClientId?: string): void {
  const data = `data: ${JSON.stringify(payload)}\n\n`
  for (const [id, client] of clients) {
    if (id === excludeClientId) continue
    try {
      if (!client.res.closed) client.res.write(data)
    } catch {
      clients.delete(id)
    }
  }
}
