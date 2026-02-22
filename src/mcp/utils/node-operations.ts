import type { PenNode, RefNode } from '../../types/pen'

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

export function cloneNodeWithNewIds(
  node: PenNode,
  generateId: () => string,
): PenNode {
  const cloned = { ...node, id: generateId() } as PenNode
  if ('children' in cloned && cloned.children) {
    cloned.children = cloned.children.map((c) =>
      cloneNodeWithNewIds(c, generateId),
    )
  }
  return cloned
}

/** Get the bounding box of a node, resolving RefNode dimensions from component. */
export function getNodeBounds(
  node: PenNode,
  allNodes: PenNode[],
): { x: number; y: number; w: number; h: number } {
  const x = node.x ?? 0
  const y = node.y ?? 0
  let w = 'width' in node && typeof node.width === 'number' ? node.width : 0
  let h = 'height' in node && typeof node.height === 'number' ? node.height : 0
  if (node.type === 'ref' && !w) {
    const refComp = findNodeInTree(allNodes, (node as RefNode).ref)
    if (refComp) {
      w =
        'width' in refComp && typeof refComp.width === 'number'
          ? refComp.width
          : 100
      h =
        'height' in refComp && typeof refComp.height === 'number'
          ? refComp.height
          : 100
    }
  }
  return { x, y, w: w || 100, h: h || 100 }
}

/**
 * Search nodes matching a pattern. Supports:
 * - type: exact match on node type
 * - name: regex match on node name
 * - reusable: match reusable flag
 */
export function searchNodes(
  nodes: PenNode[],
  pattern: { type?: string; name?: string; reusable?: boolean },
  maxDepth = Infinity,
  currentDepth = 0,
): PenNode[] {
  if (currentDepth > maxDepth) return []
  const results: PenNode[] = []
  for (const node of nodes) {
    let matches = true
    if (pattern.type && node.type !== pattern.type) matches = false
    if (pattern.name) {
      const regex = new RegExp(pattern.name, 'i')
      if (!regex.test(node.name ?? '')) matches = false
    }
    if (pattern.reusable !== undefined) {
      const isReusable = 'reusable' in node && (node as any).reusable === true
      if (pattern.reusable !== isReusable) matches = false
    }
    if (matches) results.push(node)
    if ('children' in node && node.children) {
      results.push(
        ...searchNodes(node.children, pattern, maxDepth, currentDepth + 1),
      )
    }
  }
  return results
}

/** Read a node with depth-limited children. */
export function readNodeWithDepth(
  node: PenNode,
  depth: number,
): Record<string, unknown> {
  const result: Record<string, unknown> = { ...node }
  if (depth <= 0 && 'children' in node && node.children?.length) {
    result.children = '...'
  } else if ('children' in node && node.children) {
    result.children = node.children.map((c) =>
      readNodeWithDepth(c, depth - 1),
    )
  }
  return result
}

/** Compute bounding box layout tree for snapshot_layout. */
export function computeLayoutTree(
  nodes: PenNode[],
  allNodes: PenNode[],
  maxDepth: number,
  currentDepth = 0,
  parentX = 0,
  parentY = 0,
): LayoutEntry[] {
  const entries: LayoutEntry[] = []
  for (const node of nodes) {
    const bounds = getNodeBounds(node, allNodes)
    const absX = parentX + bounds.x
    const absY = parentY + bounds.y
    const entry: LayoutEntry = {
      id: node.id,
      name: node.name,
      type: node.type,
      x: absX,
      y: absY,
      width: bounds.w,
      height: bounds.h,
    }
    if (
      'children' in node &&
      node.children?.length &&
      currentDepth < maxDepth
    ) {
      entry.children = computeLayoutTree(
        node.children,
        allNodes,
        maxDepth,
        currentDepth + 1,
        absX,
        absY,
      )
    }
    entries.push(entry)
  }
  return entries
}

export interface LayoutEntry {
  id: string
  name?: string
  type: string
  x: number
  y: number
  width: number
  height: number
  children?: LayoutEntry[]
}
