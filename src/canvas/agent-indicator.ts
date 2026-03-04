// ---------------------------------------------------------------------------
// Agent indicator state — tracks which nodes have active agent overlays.
//
// Uses globalThis to guarantee a single shared instance across all module
// chunks — eliminates Vite module-splitting isolation issues.
// ---------------------------------------------------------------------------

export interface AgentIndicatorEntry {
  nodeId: string
  color: string
  name: string
}

const INDICATORS_KEY = '__openpencil_agent_indicators__'
const PREVIEWS_KEY = '__openpencil_agent_previews__'

function getIndicatorMap(): Map<string, AgentIndicatorEntry> {
  const g = globalThis as Record<string, unknown>
  if (!g[INDICATORS_KEY]) {
    g[INDICATORS_KEY] = new Map<string, AgentIndicatorEntry>()
  }
  return g[INDICATORS_KEY] as Map<string, AgentIndicatorEntry>
}

function getPreviewSet(): Set<string> {
  const g = globalThis as Record<string, unknown>
  if (!g[PREVIEWS_KEY]) {
    g[PREVIEWS_KEY] = new Set<string>()
  }
  return g[PREVIEWS_KEY] as Set<string>
}

export function getActiveAgentIndicators(): Map<string, AgentIndicatorEntry> {
  return getIndicatorMap()
}

export function addAgentIndicator(nodeId: string, color: string, name: string): void {
  getIndicatorMap().set(nodeId, { nodeId, color, name })
}

export function removeAgentIndicator(nodeId: string): void {
  getIndicatorMap().delete(nodeId)
  getPreviewSet().delete(nodeId)
}

export function addPreviewNode(nodeId: string): void {
  getPreviewSet().add(nodeId)
}

export function removePreviewNode(nodeId: string): void {
  getPreviewSet().delete(nodeId)
}

export function isPreviewNode(nodeId: string): boolean {
  return getPreviewSet().has(nodeId)
}

/** Remove all indicators whose nodeId starts with the given prefix. */
export function removeAgentIndicatorsByPrefix(prefix: string): void {
  const map = getIndicatorMap()
  const set = getPreviewSet()
  const prefixDash = `${prefix}-`
  for (const key of [...map.keys()]) {
    if (key.startsWith(prefixDash)) {
      map.delete(key)
      set.delete(key)
    }
  }
}

export function clearAgentIndicators(): void {
  getIndicatorMap().clear()
  getPreviewSet().clear()
}
