import type {
  QueryEngineHandle,
  IteratorHandle,
  ProviderHandle,
  ToolRegistryHandle,
  TeamHandle,
} from '@zseven-w/agent-native';
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

export interface AgentSession {
  engine?: QueryEngineHandle;
  team?: TeamHandle;
  iter?: IteratorHandle;
  provider: ProviderHandle;
  tools?: ToolRegistryHandle;
  memberHandles?: Array<{ provider: ProviderHandle; tools: ToolRegistryHandle }>;
  createdAt: number;
  lastActivity: number;
  /** toolCallId → memberId — routes async tool results to the correct member engine. */
  toolOwners: Map<string, string>;
  /** toolCallId → tool name — used for session-level tool guards and state updates. */
  toolNames: Map<string, string>;
  /** memberId → role — used for delegation-time skill resolution. */
  memberRoles: Map<string, string>;
  /** Session-local layout progress for builtin single-agent guardrails. */
  layoutPhase: LayoutPhase;
  layoutRootId: string | null;
}

/** Create a session with required defaults. */
export function createSession(
  fields: Omit<
    AgentSession,
    'toolOwners' | 'toolNames' | 'memberRoles' | 'layoutPhase' | 'layoutRootId'
  > &
    Partial<
      Pick<
        AgentSession,
        'toolOwners' | 'toolNames' | 'memberRoles' | 'layoutPhase' | 'layoutRootId'
      >
    >,
): AgentSession {
  return {
    ...fields,
    toolOwners: fields.toolOwners ?? new Map(),
    toolNames: fields.toolNames ?? new Map(),
    memberRoles: fields.memberRoles ?? new Map(),
    layoutPhase: fields.layoutPhase ?? 'idle',
    layoutRootId: fields.layoutRootId ?? null,
  };
}

export const agentSessions = new Map<string, AgentSession>();

/** Mark a session as active so long-running external tool callbacks are not expired. */
export function touchSession(session: Pick<AgentSession, 'lastActivity'>, now = Date.now()): void {
  session.lastActivity = now;
}

/** Idempotent cleanup — nullifies handles after destroying to prevent double-free. */
export function cleanup(session: AgentSession): void {
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

/** Abort a session — makes pending nextEvent resolve null. */
export function abortSession(session: AgentSession): void {
  if (session.team) abortTeam(session.team);
  else if (session.engine) abortEngine(session.engine);
}

// Cleanup stale sessions every 60s (5-minute TTL from last activity)
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
    /* ignore cleanup errors */
  }
}, 60_000);
