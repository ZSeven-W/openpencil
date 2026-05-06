import type {
  QueryEngineHandle,
  IteratorHandle,
  ProviderHandle,
  ToolRegistryHandle,
  TeamHandle,
} from '@zseven-w/agent-native';
import type { ClientSideConnection } from '@agentclientprotocol/sdk';
import type { LayoutPhase } from './agent-tool-guard';
import {
  abortEngine,
  destroyIterator,
  destroyQueryEngine,
  destroyToolRegistry,
  destroyProvider,
  abortTeam,
  destroyTeam,
} from '@zseven-w/agent-native';

export interface NativeAgentSession {
  type: 'native';
  engine?: QueryEngineHandle;
  team?: TeamHandle;
  iter?: IteratorHandle;
  provider: ProviderHandle;
  tools?: ToolRegistryHandle;
  memberHandles?: Array<{ provider: ProviderHandle; tools: ToolRegistryHandle }>;
  createdAt: number;
  lastActivity: number;
  /** toolCallId → memberId — 将异步工具结果路由到正确的成员引擎。 */
  toolOwners: Map<string, string>;
  /** toolCallId → 工具名称 — 用于会话级工具防护和状态更新。 */
  toolNames: Map<string, string>;
  /** memberId → 角色 — 用于委派时间技能解析。 */
  memberRoles: Map<string, string>;
  /** Session-内置单代理护栏的本地布局进度。 */
  layoutPhase: LayoutPhase;
  layoutRootId: string | null;
}

export interface AcpAgentSession {
  type: 'acp';
  acpSessionId: string;
  acpAgentId: string;
  connection: ClientSideConnection;
  createdAt: number;
  lastActivity: number;
  toolNames: Map<string, string>;
  toolOwners: Map<string, string>;
  layoutPhase: LayoutPhase;
  layoutRootId: string | null;
}

export type AgentSession = NativeAgentSession | AcpAgentSession;

/** Create 具有所需默认值的本机会话。 */
export function createSession(
  fields: Omit<
    NativeAgentSession,
    'type' | 'toolOwners' | 'toolNames' | 'memberRoles' | 'layoutPhase' | 'layoutRootId'
  > &
    Partial<
      Pick<
        NativeAgentSession,
        'toolOwners' | 'toolNames' | 'memberRoles' | 'layoutPhase' | 'layoutRootId'
      >
    >,
): NativeAgentSession {
  return {
    type: 'native',
    ...fields,
    toolOwners: fields.toolOwners ?? new Map(),
    toolNames: fields.toolNames ?? new Map(),
    memberRoles: fields.memberRoles ?? new Map(),
    layoutPhase: fields.layoutPhase ?? 'idle',
    layoutRootId: fields.layoutRootId ?? null,
  };
}

/** Create 具有所需默认值的 ACP 会话。 */
export function createAcpSession(fields: {
  acpSessionId: string;
  acpAgentId: string;
  connection: ClientSideConnection;
}): AcpAgentSession {
  return {
    type: 'acp',
    ...fields,
    createdAt: Date.now(),
    lastActivity: Date.now(),
    toolNames: new Map(),
    toolOwners: new Map(),
    layoutPhase: 'idle',
    layoutRootId: null,
  };
}

export const agentSessions = new Map<string, AgentSession>();

/** Mark 会话处于活动状态，因此长时间运行的外部工具回调不会过期。 */
export function touchSession(session: Pick<AgentSession, 'lastActivity'>, now = Date.now()): void {
  session.lastActivity = now;
}

/** Idempotent cleanup — 销毁后使句柄无效以防止双重释放。 */
export function cleanup(session: AgentSession): void {
  if (session.type === 'acp') return; // ACP 连接由 acp-connection-manager 管理
  if (session.iter) {
    destroyIterator(session.iter);
    session.iter = undefined;
  }
  if (session.team) {
    abortTeam(session.team);
    destroyTeam(session.team);
    session.team = undefined;
  }
  if (session.engine) {
    destroyQueryEngine(session.engine);
    session.engine = undefined;
  }
  if (session.memberHandles) {
    for (const mh of session.memberHandles) {
      destroyToolRegistry(mh.tools);
      destroyProvider(mh.provider);
    }
    session.memberHandles = undefined;
  }
  if (session.tools) {
    destroyToolRegistry(session.tools);
    session.tools = undefined;
  }
  if (session.provider) {
    destroyProvider(session.provider);
    (session as any).provider = undefined;
  }
}

/** Abort 会话 — 使挂起的 nextEvent 解析为 null。 */
export function abortSession(session: AgentSession): void {
  if (session.type === 'acp') {
    try {
      (session.connection as any).cancel?.({ sessionId: session.acpSessionId });
    } catch {}
    return;
  }
  if (session.team) abortTeam(session.team);
  else if (session.engine) abortEngine(session.engine);
}

// 每 60 秒 Cleanup 陈旧会话（距上次活动 5 分钟的 TTL）
setInterval(() => {
  try {
    const now = Date.now();
    for (const [id, session] of agentSessions) {
      if (now - session.lastActivity > 5 * 60_000) {
        abortSession(session);
        cleanup(session);
        agentSessions.delete(id);
      }
    }
  } catch {
    /* 忽略清理错误 */
  }
}, 60_000);
