import { openDocument, saveDocument, resolveDocPath } from '../document-manager'
import {
  findNodeInTree,
  flattenNodes,
  getDocChildren,
  setDocChildren,
  computeLayoutTree,
} from '../utils/node-operations'
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
import type { PenNode } from '../../types/pen'
import type { LayoutEntry } from '../utils/node-operations'

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface DesignRefineParams {
  filePath?: string
  rootId: string
  canvasWidth?: number
  pageId?: string
}

interface RefineFix {
  nodeId: string
  nodeName?: string
  fix: string
}

interface DesignRefineResult {
  rootId: string
  totalNodeCount: number
  fixes: RefineFix[]
  layoutSnapshot: LayoutEntry[]
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

export async function handleDesignRefine(
  params: DesignRefineParams,
): Promise<DesignRefineResult> {
  const filePath = resolveDocPath(params.filePath)
  let doc = await openDocument(filePath)
  doc = structuredClone(doc)
  const pageId = params.pageId
  const canvasWidth = params.canvasWidth ?? 1200

  const allChildren = getDocChildren(doc, pageId)
  const root = findNodeInTree(allChildren, params.rootId)
  if (!root) {
    throw new Error(`Root node not found: ${params.rootId}`)
  }

  const fixes: RefineFix[] = []

  // Snapshot state before processing for diff
  const beforeSnapshot = captureNodeState(root)

  // 1. Full-tree role resolution
  resolveTreeRoles(root, canvasWidth)

  // 2. Full-tree post-pass (card row equalization, overflow fix, form normalization, etc.)
  resolveTreePostPass(root, canvasWidth)

  // 3. Icon resolution + emoji removal
  const flat = flattenNodes([root])
  for (const node of flat) {
    if (node.type === 'path') applyIconPathResolution(node)
    if (node.type === 'text') applyNoEmojiIconHeuristic(node)
  }

  // 4. Sanitization passes
  const usedIds = new Set<string>()
  const idCounters = new Map<string, number>()
  ensureUniqueNodeIds(root, usedIds, idCounters)
  sanitizeLayoutChildPositions(root, false)
  sanitizeScreenFrameBounds(root)

  // Diff to find what was changed
  const afterSnapshot = captureNodeState(root)
  diffSnapshots(beforeSnapshot, afterSnapshot, fixes)

  // Persist
  setDocChildren(doc, allChildren, pageId)
  await saveDocument(filePath, doc)

  // Build layout snapshot
  const layoutSnapshot = computeLayoutTree([root], allChildren, 3)

  return {
    rootId: params.rootId,
    totalNodeCount: flat.length,
    fixes,
    layoutSnapshot,
  }
}

// ---------------------------------------------------------------------------
// State capture and diffing for fix reporting
// ---------------------------------------------------------------------------

interface NodeState {
  id: string
  name?: string
  props: Record<string, unknown>
}

/** Capture a flat snapshot of all node properties for diffing. */
function captureNodeState(root: PenNode): Map<string, NodeState> {
  const states = new Map<string, NodeState>()
  const flat = flattenNodes([root])
  for (const node of flat) {
    const props: Record<string, unknown> = {}
    for (const [key, value] of Object.entries(node)) {
      if (key === 'children' || key === 'id') continue
      props[key] = typeof value === 'object' ? JSON.stringify(value) : value
    }
    states.set(node.id, { id: node.id, name: node.name, props })
  }
  return states
}

/** Compare before/after snapshots and report fixes. */
function diffSnapshots(
  before: Map<string, NodeState>,
  after: Map<string, NodeState>,
  fixes: RefineFix[],
): void {
  for (const [id, afterState] of after) {
    const beforeState = before.get(id)
    if (!beforeState) continue

    const changedProps: string[] = []
    for (const [key, afterVal] of Object.entries(afterState.props)) {
      const beforeVal = beforeState.props[key]
      if (beforeVal !== afterVal) {
        const displayVal =
          typeof afterVal === 'string' && afterVal.startsWith('{')
            ? key
            : `${key}=${String(afterVal)}`
        changedProps.push(displayVal)
      }
    }

    if (changedProps.length > 0) {
      fixes.push({
        nodeId: id,
        nodeName: afterState.name,
        fix: `Updated: ${changedProps.join(', ')}`,
      })
    }
  }
}
