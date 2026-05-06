import { spawn } from 'node:child_process';
import { Readable, Writable } from 'node:stream';
import { ClientSideConnection, ndJsonStream, PROTOCOL_VERSION } from '@agentclientprotocol/sdk';
import type { AcpAgentConfig, AcpConnectionState } from './types';

/** Establish 与本地进程或远程端点的 ACP 连接。 */
export async function connectAcpAgent(config: AcpAgentConfig): Promise<AcpConnectionState> {
  if (config.connectionType === 'local') {
    return connectLocal(config);
  }
  return connectRemote(config);
}

/**
 * 来自双向 ndJSON
 * 流的 Create 和 ClientSideConnection。 The Client.sessionUpdate
 * 回调将事件分派到 state.sessionUpdateEmitter，以便 SSE 处理程序可以使用它们。
 */
function createConnection(
  stream: ReturnType<typeof ndJsonStream>,
  state: AcpConnectionState,
): ClientSideConnection {
  return new ClientSideConnection(
    (_agent) => ({
      sessionUpdate: async (params) => {
        state.sessionUpdateEmitter?.dispatchEvent(new CustomEvent('update', { detail: params }));
      },
      // Auto-批准所有工具调用。 The 用户已通过在设置中连接此 ACP 代理建立了信任。 Claude Agent ACP 在每次
      // MCP 工具调用之前请求权限 - 如果我们不批准，工具会失败并显示“Tool use aborted”。
      // Future：如果破坏性操作需要每次调用批准，则通过 AgentToolExecutor 的 TOOL_AUTH_MAP
      // 进行路由。
      requestPermission: async (params) => {
        // Prefer 第一个允许选项（如果存在）；回退到通用允许。
        const allowOption = params.options?.find(
          (o) =>
            o.kind === 'allow_once' || o.kind === 'allow_always' || o.optionId.startsWith('allow'),
        );
        return {
          outcome: {
            outcome: 'selected' as const,
            optionId: allowOption?.optionId ?? params.options?.[0]?.optionId ?? 'allow',
          },
        };
      },
    }),
    stream,
  );
}

async function connectLocal(config: AcpAgentConfig): Promise<AcpConnectionState> {
  if (!config.command) throw new Error('Local ACP agent requires a command');

  const proc = spawn(config.command, config.args ?? [], {
    stdio: ['pipe', 'pipe', 'pipe'],
    env: { ...process.env, ...config.env },
  });

  // 节点：流 toWeb 返回 ReadableStream<any>； ndJsonStream 需要
  // ReadableStream<Uint8Array>。 The 运行时数据是字节，因此转换是安全的 - 只是 TypeScript
  // 的方差在这里太严格。
  const input = Writable.toWeb(proc.stdin!) as WritableStream<Uint8Array>;
  const output = Readable.toWeb(proc.stdout!) as ReadableStream<Uint8Array>;
  const stream = ndJsonStream(input, output);

  const state: AcpConnectionState = {
    connection: null!,
    agentInfo: { name: 'unknown' },
    process: proc,
    sessionUpdateEmitter: null,
  };
  state.connection = createConnection(stream, state);

  const initResult = await state.connection.initialize({
    protocolVersion: PROTOCOL_VERSION,
    clientCapabilities: {},
    clientInfo: { name: 'openpencil', version: '0.7.1' },
  });

  state.agentInfo = {
    name: initResult.agentInfo?.name ?? config.displayName,
    title: initResult.agentInfo?.title ?? undefined,
    version: initResult.agentInfo?.version ?? undefined,
  };

  return state;
}

async function connectRemote(config: AcpAgentConfig): Promise<AcpConnectionState> {
  if (!config.url) throw new Error('Remote ACP agent requires a URL');

  const { WebSocket: WS } = await import('ws');
  const ws = new WS(config.url);
  await new Promise<void>((resolve, reject) => {
    ws.addEventListener('open', () => resolve());
    ws.addEventListener('error', (e) => reject(new Error(`WebSocket error: ${e}`)));
  });

  const readable = new ReadableStream<Uint8Array>({
    start(controller) {
      ws.addEventListener('message', (e) => {
        const data = typeof e.data === 'string' ? e.data : String(e.data);
        controller.enqueue(new TextEncoder().encode(data + '\n'));
      });
      ws.addEventListener('close', () => controller.close());
    },
  });
  const writable = new WritableStream<Uint8Array>({
    write(chunk) {
      ws.send(new TextDecoder().decode(chunk));
    },
  });

  const stream = ndJsonStream(writable, readable);

  const state: AcpConnectionState = {
    connection: null!,
    agentInfo: { name: 'unknown' },
    sessionUpdateEmitter: null,
  };
  state.connection = createConnection(stream, state);

  const initResult = await state.connection.initialize({
    protocolVersion: PROTOCOL_VERSION,
    clientCapabilities: {},
    clientInfo: { name: 'openpencil', version: '0.7.1' },
  });

  state.agentInfo = {
    name: initResult.agentInfo?.name ?? config.displayName,
    title: initResult.agentInfo?.title ?? undefined,
    version: initResult.agentInfo?.version ?? undefined,
  };

  return state;
}

/** Disconnect an ACP connection and kill the process if local. */
export function disconnectAcpAgent(state: AcpConnectionState): void {
  if (state.process) {
    state.process.kill('SIGTERM');
  }
}
