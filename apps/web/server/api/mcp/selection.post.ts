import { defineEventHandler, readBody, createError } from 'h3';
import { setSyncSelection } from '../../utils/mcp-sync-state';

interface PostBody {
  selectedIds: string[];
  activePageId?: string | null;
  sourceClientId?: string;
}

/** POST /api/mcp/selection — Receives 从渲染器选择更新。 */
export default defineEventHandler(async (event) => {
  const body = await readBody<PostBody>(event);
  if (!body || !Array.isArray(body.selectedIds)) {
    throw createError({ statusCode: 400, statusMessage: 'Missing selectedIds array' });
  }
  setSyncSelection(body.selectedIds, body.activePageId, body.sourceClientId);
  return { ok: true };
});
