import type {
  QueryEngineHandle,
  IteratorHandle,
  ProviderHandle,
  ToolRegistryHandle,
} from '@zseven-w/agent-native';
import {
  abortEngine,
  destroyIterator,
  destroyQueryEngine,
  destroyToolRegistry,
  destroyProvider,
} from '@zseven-w/agent-native';

export interface AgentSession {
  engine?: QueryEngineHandle;
  iter?: IteratorHandle;
  provider: ProviderHandle;
  tools?: ToolRegistryHandle;
  createdAt: number;
  lastActivity: number;
}

export const agentSessions = new Map<string, AgentSession>();

/** Idempotent cleanup — nullifies handles after destroying to prevent double-free. */
export function cleanup(session: AgentSession): void {
  if (session.iter) {
    destroyIterator(session.iter);
    session.iter = undefined;
  }
  if (session.engine) {
    destroyQueryEngine(session.engine);
    session.engine = undefined;
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
  if (session.engine) abortEngine(session.engine);
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
