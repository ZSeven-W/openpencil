// apps/web/server/api/mcp/screenshot-response.post.ts
import { defineEventHandler, readBody } from 'h3';
import { resolvePending, type ScreenshotResponse } from '../../utils/mcp-screenshot-rpc';

export default defineEventHandler(async (event) => {
  const body = (await readBody(event)) as ScreenshotResponse;
  const accepted = resolvePending(body);
  if (!accepted) {
    return { received: false, reason: 'no pending request (timed out or duplicate)' };
  }
  return { received: true };
});
