import { openDocument, saveDocument, resolveDocPath } from '../document-manager'
import {
  findNodeInTree,
  findParentInTree,
  insertNodeInTree,
  updateNodeInTree,
  removeNodeFromTree,
  cloneNodeWithNewIds,
  flattenNodes,
  getDocChildren,
  setDocChildren,
} from '../utils/node-operations'
import { generateId } from '../utils/id'
import { sanitizeObject } from '../utils/sanitize'
import { resolveTreeRoles, resolveTreePostPass } from '../../services/ai/role-resolver'
import '../../services/ai/role-definitions/index'
import {
  applyIconPathResolution,
  applyNoEmojiIconHeuristic,
} from '../../services/ai/icon-resolver'
import {
  ensureUniqueNodeIds,
  sanitizeLayoutChildPositions,
  sanitizeScreenFrameBounds,
} from '../../services/ai/design-node-sanitization'
import type { PenDocument, PenNode } from '../../types/pen'

// ---------------------------------------------------------------------------
// Shared post-processing (extracted from batch-design.ts)
// ---------------------------------------------------------------------------

export function postProcessNode(doc: PenDocument, canvasWidth: number): void {
  const children = getDocChildren(doc)
  for (const target of children) {
    resolveTreeRoles(target, canvasWidth)
    resolveTreePostPass(target, canvasWidth)

    const flat = flattenNodes([target])
    for (const node of flat) {
      if (node.type === 'path') applyIconPathResolution(node)
      if (node.type === 'text') applyNoEmojiIconHeuristic(node)
    }

    const usedIds = new Set<string>()
    const idCounters = new Map<string, number>()
    ensureUniqueNodeIds(target, usedIds, idCounters)
    sanitizeLayoutChildPositions(target, false)
    sanitizeScreenFrameBounds(target)
  }
}

function countNodes(nodes: PenNode[]): number {
  let count = 0
  for (const node of nodes) {
    count++
    if ('children' in node && node.children) {
      count += countNodes(node.children)
    }
  }
  return count
}

/** A root frame is "empty" if it has no children. */
function isEmptyFrame(node: PenNode): boolean {
  return (
    node.type === 'frame' &&
    (!('children' in node) || !node.children || node.children.length === 0)
  )
}

// ---------------------------------------------------------------------------
// insert_node
// ---------------------------------------------------------------------------

export interface InsertNodeParams {
  filePath: string
  parent: string | null
  data: Record<string, any>
  postProcess?: boolean
  canvasWidth?: number
}

export async function handleInsertNode(
  params: InsertNodeParams,
): Promise<{ nodeId: string; nodeCount: number; postProcessed?: boolean }> {
  const filePath = resolveDocPath(params.filePath)
  let doc = await openDocument(filePath)
  doc = structuredClone(doc)

  const data = sanitizeObject(params.data)
  const node = { ...data, id: generateId() } as PenNode
  const parent = params.parent

  // Auto-replace: when inserting a frame at root level and an empty
  // root frame exists, replace it instead of creating a sibling.
  if (parent === null && data.type === 'frame') {
    const children = getDocChildren(doc)
    const emptyIdx = children.findIndex((n) => isEmptyFrame(n))
    if (emptyIdx !== -1) {
      const emptyFrame = children[emptyIdx]
      if (emptyFrame.x !== undefined) (node as any).x = emptyFrame.x
      if (emptyFrame.y !== undefined) (node as any).y = emptyFrame.y
      let updated = removeNodeFromTree(children, emptyFrame.id)
      updated = insertNodeInTree(updated, null, node, emptyIdx)
      setDocChildren(doc, updated)

      if (params.postProcess) postProcessNode(doc, params.canvasWidth ?? 1200)
      await saveDocument(filePath, doc)
      return {
        nodeId: node.id,
        nodeCount: countNodes(getDocChildren(doc)),
        postProcessed: params.postProcess || undefined,
      }
    }
  }

  setDocChildren(doc, insertNodeInTree(getDocChildren(doc), parent, node))

  if (params.postProcess) postProcessNode(doc, params.canvasWidth ?? 1200)
  await saveDocument(filePath, doc)
  return {
    nodeId: node.id,
    nodeCount: countNodes(getDocChildren(doc)),
    postProcessed: params.postProcess || undefined,
  }
}

// ---------------------------------------------------------------------------
// update_node
// ---------------------------------------------------------------------------

export interface UpdateNodeParams {
  filePath: string
  nodeId: string
  data: Record<string, any>
  postProcess?: boolean
  canvasWidth?: number
}

