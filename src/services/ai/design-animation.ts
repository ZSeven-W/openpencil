import type { PenNode } from '@/types/pen'

// ---------------------------------------------------------------------------
// Shared coordination set — checked by canvas-sync to trigger fade-in
// ---------------------------------------------------------------------------

/** IDs of nodes that should fade in when their Fabric object is created. */
export const pendingAnimationNodes = new Set<string>()

// ---------------------------------------------------------------------------
// Stagger counter — determines delay per node within a batch
// ---------------------------------------------------------------------------

/** Index of the next animated object across all batches in a generation. */
let currentIndex = 0
/** Index where the current batch started (reset per JSON block). */
let batchStartIndex = 0

/**
 * Mark all node IDs in the tree for fade-in animation.
 * When canvas-sync creates Fabric objects for these IDs, it will set
 * opacity to 0 and schedule a delayed fade-in.
 */
export function markNodesForAnimation(nodes: PenNode[]): void {
  for (const node of nodes) {
    pendingAnimationNodes.add(node.id)
    if ('children' in node && Array.isArray(node.children)) {
      markNodesForAnimation(node.children)
    }
  }
}

/**
 * Start a new animation batch. Resets the relative stagger so that the
 * first node in this batch starts fading in immediately (delay 0).
 * Call this before each JSON block's upsert.
 */
export function startNewAnimationBatch(): void {
  batchStartIndex = currentIndex
}

/**
 * Get the stagger delay (ms) for the next animated object.
 * Called by canvas-sync each time it creates a new Fabric object
 * whose ID is in `pendingAnimationNodes`.
 */
export function getNextStaggerDelay(): number {
  const relativeIndex = currentIndex - batchStartIndex
  currentIndex++
  return relativeIndex * 60
}

/** Reset all animation state. Call once at the start of a generation. */
export function resetAnimationState(): void {
  pendingAnimationNodes.clear()
  batchStartIndex = 0
  currentIndex = 0
}
