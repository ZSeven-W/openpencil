import { openDocument, saveDocument, resolveDocPath } from '../document-manager'
import {
  findNodeInTree,
  findParentInTree,
  insertNodeInTree,
  updateNodeInTree,
  removeNodeFromTree,
  cloneNodeWithNewIds,
  flattenNodes,
  readNodeWithDepth,
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
// Node data validation
// ---------------------------------------------------------------------------

const VALID_NODE_TYPES = new Set([
  'frame', 'group', 'rectangle', 'ellipse', 'line', 'polygon', 'path', 'text', 'image', 'ref',
])

function validateNodeData(data: Record<string, unknown>): void {
  if (!data.type) throw new Error('Node data must include a "type" field')
  if (!VALID_NODE_TYPES.has(data.type as string)) {
    throw new Error(
      `Invalid node type: "${data.type}". Valid types: ${[...VALID_NODE_TYPES].join(', ')}`,
    )
  }
  if (data.type === 'text' && data.content == null) {
    throw new Error('Text nodes must include a "content" field')
  }
  if (data.fill && !Array.isArray(data.fill)) {
    throw new Error('Fill must be an array, e.g. [{ type: "solid", color: "#hex" }]')
  }
  const strokeObj = data.stroke as Record<string, unknown> | undefined
  if (strokeObj && strokeObj.fill && !Array.isArray(strokeObj.fill)) {
    throw new Error('Stroke fill must be an array, e.g. { thickness: 1, fill: [{ type: "solid", color: "#hex" }] }')
  }
}

/** Recursively validate node data including children. */
function validateNodeTree(data: Record<string, unknown>): void {
  validateNodeData(data)
  if (Array.isArray(data.children)) {
    for (const child of data.children) {
      if (child && typeof child === 'object') validateNodeTree(child as Record<string, unknown>)
    }
  }
}

// ---------------------------------------------------------------------------
// Shared post-processing (extracted from batch-design.ts)
// ---------------------------------------------------------------------------

export function postProcessNode(doc: PenDocument, canvasWidth: number, pageId?: string): void {
  const children = getDocChildren(doc, pageId)
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
  filePath?: string
  parent: string | null
  data: Record<string, unknown>
  postProcess?: boolean
  canvasWidth?: number
  pageId?: string
}

export async function handleInsertNode(
  params: InsertNodeParams,
): Promise<{ nodeId: string; nodeCount: number; postProcessed?: boolean; node?: Record<string, unknown> }> {
  const filePath = resolveDocPath(params.filePath)
  let doc = await openDocument(filePath)
  doc = structuredClone(doc)
  const pageId = params.pageId

  const data = sanitizeObject(params.data)
  validateNodeTree(data)
  const node = { ...data, id: generateId() } as PenNode
  const parent = params.parent

  // Auto-replace: when inserting a frame at root level and an empty
  // root frame exists, replace it instead of creating a sibling.
  if (parent === null && data.type === 'frame') {
    const children = getDocChildren(doc, pageId)
    const emptyIdx = children.findIndex((n) => isEmptyFrame(n))
    if (emptyIdx !== -1) {
      const emptyFrame = children[emptyIdx]
      if (emptyFrame.x !== undefined) node.x = emptyFrame.x
      if (emptyFrame.y !== undefined) node.y = emptyFrame.y
      let updated = removeNodeFromTree(children, emptyFrame.id)
      updated = insertNodeInTree(updated, null, node, emptyIdx)
      setDocChildren(doc, updated, pageId)

      if (params.postProcess) postProcessNode(doc, params.canvasWidth ?? 1200, pageId)
      await saveDocument(filePath, doc)
      const finalNode = findNodeInTree(getDocChildren(doc, pageId), node.id)
      return {
        nodeId: node.id,
        nodeCount: countNodes(getDocChildren(doc, pageId)),
        postProcessed: params.postProcess || undefined,
        node: finalNode ? readNodeWithDepth(finalNode, 1) : undefined,
      }
    }
  }

  setDocChildren(doc, insertNodeInTree(getDocChildren(doc, pageId), parent, node), pageId)

  if (params.postProcess) postProcessNode(doc, params.canvasWidth ?? 1200, pageId)
  await saveDocument(filePath, doc)
  const finalNode = findNodeInTree(getDocChildren(doc, pageId), node.id)
  return {
    nodeId: node.id,
    nodeCount: countNodes(getDocChildren(doc, pageId)),
    postProcessed: params.postProcess || undefined,
    node: finalNode ? readNodeWithDepth(finalNode, 1) : undefined,
  }
}

// ---------------------------------------------------------------------------
// update_node
// ---------------------------------------------------------------------------

export interface UpdateNodeParams {
  filePath?: string
  nodeId: string
  data: Record<string, unknown>
  postProcess?: boolean
  canvasWidth?: number
  pageId?: string
}

export async function handleUpdateNode(
  params: UpdateNodeParams,
): Promise<{ ok: true; postProcessed?: boolean; node?: Record<string, unknown> }> {
  const filePath = resolveDocPath(params.filePath)
  let doc = await openDocument(filePath)
  doc = structuredClone(doc)
  const pageId = params.pageId

  const data = sanitizeObject(params.data)
  // Validate type if being changed
  if (data.type && !VALID_NODE_TYPES.has(data.type as string)) {
    throw new Error(`Invalid node type: "${data.type}". Valid types: ${[...VALID_NODE_TYPES].join(', ')}`)
  }
  if (data.fill && !Array.isArray(data.fill)) {
    throw new Error('Fill must be an array, e.g. [{ type: "solid", color: "#hex" }]')
  }
  const target = findNodeInTree(getDocChildren(doc, pageId), params.nodeId)
  if (!target) throw new Error(`Node not found: ${params.nodeId}`)

  setDocChildren(doc, updateNodeInTree(getDocChildren(doc, pageId), params.nodeId, data), pageId)

  if (params.postProcess) postProcessNode(doc, params.canvasWidth ?? 1200, pageId)
  await saveDocument(filePath, doc)
  const updatedNode = findNodeInTree(getDocChildren(doc, pageId), params.nodeId)
  return {
    ok: true,
    postProcessed: params.postProcess || undefined,
    node: updatedNode ? readNodeWithDepth(updatedNode, 0) : undefined,
  }
}

// ---------------------------------------------------------------------------
// delete_node
// ---------------------------------------------------------------------------

export interface DeleteNodeParams {
  filePath?: string
  nodeId: string
  pageId?: string
}

export async function handleDeleteNode(
  params: DeleteNodeParams,
): Promise<{ ok: true; remainingNodes: number }> {
  const filePath = resolveDocPath(params.filePath)
  let doc = await openDocument(filePath)
  doc = structuredClone(doc)
  const pageId = params.pageId

  const target = findNodeInTree(getDocChildren(doc, pageId), params.nodeId)
  if (!target) throw new Error(`Node not found: ${params.nodeId}`)

  setDocChildren(doc, removeNodeFromTree(getDocChildren(doc, pageId), params.nodeId), pageId)
  await saveDocument(filePath, doc)
  return { ok: true, remainingNodes: countNodes(getDocChildren(doc, pageId)) }
}

// ---------------------------------------------------------------------------
// move_node
// ---------------------------------------------------------------------------

export interface MoveNodeParams {
  filePath?: string
  nodeId: string
  parent: string | null
  index?: number
  pageId?: string
}

export async function handleMoveNode(
  params: MoveNodeParams,
): Promise<{ ok: true; nodeId: string; parentId: string | null }> {
  const filePath = resolveDocPath(params.filePath)
  let doc = await openDocument(filePath)
  doc = structuredClone(doc)
  const pageId = params.pageId

  const node = findNodeInTree(getDocChildren(doc, pageId), params.nodeId)
  if (!node) throw new Error(`Node not found: ${params.nodeId}`)

  let children = removeNodeFromTree(getDocChildren(doc, pageId), params.nodeId)
  children = insertNodeInTree(children, params.parent, node, params.index)
  setDocChildren(doc, children, pageId)

  await saveDocument(filePath, doc)
  const parentNode = findParentInTree(getDocChildren(doc, pageId), params.nodeId)
  return { ok: true, nodeId: params.nodeId, parentId: parentNode?.id ?? null }
}

// ---------------------------------------------------------------------------
// copy_node
// ---------------------------------------------------------------------------

export interface CopyNodeParams {
  filePath?: string
  sourceId: string
  parent: string | null
  overrides?: Record<string, unknown>
  pageId?: string
}

export async function handleCopyNode(
  params: CopyNodeParams,
): Promise<{ nodeId: string; nodeCount: number }> {
  const filePath = resolveDocPath(params.filePath)
  let doc = await openDocument(filePath)
  doc = structuredClone(doc)
  const pageId = params.pageId

  const source = findNodeInTree(getDocChildren(doc, pageId), params.sourceId)
  if (!source) throw new Error(`Source node not found: ${params.sourceId}`)

  const cloned = cloneNodeWithNewIds(source, generateId)
  if (params.overrides) {
    const safe = sanitizeObject(params.overrides)
    Object.assign(cloned, safe)
    // Don't override the cloned id
    if (safe.id) delete (cloned as unknown as Record<string, unknown>).id
  }

  setDocChildren(doc, insertNodeInTree(getDocChildren(doc, pageId), params.parent, cloned), pageId)
  await saveDocument(filePath, doc)
  return {
    nodeId: cloned.id,
    nodeCount: countNodes(getDocChildren(doc, pageId)),
  }
}

// ---------------------------------------------------------------------------
// replace_node
// ---------------------------------------------------------------------------

export interface ReplaceNodeParams {
  filePath?: string
  nodeId: string
  data: Record<string, unknown>
  postProcess?: boolean
  canvasWidth?: number
  pageId?: string
}

export async function handleReplaceNode(
  params: ReplaceNodeParams,
): Promise<{ nodeId: string; nodeCount: number; postProcessed?: boolean; node?: Record<string, unknown> }> {
  const filePath = resolveDocPath(params.filePath)
  let doc = await openDocument(filePath)
  doc = structuredClone(doc)
  const pageId = params.pageId

  const data = sanitizeObject(params.data)
  validateNodeTree(data)
  const oldNode = findNodeInTree(getDocChildren(doc, pageId), params.nodeId)
  if (!oldNode) throw new Error(`Node not found: ${params.nodeId}`)

  const newNode = { ...data, id: generateId() } as PenNode

  // Find parent to determine insertion index
  const parent = findParentInTree(getDocChildren(doc, pageId), params.nodeId)
  const parentId = parent ? parent.id : null
  const siblings = parent
    ? ('children' in parent ? parent.children ?? [] : [])
    : getDocChildren(doc, pageId)
  const idx = siblings.findIndex((n) => n.id === oldNode.id)

  let children = removeNodeFromTree(getDocChildren(doc, pageId), oldNode.id)
  children = insertNodeInTree(children, parentId, newNode, idx)
  setDocChildren(doc, children, pageId)

  if (params.postProcess) postProcessNode(doc, params.canvasWidth ?? 1200, pageId)
  await saveDocument(filePath, doc)
  const finalNode = findNodeInTree(getDocChildren(doc, pageId), newNode.id)
  return {
    nodeId: newNode.id,
    nodeCount: countNodes(getDocChildren(doc, pageId)),
    postProcessed: params.postProcess || undefined,
    node: finalNode ? readNodeWithDepth(finalNode, 1) : undefined,
  }
}
