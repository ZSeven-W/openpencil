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

  // 1. Resolve 目标渲染器 — 如果没有则快速失败
  const targetClientId = getLastActiveClientId();
  if (!targetClientId || !isClientConnected(targetClientId)) {
    throw createError({
      statusCode: 503,
      statusMessage:
        'No active editor client — make sure an Electron window or /editor tab is open and focused.',
    });
  }

  // 2. Allocate 请求 id 并尝试发送
  const requestId = allocateRequestId();
  const sent = sendToClient(targetClientId, {
    type: 'screenshot:request',
    requestId,
    bounds: body.bounds,
    nodeId: body.nodeId,
    opts: body.opts,
    timeoutMs,
  });

  // 3. Only 注册挂起+启动超时 AFTER 成功发送（Q3 判定）
  if (!sent) {
    throw createError({
      statusCode: 503,
      statusMessage:
        'Failed to deliver screenshot request — target editor client disconnected between check and send.',
    });
  }

  // 4. Await 渲染器响应（或超时）
  return await registerPending(requestId, timeoutMs);
});
