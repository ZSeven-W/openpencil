import type { PenDocument, PenNode } from '@/types/pen'
import { getEffect } from '@/animation/effect-registry'

/**
 * Walk the node tree and remove clips whose effectId references
 * an effect that is not registered. Call after loading a document
 * to garbage-collect stale animation data.
 */
export function reconcileAnimationWithDocument(nodes: PenNode[]): void {
  for (const node of nodes) {
    if (node.clips && node.clips.length > 0) {
      node.clips = node.clips.filter((clip) => {
        if (clip.kind !== 'animation') return true
        if (!clip.effectId) return true
        return !!getEffect(clip.effectId)
      })
      if (node.clips.length === 0) {
        node.clips = undefined
      }
    }
    if ('children' in node && node.children) {
      reconcileAnimationWithDocument(node.children)
    }
  }
}

/**
 * Return a shallow copy of the node with invalid effectIds cleared
 * from its clips array.
 */
export function cleanOrphanedClips(
  node: PenNode,
  existingEffectIds: Set<string>,
): PenNode {
  if (!node.clips || node.clips.length === 0) return node

  const cleaned = node.clips.filter((clip) => {
    if (clip.kind !== 'animation') return true
    if (!clip.effectId) return true
    return existingEffectIds.has(clip.effectId)
  })

  if (cleaned.length === node.clips.length) return node

  return {
    ...node,
    clips: cleaned.length > 0 ? cleaned : undefined,
  } as PenNode
}

// ---------------------------------------------------------------------------
// Backward-compatible stubs — these are still imported elsewhere but
// no longer needed since clips live directly on nodes.
// ---------------------------------------------------------------------------

/**
 * @deprecated Clips are persisted on nodes directly. This is a no-op.
 */
export function injectAnimationData(): void {
  // no-op — clips are already on nodes in the document
}

/**
 * @deprecated Clips are loaded with nodes directly. This is a no-op.
 */
export function extractAnimationData(_doc: PenDocument): void {
  // no-op — clips are already on nodes in the document
}
