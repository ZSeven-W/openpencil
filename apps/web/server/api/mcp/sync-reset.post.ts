import { defineEventHandler } from 'h3';
import { clearSyncState } from '../../utils/mcp-sync-state';

/** POST /api/mcp/sync-reset — Clears 页面加载/文件打开时的过时同步缓存。 */
export default defineEventHandler(() => {
  clearSyncState();
  return { ok: true };
});
