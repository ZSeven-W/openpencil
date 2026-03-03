import { defineEventHandler } from 'h3'
import { randomUUID } from 'node:crypto'
import type { ServerResponse } from 'node:http'
import { registerSSEClient, unregisterSSEClient, getSyncDocument } from '../../utils/mcp-sync-state'

/** GET /api/mcp/events — SSE stream for renderer to subscribe to live document changes. */
export default defineEventHandler((event) => {
  const clientId = randomUUID()

  // Write headers directly on the raw Node.js response for h3 v2 compatibility.
  // h3 v2 no longer supports `_handled = true` to keep connections open.
  const res = event.node!.res! as ServerResponse
  res.writeHead(200, {
    'Content-Type': 'text/event-stream',
    'Cache-Control': 'no-cache',
    'Connection': 'keep-alive',
  })

  // Send client ID so renderer can use it as sourceClientId when pushing back
  res.write(`data: ${JSON.stringify({ type: 'client:id', clientId })}\n\n`)

  // Send current document as initial state (if any)
  const { doc, version } = getSyncDocument()
  if (doc) {
    res.write(`data: ${JSON.stringify({ type: 'document:init', version, document: doc })}\n\n`)
  }

  registerSSEClient(clientId, res)

  // Keep-alive heartbeat
  const heartbeat = setInterval(() => {
    if (!res.closed) res.write(': heartbeat\n\n')
  }, 30_000)

  res.on('close', () => {
    clearInterval(heartbeat)
    unregisterSSEClient(clientId)
  })

  // Return a promise that resolves on close to prevent h3 from ending the response
  return new Promise<void>((resolve) => {
    res.on('close', resolve)
  })
})
