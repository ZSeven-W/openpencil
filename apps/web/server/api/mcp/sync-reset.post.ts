import { defineEventHandler } from 'h3'
import { clearSyncState } from '../../utils/mcp-sync-state'

/** POST /api/mcp/sync-reset — Clears stale sync cache on page load / file open. */
export default defineEventHandler(() => {
  clearSyncState()
  return { ok: true }
})