export async function handleUpdateNode(
  params: UpdateNodeParams,
): Promise<{ ok: true; postProcessed?: boolean }> {
  const filePath = resolveDocPath(params.filePath)
  let doc = await openDocument(filePath)
  doc = structuredClone(doc)

  const data = sanitizeObject(params.data)
  const target = findNodeInTree(getDocChildren(doc), params.nodeId)
  if (!target) throw new Error(`Node not found: ${params.nodeId}`)

  setDocChildren(doc, updateNodeInTree(getDocChildren(doc), params.nodeId, data))

  if (params.postProcess) postProcessNode(doc, params.canvasWidth ?? 1200)
  await saveDocument(filePath, doc)
  return { ok: true, postProcessed: params.postProcess || undefined }
}

// ---------------------------------------------------------------------------
// delete_node
// ---------------------------------------------------------------------------

export interface DeleteNodeParams {
  filePath: string
  nodeId: string
}

export async function handleDeleteNode(
  params: DeleteNodeParams,
): Promise<{ ok: true }> {
  const filePath = resolveDocPath(params.filePath)
  let doc = await openDocument(filePath)
  doc = structuredClone(doc)

  const target = findNodeInTree(getDocChildren(doc), params.nodeId)
  if (!target) throw new Error(`Node not found: ${params.nodeId}`)

  setDocChildren(doc, removeNodeFromTree(getDocChildren(doc), params.nodeId))
  await saveDocument(filePath, doc)
  return { ok: true }
}

// ---------------------------------------------------------------------------
// move_node
// ---------------------------------------------------------------------------

export interface MoveNodeParams {
  filePath: string
  nodeId: string
  parent: string | null
  index?: number
}

export async function handleMoveNode(
  params: MoveNodeParams,
): Promise<{ ok: true }> {
  const filePath = resolveDocPath(params.filePath)
  let doc = await openDocument(filePath)
  doc = structuredClone(doc)

  const node = findNodeInTree(getDocChildren(doc), params.nodeId)
  if (!node) throw new Error(`Node not found: ${params.nodeId}`)

  let children = removeNodeFromTree(getDocChildren(doc), params.nodeId)
  children = insertNodeInTree(children, params.parent, node, params.index)
  setDocChildren(doc, children)

  await saveDocument(filePath, doc)
  return { ok: true }
}

// ---------------------------------------------------------------------------
// copy_node
// ---------------------------------------------------------------------------

export interface CopyNodeParams {
  filePath: string
  sourceId: string
  parent: string | null
  overrides?: Record<string, any>
}

export async function handleCopyNode(
  params: CopyNodeParams,
): Promise<{ nodeId: string; nodeCount: number }> {
  const filePath = resolveDocPath(params.filePath)
  let doc = await openDocument(filePath)
  doc = structuredClone(doc)

  const source = findNodeInTree(getDocChildren(doc), params.sourceId)
  if (!source) throw new Error(`Source node not found: ${params.sourceId}`)

  const cloned = cloneNodeWithNewIds(source, generateId)
  if (params.overrides) {
    const safe = sanitizeObject(params.overrides)
    Object.assign(cloned, safe)
    // Don't override the cloned id
    if (safe.id) delete (cloned as any).id
  }

  setDocChildren(doc, insertNodeInTree(getDocChildren(doc), params.parent, cloned))
  await saveDocument(filePath, doc)
  return {
    nodeId: cloned.id,
    nodeCount: countNodes(getDocChildren(doc)),
  }
}

// ---------------------------------------------------------------------------
// replace_node
// ---------------------------------------------------------------------------

export interface ReplaceNodeParams {
  filePath: string
  nodeId: string
  data: Record<string, any>
  postProcess?: boolean
  canvasWidth?: number
}

export async function handleReplaceNode(
  params: ReplaceNodeParams,
): Promise<{ nodeId: string; nodeCount: number; postProcessed?: boolean }> {
  const filePath = resolveDocPath(params.filePath)
  let doc = await openDocument(filePath)
  doc = structuredClone(doc)

  const data = sanitizeObject(params.data)
  const oldNode = findNodeInTree(getDocChildren(doc), params.nodeId)
  if (!oldNode) throw new Error(`Node not found: ${params.nodeId}`)

  const newNode = { ...data, id: generateId() } as PenNode

  // Find parent to determine insertion index
  const parent = findParentInTree(getDocChildren(doc), params.nodeId)
  const parentId = parent ? parent.id : null
  const siblings = parent
    ? ('children' in parent ? parent.children ?? [] : [])
    : getDocChildren(doc)
  const idx = siblings.findIndex((n) => n.id === oldNode.id)

  let children = removeNodeFromTree(getDocChildren(doc), oldNode.id)
  children = insertNodeInTree(children, parentId, newNode, idx)
  setDocChildren(doc, children)

  if (params.postProcess) postProcessNode(doc, params.canvasWidth ?? 1200)
  await saveDocument(filePath, doc)
  return {
    nodeId: newNode.id,
    nodeCount: countNodes(getDocChildren(doc)),
    postProcessed: params.postProcess || undefined,
  }
}
