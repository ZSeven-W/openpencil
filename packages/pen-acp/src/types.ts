import type { ClientSideConnection } from '@agentclientprotocol/sdk';

/** Persisted 用户配置的 ACP 代理的配置。 */
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

/** Info 初始化握手期间由 ACP 代理返回。 */
export interface AcpAgentInfo {
  name: string;
  title?: string;
  version?: string;
}

/** Result 连接尝试。 */
export interface AcpConnectResult {
  connected: boolean;
  agentInfo?: AcpAgentInfo;
  error?: string;
}

/** Live 连接管理器保存的连接状态。 */
export interface AcpConnectionState {
  connection: ClientSideConnection;
  agentInfo: AcpAgentInfo;
  process?: import('node:child_process').ChildProcess;
  /**
   * 用于 session/update 通知的 Session 范围事件发射器。
   * Set 在调用 connection.prompt() 之前由提示处理程序执行。
   * The Client.sessionUpdate callback pushes events here;
   * SSE 流处理程序使用它们。
   */
  sessionUpdateEmitter: EventTarget | null;
}
