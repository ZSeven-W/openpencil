import type { PenNode } from '@/types/pen'
import type { AnimationClip } from '@/types/animation'

export interface AnimationIndex {
  clipsByNode: Map<string, AnimationClip[]>
  animatedNodes: Set<string>
  version: number
}

let indexVersion = 0

function collectClips(
  nodes: PenNode[],
  clipsByNode: Map<string, AnimationClip[]>,
  animatedNodes: Set<string>,
): void {
  for (const node of nodes) {
    if (node.clips && node.clips.length > 0) {
      clipsByNode.set(node.id, node.clips)
      animatedNodes.add(node.id)
    }
    if ('children' in node && node.children) {
      collectClips(node.children, clipsByNode, animatedNodes)
    }
  }
}

export function buildAnimationIndex(nodes: PenNode[]): AnimationIndex {
  const clipsByNode = new Map<string, AnimationClip[]>()
  const animatedNodes = new Set<string>()
  collectClips(nodes, clipsByNode, animatedNodes)
  return { clipsByNode, animatedNodes, version: ++indexVersion }
}

export function getClipsForNode(
  index: AnimationIndex,
  nodeId: string,
): AnimationClip[] {
  return index.clipsByNode.get(nodeId) ?? []
}
