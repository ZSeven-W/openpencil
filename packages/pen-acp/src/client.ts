import { spawn } from 'node:child_process';
import { Readable, Writable } from 'node:stream';
import { ClientSideConnection, ndJsonStream, PROTOCOL_VERSION } from '@agentclientprotocol/sdk';
import type { AcpAgentConfig, AcpConnectionState } from './types';

/** Establish an ACP connection to a local process or remote endpoint. */
export async function connectAcpAgent(config: AcpAgentConfig): Promise<AcpConnectionState> {
  if (config.connectionType === 'local') {
    return connectLocal(config);
  }
  return connectRemote(config);
}

/**
 * Create a ClientSideConnection from a bidirectional ndJSON stream.
 * The Client.sessionUpdate callback dispatches events to
 * state.sessionUpdateEmitter so the SSE handler can consume them.
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
      // Auto-approve all tool calls. The user already established trust by
      // connecting this ACP agent in settings. Claude Agent ACP requests
      // permission before each MCP tool call — if we don't approve, tools
      // fail with "Tool use aborted".
      // Future: route through AgentToolExecutor's TOOL_AUTH_MAP if per-call
      // approval is needed for destructive operations.
      requestPermission: async (params) => {
        // Prefer the first allow option if present; fall back to generic allow.
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

  // node:stream toWeb returns ReadableStream<any>; ndJsonStream expects
  // ReadableStream<Uint8Array>. The runtime data is bytes, so the cast is
  // safe — only TypeScript's variance is too strict here.
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
