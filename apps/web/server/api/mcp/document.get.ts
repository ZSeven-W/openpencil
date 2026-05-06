import { defineEventHandler, createError } from 'h3';
import { getSyncDocument } from '../../utils/mcp-sync-state';

/** GET /api/mcp/document — Returns 供 MCP 读取的当前画布文档。 */
export default defineEventHandler(() => {
  const { doc, version } = getSyncDocument();
  if (!doc) {
    throw createError({ statusCode: 404, statusMessage: 'No document loaded in editor' });
  }
  return { version, document: doc };
});
