import { defineEventHandler, readBody, setResponseHeaders } from 'h3'
import { setSyncSelection } from '../../utils/mcp-sync-state'

interface PostBody {
  selectedIds: string[]
  activePageId?: string | null
}

/** POST /api/mcp/selection — Receives selection update from renderer. */
export default defineEventHandler(async (event) => {
  setResponseHeaders(event, { 'Content-Type': 'application/json' })
  const body = await readBody<PostBody>(event)
  if (!body || !Array.isArray(body.selectedIds)) {
    return new Response(JSON.stringify({ error: 'Missing selectedIds array' }), {
      status: 400,
      headers: { 'Content-Type': 'application/json' },
    })
  }
  setSyncSelection(body.selectedIds, body.activePageId)
  return { ok: true }
})
