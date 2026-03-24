import { defineEventHandler, readBody, createError } from 'h3'
import { setSyncDocument } from '../../utils/mcp-sync-state'
import type { PenDocument } from '../../../src/types/pen'

interface PostBody {
  document: PenDocument
  sourceClientId?: string
}

/** POST /api/mcp/document — Receives document update from MCP or renderer, triggers SSE broadcast. */
export default defineEventHandler(async (event) => {
  const body = await readBody<PostBody>(event)
  if (!body?.document) {
    throw createError({ statusCode: 400, statusMessage: 'Missing document in request body' })
  }
  const doc = body.document
  if (!doc.version || (!Array.isArray(doc.children) && !Array.isArray(doc.pages))) {
    throw createError({ statusCode: 400, statusMessage: 'Invalid document format' })
  }
  const version = setSyncDocument(doc, body.sourceClientId)
  return { ok: true, version }
})
