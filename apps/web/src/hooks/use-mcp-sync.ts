import { useEffect, useRef } from 'react';
import { useDocumentStore } from '@/stores/document-store';
import { useCanvasStore } from '@/stores/canvas-store';
import type { PenDocument } from '@/types/pen';

const PUSH_DEBOUNCE_MS = 2000;
const SELECTION_DEBOUNCE_MS = 300;
const RECONNECT_DELAY_MS = 3000;
const MAX_RECONNECT_ATTEMPTS = 3;

async function handleScreenshotRequest(
  req: {
    requestId: string;
    bounds?: { x: number; y: number; w: number; h: number } | 'root';
    nodeId?: string;
    opts?: { dpr?: number; padding?: number };
    timeoutMs: number;
  },
  baseUrl: string,
): Promise<void> {
  const { getSkiaEngineRef } = await import('@/canvas/skia-engine-ref');
  const engine = getSkiaEngineRef();

  const postResponse = (payload: Record<string, unknown>) => {
    fetch(`${baseUrl}/api/mcp/screenshot-response`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload),
    }).catch(() => {});
  };

  if (!engine) {
    postResponse({
      requestId: req.requestId,
      success: false,
      error: 'canvas not ready',
    });
    return;
  }

  try {
    // Resolve effective bounds: explicit rect wins; nodeId lookup otherwise; else 'root'.
    let bounds: { x: number; y: number; w: number; h: number } | 'root' = req.bounds ?? 'root';
    if (!req.bounds && req.nodeId) {
      const renderNode = engine.spatialIndex.get(req.nodeId);
      if (!renderNode) {
        postResponse({
          requestId: req.requestId,
          success: false,
          error: `node not found in spatial index: ${req.nodeId} (layout may not have run yet — try focusing the editor and retrying)`,
        });
        return;
      }
      bounds = {
        x: renderNode.absX,
        y: renderNode.absY,
        w: renderNode.absW,
        h: renderNode.absH,
      };
    }

    const png = await engine.captureRegion(bounds, req.opts);
    if (!png) {
      postResponse({
        requestId: req.requestId,
        success: false,
        error: 'readback failed (captureRegion returned null)',
      });
      return;
    }

    let binary = '';
    for (let i = 0; i < png.length; i++) binary += String.fromCharCode(png[i]);
    const pngBase64 = btoa(binary);

    postResponse({
      requestId: req.requestId,
      success: true,
      pngBase64,
      actualBounds: bounds === 'root' ? undefined : bounds,
    });
  } catch (err) {
    postResponse({
      requestId: req.requestId,
      success: false,
      error: err instanceof Error ? err.message : String(err),
    });
  }
}

function getBaseUrl(): string {
  return window.location.origin;
}

function pushDocumentToServer(clientId: string | null) {
  const doc = useDocumentStore.getState().document;
  fetch(`${getBaseUrl()}/api/mcp/document`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ document: doc, sourceClientId: clientId }),
  }).catch(() => {});
}

/**
 * Subscribes the renderer to MCP sync events via SSE.
 * - Receives document updates from MCP and applies them to the canvas.
 * - Pushes local document changes to Nitro so MCP can read them.
 */
