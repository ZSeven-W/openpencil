import { defineEventHandler, createEventStream } from 'h3'
import { randomUUID } from 'node:crypto'
import { registerSSEClient, unregisterSSEClient, getSyncDocument } from '../../utils/mcp-sync-state'

// Bun.serve has a default idleTimeout of 10s. Heartbeat must be shorter
// to prevent the SSE connection from being killed.
const HEARTBEAT_MS = 8_000

/** GET /api/mcp/events — SSE stream for renderer to subscribe to live document changes. */
export default defineEventHandler((event) => {
  const clientId = randomUUID()
  const stream = createEventStream(event)

  let closed = false
  const cleanup = () => {
    if (closed) return
    closed = true
    clearInterval(heartbeat)
    unregisterSSEClient(clientId)
    stream.close()
  }

  const write = (data: string) => {
    if (closed) return
    stream.push(data).catch(cleanup)
  }

  // Send client ID so renderer can use it as sourceClientId when pushing back
  write(JSON.stringify({ type: 'client:id', clientId }))

  // Send current document as initial state (if any)
  const { doc, version } = getSyncDocument()
  if (doc) {
    write(JSON.stringify({ type: 'document:init', version, document: doc }))
  }

  registerSSEClient(clientId, { push: write })

  // Keep-alive heartbeat — must be shorter than Bun's idle timeout (10s)
  const heartbeat = setInterval(() => {
    if (closed) return
    stream.push(':heartbeat').catch(cleanup)
  }, HEARTBEAT_MS)

  // Clean up when client disconnects
  stream.onClosed(cleanup)

  return stream.send()
})
