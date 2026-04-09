import { useEffect, useRef } from 'react'
import { useDocumentStore } from '@/stores/document-store'
import { useCanvasStore } from '@/stores/canvas-store'
import type { PenDocument } from '@/types/pen'

const PUSH_DEBOUNCE_MS = 2000
const SELECTION_DEBOUNCE_MS = 300
const RECONNECT_DELAY_MS = 3000
const MAX_RECONNECT_ATTEMPTS = 3
const SYNC_MAX_BODY_BYTES = 2 * 1024 * 1024

let oversizeSyncWarned = false

function getBaseUrl(): string {
  return window.location.origin
}

async function pushDocumentToServer(clientId: string | null) {
  const doc = useDocumentStore.getState().document
  const body = JSON.stringify({ document: doc, sourceClientId: clientId })
  const bodyBytes = new TextEncoder().encode(body).byteLength

  if (bodyBytes > SYNC_MAX_BODY_BYTES) {
    if (!oversizeSyncWarned) {
      oversizeSyncWarned = true
      console.warn(
        `[mcp-sync] Skip oversized document push: ${(
          bodyBytes / (1024 * 1024)
        ).toFixed(2)}MiB > ${(SYNC_MAX_BODY_BYTES / (1024 * 1024)).toFixed(2)}MiB`,
      )
    }
    return
  }

  oversizeSyncWarned = false

  await fetch(`${getBaseUrl()}/api/mcp/document`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'x-openpencil-client-id': clientId ?? 'renderer:unknown',
      'x-openpencil-body-bytes': String(bodyBytes),
    },
    // 小请求在页面切换 / HMR 时尽量继续发完, 降低“客户端刚好中断”概率。
    ...(bodyBytes <= 60_000 ? { keepalive: true } : {}),
    // 给大文档同步留出更长的本地回环传输窗口, 避免默认超时过早打断。
    signal: AbortSignal.timeout(30_000),
    body,
  })
}

/**
 * Subscribes the renderer to MCP sync events via SSE.
 * - Receives document updates from MCP and applies them to the canvas.
 * - Pushes local document changes to Nitro so MCP can read them.
 */
export function useMcpSync() {
  const clientIdRef = useRef<string | null>(null)
  const pushTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const pushInFlightRef = useRef(false)
  const pushQueuedRef = useRef(false)
  const queuedClientIdRef = useRef<string | null>(null)
  // Skip debounce pushes briefly after applying an external document.
  // Use a timestamp instead of a boolean so cascading setState calls
  // (e.g. canvas sync page switch handler) are also suppressed.
  const skipPushUntilRef = useRef(0)

  useEffect(() => {
    const baseUrl = getBaseUrl()
    let eventSource: EventSource | null = null
    let reconnectTimer: ReturnType<typeof setTimeout> | null = null
    let disposed = false
    let reconnectAttempts = 0

    async function flushDocumentPush(clientId: string | null) {
      if (disposed) return
      if (pushInFlightRef.current) {
        pushQueuedRef.current = true
        queuedClientIdRef.current = clientId
        return
      }

      pushInFlightRef.current = true
      try {
        await pushDocumentToServer(clientId)
      } catch {
        // MCP sync 是增强能力, 失败时不打断主编辑器流程。
      } finally {
        pushInFlightRef.current = false
        if (pushQueuedRef.current && !disposed) {
          const nextClientId = queuedClientIdRef.current
          pushQueuedRef.current = false
          queuedClientIdRef.current = null
          void flushDocumentPush(nextClientId)
        }
      }
    }

    function connect() {
      if (disposed) return
      eventSource = new EventSource(`${baseUrl}/api/mcp/events`)

      eventSource.onmessage = (event) => {
        reconnectAttempts = 0 // Reset on successful message
        try {
          const data = JSON.parse(event.data)

          if (data.type === 'client:id') {
            clientIdRef.current = data.clientId
            // Push current document so MCP can read it immediately
            void flushDocumentPush(data.clientId)
          } else if (data.type === 'document:update' || data.type === 'document:init') {
            const doc = data.document as PenDocument
            const childCount = doc.pages?.[0]?.children?.length ?? doc.children?.length ?? 0
            console.log(`[mcp-sync] Received ${data.type}, top-level children:`, childCount)
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
        reconnectAttempts++
        if (!disposed && reconnectAttempts <= MAX_RECONNECT_ATTEMPTS) {
          reconnectTimer = setTimeout(connect, RECONNECT_DELAY_MS)
        }
        // Stop retrying after MAX_RECONNECT_ATTEMPTS to avoid console spam.
        // MCP sync is optional — the editor works fine without it.
      }
    }

    // Clear stale server cache from previous session before connecting.
    // This prevents document:init from echoing back an old document and
    // falsely marking the editor as dirty after a page refresh or file open.
    fetch(`${baseUrl}/api/mcp/sync-reset`, { method: 'POST' })
      .catch(() => {})
      .finally(() => {
        if (!disposed) connect()
      })

    // Push local document changes to Nitro (debounced).
    // On loadDocument/newDocument (isDirty transitions to false), push
    // immediately so the server cache is replaced without waiting 2s.
    const unsubDoc = useDocumentStore.subscribe((state, prevState) => {
      const documentChanged = state.document !== prevState.document
      const dirtyChanged = state.isDirty !== prevState.isDirty
      if (!documentChanged && !dirtyChanged) return
      if (Date.now() < skipPushUntilRef.current) return
      if (pushTimerRef.current) clearTimeout(pushTimerRef.current)

      const isLoadEvent = !state.isDirty && prevState.isDirty !== state.isDirty
      if (isLoadEvent) {
        void flushDocumentPush(clientIdRef.current)
        return
      }

      pushTimerRef.current = setTimeout(() => {
        void flushDocumentPush(clientIdRef.current)
      }, PUSH_DEBOUNCE_MS)
    })

    // Push selection changes to Nitro (debounced)
    let selectionTimer: ReturnType<typeof setTimeout> | null = null
    let prevSelectedIds: string[] = []
    let prevActivePageId: string | null = null

    const unsubSelection = useCanvasStore.subscribe((state) => {
      const { selectedIds } = state.selection
      const { activePageId } = state
      // Skip if nothing changed
      if (selectedIds === prevSelectedIds && activePageId === prevActivePageId) return
      prevSelectedIds = selectedIds
      prevActivePageId = activePageId

      if (selectionTimer) clearTimeout(selectionTimer)
      selectionTimer = setTimeout(() => {
        fetch(`${baseUrl}/api/mcp/selection`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ selectedIds, activePageId }),
        }).catch(() => {})
      }, SELECTION_DEBOUNCE_MS)
    })

    return () => {
      disposed = true
      eventSource?.close()
      if (reconnectTimer) clearTimeout(reconnectTimer)
      if (pushTimerRef.current) clearTimeout(pushTimerRef.current)
      if (selectionTimer) clearTimeout(selectionTimer)
      unsubDoc()
      unsubSelection()
    }
  }, [])
}
