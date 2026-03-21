import type { PenDocument, PenNode } from '@/types/pen'
import {
  findNodeInTree,
  findParentInTree,
  removeNodeFromTree,
  updateNodeInTree,
  insertNodeInTree,
  flattenNodes,
  getNodeBounds,
  cloneNodeWithNewIds,
} from '@/stores/document-tree-utils'

// Re-export from pen-core (via shim)
export {
  findNodeInTree,
  findParentInTree,
  removeNodeFromTree,
  updateNodeInTree,
  insertNodeInTree,
  flattenNodes,
  getNodeBounds,
  cloneNodeWithNewIds,
}

// ---------------------------------------------------------------------------
// MCP-specific utilities
// ---------------------------------------------------------------------------

/** Get the working children for an MCP operation. */
export function getDocChildren(doc: PenDocument, pageId?: string): PenNode[] {
  if (doc.pages && doc.pages.length > 0) {
    if (pageId) {
      const page = doc.pages.find((p) => p.id === pageId)
      if (!page) throw new Error(`Page not found: ${pageId}`)
      return page.children
    }
    return doc.pages[0].children
  }
  return doc.children
}

/** Set the working children for an MCP operation. */
export function setDocChildren(doc: PenDocument, children: PenNode[], pageId?: string): void {
  if (doc.pages && doc.pages.length > 0) {
    if (pageId) {
      const page = doc.pages.find((p) => p.id === pageId)
      if (!page) throw new Error(`Page not found: ${pageId}`)
      page.children = children
    } else {
      doc.pages[0].children = children
    }
  } else {
    doc.children = children
  }
}

/** Search nodes matching a pattern. */
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
      const isReusable = 'reusable' in node && (node as unknown as Record<string, unknown>).reusable === true
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
