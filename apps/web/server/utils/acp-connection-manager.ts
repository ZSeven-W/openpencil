import { connectAcpAgent, disconnectAcpAgent } from '@zseven-w/pen-acp';
import type { AcpConnectionState, AcpConnectResult } from '@zseven-w/pen-acp';
import type { AcpAgentConfig } from '../../src/types/agent-settings';

// Use globalThis 因此连接在 Vite HMR / Nitro 模块重新加载后仍然存在。 Without
// 这个，重新评估此模块会擦除 Map，并且现有的 UI 会话会收到“ACP 代理未连接”错误，直到重新连接。
const globalStore = globalThis as unknown as {
  __acpConnections?: Map<string, AcpConnectionState>;
};
const connections: Map<string, AcpConnectionState> =
  globalStore.__acpConnections ?? (globalStore.__acpConnections = new Map());

export function getAcpConnection(agentId: string): AcpConnectionState | undefined {
  const conn = connections.get(agentId);
  console.log(
    `[acp] getAcpConnection(${agentId}) → ${conn ? 'found' : 'MISSING'}, total connections: ${connections.size}, keys: [${Array.from(connections.keys()).join(', ')}]`,
  );
  return conn;
}

export async function connectAcp(
  agentId: string,
  config: AcpAgentConfig,
): Promise<AcpConnectResult> {
  // Server-side safety: reject local mode in hosted production web deployments
  // where spawning arbitrary processes is a security risk. Allow it when:
  //   - Running under Electron (process.versions.electron set)
  //   - Running in dev mode (NODE_ENV !== 'production')
  //   - OPENPENCIL_ALLOW_LOCAL_ACP=1 (explicit opt-in for self-hosted non-Electron)
  // Note: Nitro server runs in a Vite worker in dev, so process.versions.electron
  // is undefined even during electron:dev — hence the NODE_ENV check.
  const isElectron = !!process.versions.electron;
  const isDev = process.env.NODE_ENV !== 'production';
  const isAllowed = process.env.OPENPENCIL_ALLOW_LOCAL_ACP === '1';
  if (config.connectionType === 'local' && !isElectron && !isDev && !isAllowed) {
    return {
      connected: false,
      error:
        'Local agents are only available in the desktop app. Set OPENPENCIL_ALLOW_LOCAL_ACP=1 to enable in self-hosted deployments.',
    };
  }

  // Disconnect existing if any
  if (connections.has(agentId)) {
    await disconnectAcp(agentId);
  }

  try {
    console.log(
      `[acp] connecting ${agentId} (${config.connectionType}: ${config.command ?? config.url})`,
    );
    const state = await connectAcpAgent(config);
    connections.set(agentId, state);
    console.log(`[acp] connected ${agentId}, total connections: ${connections.size}`);
    return { connected: true, agentInfo: state.agentInfo };
  } catch (err) {
    console.error(`[acp] connect failed ${agentId}:`, err);
    return {
      connected: false,
      error: err instanceof Error ? err.message : String(err),
    };
  }
}

export async function disconnectAcp(agentId: string): Promise<void> {
  const state = connections.get(agentId);
  if (state) {
    disconnectAcpAgent(state);
    connections.delete(agentId);
  }
}

export function cleanupAllAcp(): void {
  for (const [id, state] of connections) {
    disconnectAcpAgent(state);
    connections.delete(id);
  }
}
