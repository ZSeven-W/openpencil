// ---------------------------------------------------------------------------
// Agent 指示器状态 — 跟踪哪些节点具有活动代理覆盖。
//
// Uses globalThis 确保所有模块有一个共享实例
// 块 — 消除 Vite 模块分割隔离问题。
// ---------------------------------------------------------------------------

export interface AgentIndicatorEntry {
  nodeId: string;
  color: string;
  name: string;
}

/** Tracks 代理负责哪个根框架（用于徽章放置）。 */
export interface AgentFrameEntry {
  frameId: string;
  color: string;
  name: string;
}

const INDICATORS_KEY = '__openpencil_agent_indicators__';
const PREVIEWS_KEY = '__openpencil_agent_previews__';
const AGENT_FRAMES_KEY = '__openpencil_agent_frames__';

function getIndicatorMap(): Map<string, AgentIndicatorEntry> {
  const g = globalThis as Record<string, unknown>;
  if (!g[INDICATORS_KEY]) {
    g[INDICATORS_KEY] = new Map<string, AgentIndicatorEntry>();
  }
  return g[INDICATORS_KEY] as Map<string, AgentIndicatorEntry>;
}

function getPreviewSet(): Set<string> {
  const g = globalThis as Record<string, unknown>;
  if (!g[PREVIEWS_KEY]) {
    g[PREVIEWS_KEY] = new Set<string>();
  }
  return g[PREVIEWS_KEY] as Set<string>;
}

function getAgentFrameMap(): Map<string, AgentFrameEntry> {
  const g = globalThis as Record<string, unknown>;
  if (!g[AGENT_FRAMES_KEY]) {
    g[AGENT_FRAMES_KEY] = new Map<string, AgentFrameEntry>();
  }
  return g[AGENT_FRAMES_KEY] as Map<string, AgentFrameEntry>;
}

export function getActiveAgentIndicators(): Map<string, AgentIndicatorEntry> {
  return getIndicatorMap();
}

export function getActiveAgentFrames(): Map<string, AgentFrameEntry> {
  return getAgentFrameMap();
}

export function addAgentIndicator(nodeId: string, color: string, name: string): void {
  getIndicatorMap().set(nodeId, { nodeId, color, name });
}

/** Register 一个由代理拥有的框架（对于框架名称旁边的徽章）。 */
export function addAgentFrame(frameId: string, color: string, name: string): void {
  getAgentFrameMap().set(frameId, { frameId, color, name });
}

export function removeAgentIndicator(nodeId: string): void {
  getIndicatorMap().delete(nodeId);
  getPreviewSet().delete(nodeId);
}

export function addPreviewNode(nodeId: string): void {
  getPreviewSet().add(nodeId);
}

export function removePreviewNode(nodeId: string): void {
  getPreviewSet().delete(nodeId);
}

export function isPreviewNode(nodeId: string): boolean {
  return getPreviewSet().has(nodeId);
}

/** Remove 以给定前缀开头的所有节点指示器。 Agent 框架徽章已被删除 - 它们独立存在。
 *  */
export function removeAgentIndicatorsByPrefix(prefix: string): void {
  const map = getIndicatorMap();
  const set = getPreviewSet();
  const prefixDash = `${prefix}-`;
  for (const key of Array.from(map.keys())) {
    if (key.startsWith(prefixDash)) {
      map.delete(key);
      set.delete(key);
    }
  }
}

/**
 * Recursively add agent indicators for a node and all its descendants.
 * This ensures indicators are visible on the entire subtree, not just the root.
 */
export function addAgentIndicatorRecursive(
  node: { id: string; children?: { id: string; children?: unknown[] }[] },
  color: string,
  name: string,
): void {
  const map = getIndicatorMap();
  const set = getPreviewSet();
  const walk = (n: { id: string; children?: unknown[] }) => {
    map.set(n.id, { nodeId: n.id, color, name });
    set.add(n.id);
    if (Array.isArray(n.children)) {
      for (const child of n.children) {
        walk(child as { id: string; children?: unknown[] });
      }
    }
  };
  walk(node);
}

/** Clear node indicators immediately; agent frame badges fade out after a delay. */
export function clearAgentIndicators(): void {
  getIndicatorMap().clear();
  getPreviewSet().clear();
  // Agent frame badges linger briefly so the user sees which agent built what
  setTimeout(() => getAgentFrameMap().clear(), 2000);
}
