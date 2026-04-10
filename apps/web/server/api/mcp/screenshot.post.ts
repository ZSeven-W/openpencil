// apps/web/server/api/mcp/screenshot.post.ts
import { defineEventHandler, readBody, createError } from 'h3';
import { sendToClient, getLastActiveClientId, isClientConnected } from '../../utils/mcp-sync-state';
import {
  allocateRequestId,
  registerPending,
  type ScreenshotRequestBody,
} from '../../utils/mcp-screenshot-rpc';

export default defineEventHandler(async (event) => {
  const body = (await readBody(event)) as ScreenshotRequestBody;
  const timeoutMs = Math.min(body.timeoutMs ?? 15000, 60000);

  // 1. Resolve target renderer — fail fast if none
  const targetClientId = getLastActiveClientId();
  if (!targetClientId || !isClientConnected(targetClientId)) {
    throw createError({
      statusCode: 503,
      statusMessage:
        'No active editor client — make sure an Electron window or /editor tab is open and focused.',
    });
  }

  // 2. Allocate request id and try to send
  const requestId = allocateRequestId();
  const sent = sendToClient(targetClientId, {
    type: 'screenshot:request',
    requestId,
    bounds: body.bounds,
    nodeId: body.nodeId,
    opts: body.opts,
    timeoutMs,
  });

  // 3. Only register pending + start timeout AFTER successful send (Q3 decision)
  if (!sent) {
    throw createError({
      statusCode: 503,
      statusMessage:
        'Failed to deliver screenshot request — target editor client disconnected between check and send.',
    });
  }

  // 4. Await renderer response (or timeout)
  return await registerPending(requestId, timeoutMs);
});
