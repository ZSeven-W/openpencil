import type { PenNode } from '@/types/pen'

// ---------------------------------------------------------------------------
// Shared coordination set — checked by canvas-sync to trigger fade-in
// ---------------------------------------------------------------------------

/** IDs of nodes that should fade in when their Fabric object is created. */
export const pendingAnimationNodes = new Set<string>()

// ---------------------------------------------------------------------------
// Sequential reveal queue — all nodes follow a single ordered queue.
// Each node: border appears at its scheduled time, content fades in
// BORDER_LEAD ms later. No two nodes reveal simultaneously.
// ---------------------------------------------------------------------------

/** Interval between each node's border reveal (ms). */
const REVEAL_INTERVAL = 300
/** Delay between border appearing and content fade-in (ms). */
const BORDER_LEAD = 400

/** Maps nodeId → absolute timestamp when its border should appear. */
const nodeRevealTime = new Map<string, number>()

/** The next available reveal timestamp (advances as nodes are queued). */
let nextRevealAt = 0

/**
 * Mark all node IDs in the tree for fade-in animation.
 * Assigns sequential reveal timestamps via BFS order (parent → children).
 * New nodes are always scheduled AFTER previously queued nodes,
 * even if they arrive in a later streaming chunk.
 */
export function markNodesForAnimation(nodes: PenNode[]): void {
  // Ensure new nodes start no earlier than now
  const now = Date.now()
  if (nextRevealAt < now) nextRevealAt = now

  // BFS to ensure parent before children, level by level
  const queue: PenNode[] = [...nodes]
  while (queue.length > 0) {
    const node = queue.shift()!
    pendingAnimationNodes.add(node.id)
    nodeRevealTime.set(node.id, nextRevealAt)
    nextRevealAt += REVEAL_INTERVAL
    if ('children' in node && Array.isArray(node.children)) {
      for (const child of node.children) {
        queue.push(child)
      }
    }
  }
}

/**
 * Start a new animation batch. No-op — queue continuity is maintained.
 */
export function startNewAnimationBatch(): void {
  // intentionally no-op for queue continuity
}

/**
 * Get the total delay (ms) before this node's content should start fading in.
 * = time until border reveal + BORDER_LEAD
 */
export function getNextStaggerDelay(nodeId?: string): number {
  if (!nodeId) return 0
  const revealAt = nodeRevealTime.get(nodeId)
  if (revealAt === undefined) return 0
  const now = Date.now()
  const waitForBorder = Math.max(0, revealAt - now)
  return waitForBorder + BORDER_LEAD
}

/**
 * Check if a node's border should be visible yet.
 * Returns true when the current time has reached the node's scheduled reveal.
 */
export function isNodeBorderReady(nodeId: string): boolean {
  const revealAt = nodeRevealTime.get(nodeId)
  if (revealAt === undefined) return false
  return Date.now() >= revealAt
}

/** Get the scheduled reveal timestamp for a node (undefined if not queued). */
export function getNodeRevealTime(nodeId: string): number | undefined {
  return nodeRevealTime.get(nodeId)
}

/** Reset all animation state. Call once at the start of a generation. */
export function resetAnimationState(): void {
  pendingAnimationNodes.clear()
  nodeRevealTime.clear()
  nextRevealAt = 0
}
