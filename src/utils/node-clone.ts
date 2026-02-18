import type { PenNode } from '@/types/pen'

function reassignIds(node: PenNode): PenNode {
  const cloned = { ...node, id: crypto.randomUUID() }
  if ('children' in cloned && cloned.children) {
    cloned.children = cloned.children.map(reassignIds)
  }
  return cloned as PenNode
}

export function cloneNodesWithNewIds(
  nodes: PenNode[],
  offset = 0,
): PenNode[] {
  return structuredClone(nodes).map((node) => {
    const withNewId = reassignIds(node)
    if (offset !== 0) {
      withNewId.x = (withNewId.x ?? 0) + offset
      withNewId.y = (withNewId.y ?? 0) + offset
    }
    return withNewId
  })
}
