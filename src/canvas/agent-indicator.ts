// ---------------------------------------------------------------------------
// Agent indicator state — tracks which nodes have active agent overlays.
// Module-level mutable state, same pattern as insertion-indicator.ts.
// ---------------------------------------------------------------------------

export interface AgentIndicatorEntry {
  nodeId: string
  color: string
  name: string
}

/** Active agent indicators, keyed by nodeId. */
const activeAgentIndicators = new Map<string, AgentIndicatorEntry>()

/** Nodes in "preview" phase — outline shown, waiting to materialize. */
const previewNodes = new Set<string>()

/** Getter for cross-module access — more robust than exporting the let binding. */
export function getActiveAgentIndicators(): Map<string, AgentIndicatorEntry> {
  return activeAgentIndicators
}

export function addAgentIndicator(nodeId: string, color: string, name: string): void {
  activeAgentIndicators.set(nodeId, { nodeId, color, name })
}

export function removeAgentIndicator(nodeId: string): void {
  activeAgentIndicators.delete(nodeId)
  previewNodes.delete(nodeId)
}

/** Mark a node as in preview phase (outline visible, content hidden). */
export function addPreviewNode(nodeId: string): void {
  previewNodes.add(nodeId)
}

export function removePreviewNode(nodeId: string): void {
  previewNodes.delete(nodeId)
}

export function isPreviewNode(nodeId: string): boolean {
  return previewNodes.has(nodeId)
}

/** Remove all indicators whose nodeId starts with the given prefix. */
export function removeAgentIndicatorsByPrefix(prefix: string): void {
  const prefixDash = `${prefix}-`
  for (const key of [...activeAgentIndicators.keys()]) {
    if (key.startsWith(prefixDash)) {
      activeAgentIndicators.delete(key)
      previewNodes.delete(key)
    }
  }
}

export function clearAgentIndicators(): void {
  activeAgentIndicators.clear()
  previewNodes.clear()
}
