import { defineEventHandler } from 'h3'
import { randomUUID } from 'node:crypto'
import { registerSSEClient, unregisterSSEClient, getSyncDocument } from '../../utils/mcp-sync-state'

/** GET /api/mcp/events — SSE stream for renderer to subscribe to live document changes. */
export default defineEventHandler((_event) => {
  const clientId = randomUUID()

  const { readable, writable } = new TransformStream()
  const writer = writable.getWriter()
  const encoder = new TextEncoder()

  let closed = false
  const cleanup = () => {
    if (closed) return
    closed = true
    clearInterval(heartbeat)
    unregisterSSEClient(clientId)
    writer.close().catch(() => {})
  }

  const write = (data: string) => {
    if (closed) return
    writer.write(encoder.encode(`data: ${data}\n\n`)).catch(cleanup)
  }

  // Send client ID so renderer can use it as sourceClientId when pushing back
  write(JSON.stringify({ type: 'client:id', clientId }))

  // Send current document as initial state (if any)
  const { doc, version } = getSyncDocument()
  if (doc) {
    write(JSON.stringify({ type: 'document:init', version, document: doc }))
  }

  registerSSEClient(clientId, { push: write })

  // Keep-alive heartbeat — also serves as connection health check
  const heartbeat = setInterval(() => {
    if (closed) return
    writer.write(encoder.encode(': heartbeat\n\n')).catch(cleanup)
  }, 30_000)

  // Return a proper Response object so Vite/Bun proxy can handle it
  return new Response(readable, {
    headers: {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
      'Connection': 'keep-alive',
    },
  })
})