export function useMcpSync() {
  const clientIdRef = useRef<string | null>(null);
  const pushTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  // Skip debounce pushes briefly after applying an external document.
  // Use a timestamp instead of a boolean so cascading setState calls
  // (e.g. canvas sync page switch handler) are also suppressed.
  const skipPushUntilRef = useRef(0);

  useEffect(() => {
    const baseUrl = getBaseUrl();
    let eventSource: EventSource | null = null;
    let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
    let disposed = false;
    let reconnectAttempts = 0;

    // ---- Focus / visibility ping: keep lastActiveClientId accurate ----
    const sendActivePing = () => {
      if (typeof document !== 'undefined' && document.hidden) return;
      const id = clientIdRef.current;
      if (!id) return;
      fetch(`${baseUrl}/api/mcp/active-ping`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ clientId: id }),
      }).catch(() => {});
    };

    if (typeof window !== 'undefined') {
      window.addEventListener('focus', sendActivePing);
      document.addEventListener('visibilitychange', sendActivePing);
    }

    function connect() {
      if (disposed) return;
      eventSource = new EventSource(`${baseUrl}/api/mcp/events`);

      eventSource.onmessage = (event) => {
        reconnectAttempts = 0; // Reset on successful message
        try {
          const data = JSON.parse(event.data);

          if (data.type === 'client:id') {
            clientIdRef.current = data.clientId;
            // Push current document so MCP can read it immediately
            pushDocumentToServer(data.clientId);
            // Announce this tab as the active one
            sendActivePing();
          } else if (data.type === 'document:update' || data.type === 'document:init') {
            const doc = data.document as PenDocument;
            const childCount = doc.pages?.[0]?.children?.length ?? doc.children?.length ?? 0;
            console.log(`[mcp-sync] Received ${data.type}, top-level children:`, childCount);
            // Suppress push-back for a short window — applyExternalDocument
            // may trigger multiple cascading setState calls.
            skipPushUntilRef.current = Date.now() + 200;
            useDocumentStore.getState().applyExternalDocument(doc);
          } else if (data.type === 'screenshot:request') {
            void handleScreenshotRequest(
              data as {
                requestId: string;
                bounds?: { x: number; y: number; w: number; h: number } | 'root';
                nodeId?: string;
                opts?: { dpr?: number; padding?: number };
                timeoutMs: number;
              },
              baseUrl,
            );
          }
        } catch {
          // Ignore malformed events
        }
      };

      eventSource.onerror = () => {
        eventSource?.close();
        eventSource = null;
        reconnectAttempts++;
        if (!disposed && reconnectAttempts <= MAX_RECONNECT_ATTEMPTS) {
          reconnectTimer = setTimeout(connect, RECONNECT_DELAY_MS);
        }
        // Stop retrying after MAX_RECONNECT_ATTEMPTS to avoid console spam.
        // MCP sync is optional — the editor works fine without it.
      };
    }

    // Clear stale server cache from previous session before connecting.
    // This prevents document:init from echoing back an old document and
    // falsely marking the editor as dirty after a page refresh or file open.
    fetch(`${baseUrl}/api/mcp/sync-reset`, { method: 'POST' })
      .catch(() => {})
      .finally(() => {
        if (!disposed) connect();
      });

    // Push local document changes to Nitro (debounced).
    // On loadDocument/newDocument (isDirty transitions to false), push
    // immediately so the server cache is replaced without waiting 2s.
    const unsubDoc = useDocumentStore.subscribe((state, prevState) => {
      if (Date.now() < skipPushUntilRef.current) return;
      if (pushTimerRef.current) clearTimeout(pushTimerRef.current);

      const isLoadEvent = !state.isDirty && prevState.isDirty !== state.isDirty;
      if (isLoadEvent) {
        pushDocumentToServer(clientIdRef.current);
        return;
      }

      pushTimerRef.current = setTimeout(() => {
        pushDocumentToServer(clientIdRef.current);
      }, PUSH_DEBOUNCE_MS);
    });

    // Push selection changes to Nitro (debounced)
    let selectionTimer: ReturnType<typeof setTimeout> | null = null;
    let prevSelectedIds: string[] = [];
    let prevActivePageId: string | null = null;

    const unsubSelection = useCanvasStore.subscribe((state) => {
      const { selectedIds } = state.selection;
      const { activePageId } = state;
      // Skip if nothing changed
      if (selectedIds === prevSelectedIds && activePageId === prevActivePageId) return;
      prevSelectedIds = selectedIds;
      prevActivePageId = activePageId;

      if (selectionTimer) clearTimeout(selectionTimer);
      selectionTimer = setTimeout(() => {
        fetch(`${baseUrl}/api/mcp/selection`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            selectedIds,
            activePageId,
            sourceClientId: clientIdRef.current,
          }),
        }).catch(() => {});
      }, SELECTION_DEBOUNCE_MS);
    });

    return () => {
      disposed = true;
      eventSource?.close();
      if (reconnectTimer) clearTimeout(reconnectTimer);
      if (pushTimerRef.current) clearTimeout(pushTimerRef.current);
      if (selectionTimer) clearTimeout(selectionTimer);
      unsubDoc();
      unsubSelection();
      if (typeof window !== 'undefined') {
        window.removeEventListener('focus', sendActivePing);
        document.removeEventListener('visibilitychange', sendActivePing);
      }
    };
  }, []);
}
