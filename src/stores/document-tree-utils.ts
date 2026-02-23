import type { PenDocument, PenNode, RefNode } from '@/types/pen'

export const DEFAULT_FRAME_ID = 'root-frame'

export function createEmptyDocument(): PenDocument {
  return {
    version: '1.0.0',
    children: [
      {
        id: DEFAULT_FRAME_ID,
        type: 'frame',
        name: 'Frame',
        x: 0,
        y: 0,
        width: 1200,
        height: 800,
        fill: [{ type: 'solid', color: '#FFFFFF' }],
        children: [],
      },
    ],
  }
}

export function findNodeInTree(
  nodes: PenNode[],
  id: string,
): PenNode | undefined {
  for (const node of nodes) {
    if (node.id === id) return node
    if ('children' in node && node.children) {
      const found = findNodeInTree(node.children, id)
      if (found) return found
    }
  }
  return undefined
}

export function findParentInTree(
  nodes: PenNode[],
  id: string,
): PenNode | undefined {
  for (const node of nodes) {
    if ('children' in node && node.children) {
      for (const child of node.children) {
        if (child.id === id) return node
      }
      const found = findParentInTree(node.children, id)
      if (found) return found
    }
  }
  return undefined
}

export function removeNodeFromTree(nodes: PenNode[], id: string): PenNode[] {
  return nodes
    .filter((n) => n.id !== id)
    .map((n) => {
      if ('children' in n && n.children) {
        return { ...n, children: removeNodeFromTree(n.children, id) }
      }
      return n
    })
}

export function updateNodeInTree(
  nodes: PenNode[],
  id: string,
  updates: Partial<PenNode>,
): PenNode[] {
  return nodes.map((n) => {
    if (n.id === id) {
      return { ...n, ...updates } as PenNode
    }
    if ('children' in n && n.children) {
      return {
        ...n,
        children: updateNodeInTree(n.children, id, updates),
      } as PenNode
    }
    return n
  })
}

export function flattenNodes(nodes: PenNode[]): PenNode[] {
  const result: PenNode[] = []
  for (const node of nodes) {
    result.push(node)
    if ('children' in node && node.children) {
      result.push(...flattenNodes(node.children))
    }
  }
  return result
}

export function insertNodeInTree(
  nodes: PenNode[],
  parentId: string | null,
  node: PenNode,
  index?: number,
): PenNode[] {
  if (parentId === null) {
    const arr = [...nodes]
    if (index !== undefined) {
      arr.splice(index, 0, node)
    } else {
      arr.push(node)
    }
    return arr
  }

  return nodes.map((n) => {
    if (n.id === parentId) {
      const children = 'children' in n && n.children ? [...n.children] : []
      if (index !== undefined) {
        children.splice(index, 0, node)
      } else {
        children.push(node)
      }
      return { ...n, children } as PenNode
    }
    if ('children' in n && n.children) {
      return {
        ...n,
        children: insertNodeInTree(n.children, parentId, node, index),
      } as PenNode
    }
    return n
  })
}

export function isDescendantOf(
  nodes: PenNode[],
  nodeId: string,
  ancestorId: string,
): boolean {
  const ancestor = findNodeInTree(nodes, ancestorId)
  if (!ancestor || !('children' in ancestor) || !ancestor.children) return false
  for (const child of ancestor.children) {
    if (child.id === nodeId) return true
    if (isDescendantOf([child], nodeId, child.id)) return true
  }
  return false
}

/** Resolve the bounding box of a node, falling back to its referenced component for RefNodes. */
export function getNodeBounds(
  node: PenNode,
  allNodes: PenNode[],
): { x: number; y: number; w: number; h: number } {
  const x = node.x ?? 0
  const y = node.y ?? 0
  let w = ('width' in node && typeof node.width === 'number') ? node.width : 0
  let h = ('height' in node && typeof node.height === 'number') ? node.height : 0
  if (node.type === 'ref' && !w) {
    const refComp = findNodeInTree(allNodes, (node as RefNode).ref)
    if (refComp) {
      w = ('width' in refComp && typeof refComp.width === 'number') ? refComp.width : 100
      h = ('height' in refComp && typeof refComp.height === 'number') ? refComp.height : 100
    }
  }
  return { x, y, w: w || 100, h: h || 100 }
}

/**
 * Find a clear X position to the right of `sourceX + sourceW` that doesn't
 * overlap any sibling (excluding `excludeId`) on the same vertical band.
 */
export function findClearX(
  sourceX: number,
  sourceW: number,
  proposedY: number,
  proposedH: number,
  siblings: PenNode[],
  excludeId: string,
  allNodes: PenNode[],
  gap = 20,
): number {
  const proposedW = sourceW
  let proposedX = sourceX + sourceW + gap

  const siblingBounds: { x: number; y: number; w: number; h: number }[] = []
  for (const sib of siblings) {
    if (sib.id === excludeId) continue
    const b = getNodeBounds(sib, allNodes)
    if (b.w > 0 && b.h > 0) siblingBounds.push(b)
  }

  let maxAttempts = 100
  while (maxAttempts-- > 0) {
    const hasOverlap = siblingBounds.some((b) => {
      const overlapX = proposedX < b.x + b.w && proposedX + proposedW > b.x
      const overlapY = proposedY < b.y + b.h && proposedY + proposedH > b.y
      return overlapX && overlapY
    })
    if (!hasOverlap) break
    let maxRight = proposedX
    for (const b of siblingBounds) {
      const overlapX = proposedX < b.x + b.w && proposedX + proposedW > b.x
      const overlapY = proposedY < b.y + b.h && proposedY + proposedH > b.y
      if (overlapX && overlapY && b.x + b.w > maxRight) {
        maxRight = b.x + b.w
      }
    }
    proposedX = maxRight + gap
  }

  return proposedX
}

/** Recursively scale all children's relative positions and sizes. */
export function scaleChildrenInPlace(
  children: PenNode[],
  scaleX: number,
  scaleY: number,
): PenNode[] {
  return children.map((child) => {
    const updated: Record<string, unknown> = { ...child }
    if (child.x !== undefined) updated.x = child.x * scaleX
    if (child.y !== undefined) updated.y = child.y * scaleY
    if ('width' in child && typeof child.width === 'number') {
      updated.width = child.width * scaleX
    }
    if ('height' in child && typeof child.height === 'number') {
      updated.height = child.height * scaleY
    }
    if ('children' in child && child.children) {
      updated.children = scaleChildrenInPlace(child.children, scaleX, scaleY)
    }
    return updated as unknown as PenNode
  })
}

/** Recursively rotate all children's relative positions and angles. */
export function rotateChildrenInPlace(
  children: PenNode[],
  angleDeltaDeg: number,
): PenNode[] {
  const rad = (angleDeltaDeg * Math.PI) / 180
  const cos = Math.cos(rad)
  const sin = Math.sin(rad)
  return children.map((child) => {
    const x = child.x ?? 0
    const y = child.y ?? 0
    const updated: Record<string, unknown> = { ...child }
    updated.x = x * cos - y * sin
    updated.y = x * sin + y * cos
    updated.rotation = ((child.rotation ?? 0) + angleDeltaDeg) % 360
    if ('children' in child && child.children) {
      updated.children = rotateChildrenInPlace(child.children, angleDeltaDeg)
    }
    return updated as unknown as PenNode
  })
}
