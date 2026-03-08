import { openDocument, resolveDocPath } from '../document-manager'
import {
  findNodeInTree,
  searchNodes,
  readNodeWithDepth,
  getDocChildren,
} from '../utils/node-operations'
import type { PenNode } from '../../types/pen'

export interface SearchPattern {
  type?: string
  name?: string
  reusable?: boolean
}

export interface BatchGetParams {
  filePath?: string
  patterns?: SearchPattern[]
  nodeIds?: string[]
  parentId?: string
  readDepth?: number
  searchDepth?: number
  pageId?: string
}

export async function handleBatchGet(
  params: BatchGetParams,
): Promise<{ nodes: Record<string, unknown>[] }> {
  const filePath = resolveDocPath(params.filePath)
  const doc = await openDocument(filePath)
  const pageId = params.pageId

  const readDepth = params.readDepth ?? 1
  const searchDepth = params.searchDepth ?? Infinity

  // If no patterns or nodeIds, return top-level children
  if (!params.patterns?.length && !params.nodeIds?.length) {
    const rootNodes = params.parentId
      ? (() => {
          const parent = findNodeInTree(getDocChildren(doc, pageId), params.parentId)
          return parent && 'children' in parent && parent.children
            ? parent.children
            : []
        })()
      : getDocChildren(doc, pageId)
    return {
      nodes: rootNodes.map((n) => readNodeWithDepth(n, readDepth)),
    }
  }

  const results: PenNode[] = []
  const seen = new Set<string>()

  // Search by patterns
  if (params.patterns?.length) {
    const searchRoot = params.parentId
      ? (() => {
          const parent = findNodeInTree(getDocChildren(doc, pageId), params.parentId)
          return parent && 'children' in parent && parent.children
            ? parent.children
            : []
        })()
      : getDocChildren(doc, pageId)

    for (const pattern of params.patterns) {
      const found = searchNodes(searchRoot, pattern, searchDepth)
      for (const node of found) {
        if (!seen.has(node.id)) {
          seen.add(node.id)
          results.push(node)
        }
      }
    }
  }

  // Read by IDs
  if (params.nodeIds?.length) {
    for (const id of params.nodeIds) {
      if (seen.has(id)) continue
      const node = findNodeInTree(getDocChildren(doc, pageId), id)
      if (node) {
        seen.add(id)
        results.push(node)
      }
    }
  }

  return {
    nodes: results.map((n) => readNodeWithDepth(n, readDepth)),
  }
}
