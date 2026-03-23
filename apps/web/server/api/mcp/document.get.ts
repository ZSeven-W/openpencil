import { defineEventHandler, createError } from 'h3'
import { getSyncDocument } from '../../utils/mcp-sync-state'

/** GET /api/mcp/document — Returns the current canvas document for MCP to read. */
export default defineEventHandler(() => {
  const { doc, version } = getSyncDocument()
  if (!doc) {
    throw createError({ statusCode: 404, statusMessage: 'No document loaded in editor' })
  }
  return { version, document: doc }
})
