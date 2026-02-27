import { resolve } from 'node:path'
import { openDocument } from '../document-manager'
import { getNodeBounds, findNodeInTree, getDocChildren } from '../utils/node-operations'
import type { PenNode } from '../../types/pen'

export interface FindEmptySpaceParams {
  filePath: string
  width: number
  height: number
  padding?: number
  direction: 'top' | 'right' | 'bottom' | 'left'
  nodeId?: string
}

export interface FindEmptySpaceResult {
  x: number
  y: number
}

export async function handleFindEmptySpace(
  params: FindEmptySpaceParams,
): Promise<FindEmptySpaceResult> {
  const filePath = resolve(params.filePath)
  const doc = await openDocument(filePath)
  const padding = params.padding ?? 50

  // Compute bounding box of reference content
  const docChildren = getDocChildren(doc)
  let nodes: PenNode[]
  if (params.nodeId) {
    const node = findNodeInTree(docChildren, params.nodeId)
    if (!node) throw new Error(`Node not found: ${params.nodeId}`)
    nodes = [node]
  } else {
    nodes = docChildren
  }

  if (nodes.length === 0) {
    return { x: 0, y: 0 }
  }

  // Compute combined bounding box
  let minX = Infinity
  let minY = Infinity
  let maxX = -Infinity
  let maxY = -Infinity
  for (const node of nodes) {
    const b = getNodeBounds(node, docChildren)
    minX = Math.min(minX, b.x)
    minY = Math.min(minY, b.y)
    maxX = Math.max(maxX, b.x + b.w)
    maxY = Math.max(maxY, b.y + b.h)
  }

  switch (params.direction) {
    case 'right':
      return { x: maxX + padding, y: minY }
    case 'left':
      return { x: minX - padding - params.width, y: minY }
    case 'bottom':
      return { x: minX, y: maxY + padding }
    case 'top':
      return { x: minX, y: minY - padding - params.height }
  }
}
