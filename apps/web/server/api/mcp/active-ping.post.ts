import { defineEventHandler, readBody } from 'h3';
import { markClientActive } from '../../utils/mcp-sync-state';

export default defineEventHandler(async (event) => {
  const body = (await readBody(event)) as { clientId?: string };
  if (body.clientId) {
    markClientActive(body.clientId);
  }
  return { ok: true };
});
