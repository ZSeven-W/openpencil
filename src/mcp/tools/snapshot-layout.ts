import { resolve } from 'node:path'
import { openDocument } from '../document-manager'
import { computeLayoutTree, type LayoutEntry } from '../utils/node-operations'

export interface SnapshotLayoutParams {
  filePath: string
  parentId?: string
  maxDepth?: number
}

export async function handleSnapshotLayout(
  params: SnapshotLayoutParams,
): Promise<{ layout: LayoutEntry[] }> {
  const filePath = resolve(params.filePath)
  const doc = await openDocument(filePath)

  const maxDepth = params.maxDepth ?? 1

  let nodes = doc.children
  if (params.parentId) {
    const findNode = (
      list: typeof nodes,
      id: string,
    ): (typeof nodes)[0] | undefined => {
      for (const n of list) {
        if (n.id === id) return n
        if ('children' in n && n.children) {
          const found = findNode(n.children, id)
          if (found) return found
        }
      }
      return undefined
    }
    const parent = findNode(doc.children, params.parentId)
    if (!parent) {
      throw new Error(`Node not found: ${params.parentId}`)
    }
    nodes =
      'children' in parent && parent.children ? parent.children : []
  }

  const layout = computeLayoutTree(nodes, doc.children, maxDepth)
  return { layout }
}
