import type { ClientSideConnection } from '@agentclientprotocol/sdk';

/** Persisted config for a user-configured ACP agent. */
export interface AcpAgentConfig {
  id: string;
  displayName: string;
  connectionType: 'local' | 'remote';
  command?: string;
  args?: string[];
  env?: Record<string, string>;
  url?: string;
  enabled: boolean;
}

/** Info returned by the ACP agent during initialize handshake. */
export interface AcpAgentInfo {
  name: string;
  title?: string;
  version?: string;
}

/** Result of a connect attempt. */
export interface AcpConnectResult {
  connected: boolean;
  agentInfo?: AcpAgentInfo;
  error?: string;
}

/** Live connection state held by the connection manager. */
export interface AcpConnectionState {
  connection: ClientSideConnection;
  agentInfo: AcpAgentInfo;
  process?: import('node:child_process').ChildProcess;
  /**
   * Session-scoped event emitter for session/update notifications.
   * Set by the prompt handler before calling connection.prompt().
   * The Client.sessionUpdate callback pushes events here;
   * the SSE stream handler consumes them.
   */
  sessionUpdateEmitter: EventTarget | null;
}
