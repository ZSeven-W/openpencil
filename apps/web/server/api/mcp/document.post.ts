import { defineEventHandler, readBody, setResponseHeaders } from 'h3'
import { setSyncDocument } from '../../utils/mcp-sync-state'
import type { PenDocument } from '../../../src/types/pen'

interface PostBody {
  document: PenDocument
  sourceClientId?: string
}

/** POST /api/mcp/document — Receives document update from MCP or renderer, triggers SSE broadcast. */
export default defineEventHandler(async (event) => {
  setResponseHeaders(event, { 'Content-Type': 'application/json' })
  const body = await readBody<PostBody>(event)
  if (!body?.document) {
    return new Response(JSON.stringify({ error: 'Missing document in request body' }), {
      status: 400,
      headers: { 'Content-Type': 'application/json' },
    })
  }
  const doc = body.document
  if (!doc.version || (!Array.isArray(doc.children) && !Array.isArray(doc.pages))) {
    return new Response(JSON.stringify({ error: 'Invalid document format' }), {
      status: 400,
      headers: { 'Content-Type': 'application/json' },
    })
  }
  const version = setSyncDocument(doc, body.sourceClientId)
  return { ok: true, version }
})
