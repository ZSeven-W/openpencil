import { defineEventHandler, readBody, createError } from 'h3'
import { setSyncSelection } from '../../utils/mcp-sync-state'

interface PostBody {
  selectedIds: string[]
  activePageId?: string | null
}

/** POST /api/mcp/selection — Receives selection update from renderer. */
export default defineEventHandler(async (event) => {
  const body = await readBody<PostBody>(event)
  if (!body || !Array.isArray(body.selectedIds)) {
    throw createError({ statusCode: 400, statusMessage: 'Missing selectedIds array' })
  }
  setSyncSelection(body.selectedIds, body.activePageId)
  return { ok: true }
})
