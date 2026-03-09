import { useEffect, useRef } from 'react'
import { useDocumentStore } from '@/stores/document-store'
import type { PenDocument } from '@/types/pen'

const PUSH_DEBOUNCE_MS = 2000
const RECONNECT_DELAY_MS = 3000

function getBaseUrl(): string {
  return window.location.origin
}

function pushDocumentToServer(clientId: string | null) {
  const doc = useDocumentStore.getState().document
  fetch(`${getBaseUrl()}/api/mcp/document`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ document: doc, sourceClientId: clientId }),
  }).catch(() => {})
}

/**
 * Subscribes the renderer to MCP sync events via SSE.
 * - Receives document updates from MCP and applies them to the canvas.
 * - Pushes local document changes to Nitro so MCP can read them.
 */
export function useMcpSync() {
  const clientIdRef = useRef<string | null>(null)
  const pushTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  // Skip debounce pushes briefly after applying an external document.
  // Use a timestamp instead of a boolean so cascading setState calls
  // (e.g. canvas sync page switch handler) are also suppressed.
  const skipPushUntilRef = useRef(0)

  useEffect(() => {
    const baseUrl = getBaseUrl()
    let eventSource: EventSource | null = null
    let reconnectTimer: ReturnType<typeof setTimeout> | null = null
    let disposed = false

    function connect() {
      if (disposed) return
      eventSource = new EventSource(`${baseUrl}/api/mcp/events`)

      eventSource.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data)

          if (data.type === 'client:id') {
            clientIdRef.current = data.clientId
            // Push current document so MCP can read it immediately
            pushDocumentToServer(data.clientId)
          } else if (data.type === 'document:update') {
            const doc = data.document as PenDocument
            const childCount = doc.pages?.[0]?.children?.length ?? doc.children?.length ?? 0
            console.log('[mcp-sync] Received document:update, top-level children:', childCount)
            // Suppress push-back for a short window — applyExternalDocument
            // may trigger multiple cascading setState calls.
            skipPushUntilRef.current = Date.now() + 200
            useDocumentStore.getState().applyExternalDocument(doc)
          }
        } catch {
          // Ignore malformed events
        }
      }

      eventSource.onerror = () => {
        eventSource?.close()
        eventSource = null
        if (!disposed) {
          reconnectTimer = setTimeout(connect, RECONNECT_DELAY_MS)
        }
      }
    }

    connect()

    // Push local document changes to Nitro (debounced)
    const unsubDoc = useDocumentStore.subscribe(() => {
      if (Date.now() < skipPushUntilRef.current) return
      if (pushTimerRef.current) clearTimeout(pushTimerRef.current)
      pushTimerRef.current = setTimeout(() => {
        pushDocumentToServer(clientIdRef.current)
      }, PUSH_DEBOUNCE_MS)
    })

    return () => {
      disposed = true
      eventSource?.close()
      if (reconnectTimer) clearTimeout(reconnectTimer)
      if (pushTimerRef.current) clearTimeout(pushTimerRef.current)
      unsubDoc()
    }
  }, [])
}
