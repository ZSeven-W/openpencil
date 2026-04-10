/**
 * In-memory sync state for MCP <-> Renderer real-time communication.
 * Shared across Nitro API endpoints: GET/POST /api/mcp/document, GET /api/mcp/events.
 */

import type { PenDocument } from '../../src/types/pen';

let currentDocument: PenDocument | null = null;
let documentVersion = 0;
let currentSelection: string[] = [];
let currentActivePageId: string | null = null;
let lastActiveClientId: string | null = null;

interface SSEWriter {
  push(data: string): void;
}

interface SSEClient {
  id: string;
  writer: SSEWriter;
}

const clients = new Map<string, SSEClient>();

export function getSyncDocument(): { doc: PenDocument | null; version: number } {
  return { doc: currentDocument, version: documentVersion };
}

export function setSyncDocument(doc: PenDocument, sourceClientId?: string): number {
  currentDocument = doc;
  documentVersion++;
  if (sourceClientId) lastActiveClientId = sourceClientId;
  broadcast({ type: 'document:update', version: documentVersion, document: doc }, sourceClientId);
  return documentVersion;
}

export function getSyncSelection(): { selectedIds: string[]; activePageId: string | null } {
  return { selectedIds: currentSelection, activePageId: currentActivePageId };
}

export function clearSyncState(): void {
  currentDocument = null;
  documentVersion = 0;
  currentSelection = [];
  currentActivePageId = null;
  lastActiveClientId = null;
}

export function setSyncSelection(
  selectedIds: string[],
  activePageId?: string | null,
  sourceClientId?: string,
): void {
  currentSelection = selectedIds;
  if (activePageId !== undefined) currentActivePageId = activePageId;
  if (sourceClientId) lastActiveClientId = sourceClientId;
}

export function registerSSEClient(id: string, writer: SSEWriter): void {
  clients.set(id, { id, writer });
}

export function unregisterSSEClient(id: string): void {
  clients.delete(id);
}

function broadcast(payload: Record<string, unknown>, excludeClientId?: string): void {
  const data = JSON.stringify(payload);
  for (const [id, client] of clients) {
    if (id === excludeClientId) continue;
    try {
      client.writer.push(data);
    } catch {
      clients.delete(id);
    }
  }
}

export function markClientActive(clientId: string): void {
  if (clients.has(clientId)) {
    lastActiveClientId = clientId;
  }
}

export function getLastActiveClientId(): string | null {
  return lastActiveClientId;
}

export function isClientConnected(clientId: string): boolean {
  return clients.has(clientId);
}

export function sendToClient(clientId: string, payload: Record<string, unknown>): boolean {
  const client = clients.get(clientId);
  if (!client) return false;
  try {
    client.writer.push(JSON.stringify(payload));
    return true;
  } catch {
    clients.delete(clientId);
    return false;
  }
}
