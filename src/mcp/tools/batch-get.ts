import { resolve } from 'node:path'
import { openDocument } from '../document-manager'
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
  filePath: string
  patterns?: SearchPattern[]
  nodeIds?: string[]
  parentId?: string
  readDepth?: number
  searchDepth?: number
}

export async function handleBatchGet(
  params: BatchGetParams,
): Promise<{ nodes: Record<string, unknown>[] }> {
  const filePath = resolve(params.filePath)
  const doc = await openDocument(filePath)

  const readDepth = params.readDepth ?? 1
  const searchDepth = params.searchDepth ?? Infinity

  // If no patterns or nodeIds, return top-level children
  if (!params.patterns?.length && !params.nodeIds?.length) {
    const rootNodes = params.parentId
      ? (() => {
          const parent = findNodeInTree(getDocChildren(doc), params.parentId)
          return parent && 'children' in parent && parent.children
            ? parent.children
            : []
        })()
      : getDocChildren(doc)
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
          const parent = findNodeInTree(getDocChildren(doc), params.parentId)
          return parent && 'children' in parent && parent.children
            ? parent.children
            : []
        })()
      : getDocChildren(doc)

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
      const node = findNodeInTree(getDocChildren(doc), id)
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
