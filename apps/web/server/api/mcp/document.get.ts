import { defineEventHandler, setResponseHeaders } from 'h3'
import { getSyncDocument } from '../../utils/mcp-sync-state'

/** GET /api/mcp/document — Returns the current canvas document for MCP to read. */
export default defineEventHandler((event) => {
  setResponseHeaders(event, { 'Content-Type': 'application/json' })
  const { doc, version } = getSyncDocument()
  if (!doc) {
    return new Response(JSON.stringify({ error: 'No document loaded in editor' }), {
      status: 404,
      headers: { 'Content-Type': 'application/json' },
    })
  }
  return { version, document: doc }
})
