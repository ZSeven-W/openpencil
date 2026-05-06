import { defineEventHandler, createEventStream } from 'h3';
import { randomUUID } from 'node:crypto';
import {
  registerSSEClient,
  unregisterSSEClient,
  getSyncDocument,
} from '../../utils/mcp-sync-state';

// Bun.serve 的默认 idleTimeout 为 10 秒。 Heartbeat 必须更短，以防止 SSE 连接被终止。
const HEARTBEAT_MS = 8_000;

/** GET /api/mcp/events — SSE 流，供渲染器订阅实时文档更改。 */
export default defineEventHandler((event) => {
  const clientId = randomUUID();
  const stream = createEventStream(event);

  let closed = false;
  const cleanup = () => {
    if (closed) return;
    closed = true;
    clearInterval(heartbeat);
    unregisterSSEClient(clientId);
    stream.close();
  };

  const write = (data: string) => {
    if (closed) return;
    stream.push(data).catch(cleanup);
  };

  // Send 客户端 ID 因此渲染器在推回时可以将其用作 sourceClientId
  write(JSON.stringify({ type: 'client:id', clientId }));

  // Send 当前文档作为初始状态（如果有）
  const { doc, version } = getSyncDocument();
  if (doc) {
    write(JSON.stringify({ type: 'document:init', version, document: doc }));
  }

  registerSSEClient(clientId, { push: write });

  // Keep-alive heartbeat — 必须短于 Bun 的空闲超时（10 秒）
  const heartbeat = setInterval(() => {
    if (closed) return;
    stream.push(':heartbeat').catch(cleanup);
  }, HEARTBEAT_MS);

  // 当客户端断开连接时 Clean 启动
  stream.onClosed(cleanup);

  return stream.send();
});
