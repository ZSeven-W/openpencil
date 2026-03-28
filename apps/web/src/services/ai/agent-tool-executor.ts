import type { AgentEvent, ToolResult, AuthLevel } from '@zseven-w/agent'
import type { PenNode } from '@/types/pen'

type ToolCallEvent = Extract<AgentEvent, { type: 'tool_call' }>

/** Auth levels that mutate the document and should be wrapped in an undo batch. */
const WRITE_LEVELS: Set<AuthLevel> = new Set(['create', 'modify', 'delete'])

/**
 * Client-side tool executor.
 *
 * Receives `tool_call` events from the SSE stream, dispatches them against the
 * live Zustand document store, wraps write operations in an undo batch, and
 * POSTs the result back to the server to unblock the agent loop.
 */
export class AgentToolExecutor {
  private sessionId: string
  /** Track root-level insert to prevent duplicate designs */
  private rootInsertId: string | null = null

  constructor(sessionId: string) {
    this.sessionId = sessionId
  }

  async execute(toolCall: ToolCallEvent): Promise<void> {
    const { id, name, args, level } = toolCall
    const isWrite = WRITE_LEVELS.has(level)

    if (isWrite) {
      const { useHistoryStore } = await import('@/stores/history-store')
      const { useDocumentStore } = await import('@/stores/document-store')
      useHistoryStore.getState().startBatch(useDocumentStore.getState().document)
    }

    let result: ToolResult
    try {
      result = await this.dispatch(name, args)
    } catch (err) {
      result = { success: false, error: String(err) }
    }

    if (isWrite) {
      const { useHistoryStore } = await import('@/stores/history-store')
      const { useDocumentStore } = await import('@/stores/document-store')
      useHistoryStore.getState().endBatch(useDocumentStore.getState().document)
    }

    await fetch('/api/ai/agent?action=result', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        sessionId: this.sessionId,
        toolCallId: id,
        result,
      }),
    })
  }

  // ---------------------------------------------------------------------------
  // Tool dispatch
  // ---------------------------------------------------------------------------

  private async dispatch(name: string, args: unknown): Promise<ToolResult> {
    switch (name) {
      case 'batch_get':
        return this.handleBatchGet(args as { ids?: string[]; patterns?: string[] })
      case 'snapshot_layout':
        return this.handleSnapshotLayout(args as { pageId?: string })
      case 'insert_node':
        return this.handleInsertNode(
          args as { parent: string | null; data: Record<string, unknown>; pageId?: string },
        )
      case 'update_node':
        return this.handleUpdateNode(args as { id: string; data: Record<string, unknown> })
      case 'delete_node':
        return this.handleDeleteNode(args as { id: string })
      case 'find_empty_space':
        return this.handleFindEmptySpace(
          args as { width: number; height: number; pageId?: string },
        )
      default:
        return { success: false, error: `Unknown tool: ${name}` }
    }
  }

  // ---------------------------------------------------------------------------
  // Read tools
  // ---------------------------------------------------------------------------

  private async handleBatchGet(
    args: { ids?: string[]; patterns?: string[] },
  ): Promise<ToolResult> {
    const { useDocumentStore } = await import('@/stores/document-store')
    const docStore = useDocumentStore.getState()

    if (!args.ids?.length && !args.patterns?.length) {
      const children = docStore.document.children ?? []
      const nodes = children.map((n) => ({
        id: n.id,
        name: n.name,
        type: n.type,
      }))
      return { success: true, data: nodes }
    }

    const results: Record<string, unknown>[] = []
    const seen = new Set<string>()

    if (args.ids?.length) {
      for (const id of args.ids) {
        if (seen.has(id)) continue
        const node = docStore.getNodeById(id)
        if (node) {
          seen.add(id)
          results.push({ ...node })
        }
      }
    }

    if (args.patterns?.length) {
      const flat = docStore.getFlatNodes()
      for (const pattern of args.patterns) {
        const regex = new RegExp(pattern, 'i')
        for (const node of flat) {
          if (seen.has(node.id)) continue
          if (regex.test(node.name ?? '') || regex.test(node.type)) {
            seen.add(node.id)
            results.push({ ...node })
          }
        }
      }
    }

    return { success: true, data: results }
  }

  private async handleSnapshotLayout(
    args: { pageId?: string },
  ): Promise<ToolResult> {
    const { useDocumentStore, getActivePageChildren, getAllChildren } =
      await import('@/stores/document-store')
    const { useCanvasStore } = await import('@/stores/canvas-store')
    const doc = useDocumentStore.getState().document
    const pageId = args.pageId ?? useCanvasStore.getState().activePageId
    const children = getActivePageChildren(doc, pageId)
    const allChildren = getAllChildren(doc)

    const { getNodeBounds } = await import('@/stores/document-tree-utils')

    const buildLayout = (
      nodes: typeof children,
      maxDepth: number,
      depth = 0,
    ): { id: string; name?: string; type: string; x: number; y: number; width: number; height: number; children?: unknown[] }[] =>
      nodes.map((node) => {
        const b = getNodeBounds(node, allChildren)
        const entry: {
          id: string
          name?: string
          type: string
          x: number
          y: number
          width: number
          height: number
          children?: unknown[]
        } = {
          id: node.id,
          name: node.name,
          type: node.type,
          x: b.x,
          y: b.y,
          width: b.w,
          height: b.h,
        }
        if ('children' in node && node.children?.length && depth < maxDepth) {
          entry.children = buildLayout(node.children, maxDepth, depth + 1)
        }
        return entry
      })

    return { success: true, data: buildLayout(children, 1) }
  }

  private async handleFindEmptySpace(
    args: { width: number; height: number; pageId?: string },
  ): Promise<ToolResult> {
    const { useDocumentStore, getActivePageChildren, getAllChildren } =
      await import('@/stores/document-store')
    const { useCanvasStore } = await import('@/stores/canvas-store')
    const { getNodeBounds } = await import('@/stores/document-tree-utils')

    const doc = useDocumentStore.getState().document
    const pageId = args.pageId ?? useCanvasStore.getState().activePageId
    const children = getActivePageChildren(doc, pageId)
    const allChildren = getAllChildren(doc)
    const padding = 50

    if (children.length === 0) {
      return { success: true, data: { x: 0, y: 0 } }
    }

    let minY = Infinity
    let maxX = -Infinity
    for (const node of children) {
      const b = getNodeBounds(node, allChildren)
      if (b.x + b.w > maxX) maxX = b.x + b.w
      if (b.y < minY) minY = b.y
    }

    return { success: true, data: { x: maxX + padding, y: minY } }
  }

  // ---------------------------------------------------------------------------
  // Write tools
  // ---------------------------------------------------------------------------

  /**
   * Insert a node with full support for nested children.
   * After insertion, runs the same post-processing as the MCP batch_design:
   * role resolution, icon resolution, layout sanitization, unique IDs.
   */
  /**
   * Insert a node — aligned with MCP batch_design behavior:
   * 1. Parse stringified data
   * 2. Sanitize invalid properties (border→strokes, etc.)
   * 3. Auto-replace empty root frame (same as batch_design line 146-161)
   * 4. Post-process: role resolution, icon resolution, layout sanitization
   * 5. Auto-zoom to show new design
   */
  private async handleInsertNode(
    args: { parent: string | null; data: Record<string, unknown>; pageId?: string },
  ): Promise<ToolResult> {
    // Prevent duplicate root-level design inserts (common with weaker models)
    if (args.parent === null && this.rootInsertId) {
      return {
        success: true,
        data: {
          id: this.rootInsertId,
          message: `Design already created (id: ${this.rootInsertId}). Use update_node to modify it. Do NOT insert again.`,
        },
      }
    }

    const { useDocumentStore, getActivePageChildren } = await import('@/stores/document-store')
    const { useCanvasStore } = await import('@/stores/canvas-store')
    const { nanoid } = await import('nanoid')
    const docStore = useDocumentStore.getState()

    // Some models send data as a JSON string instead of an object — parse it
    let nodeData = args.data
    if (typeof nodeData === 'string') {
      try { nodeData = JSON.parse(nodeData) } catch {
        return { success: false, error: 'Invalid node data: could not parse JSON string' }
      }
    }

    // Recursively assign IDs and sanitize invalid properties
    const sanitizeAndAssignIds = (data: Record<string, unknown>): PenNode => {
      const n = { ...data, id: nanoid() } as any
      // Convert 'border' → 'strokes' (common model mistake)
      if (n.border && !n.strokes) {
        n.strokes = [n.border]
        delete n.border
      }
      // Ensure children is a valid array
      if (n.children && !Array.isArray(n.children)) {
        delete n.children
      }
      if (Array.isArray(n.children)) {
        n.children = n.children
          .filter((child: unknown) => child != null && typeof child === 'object')
          .map((child: Record<string, unknown>) => sanitizeAndAssignIds(child))
      }
      return n as PenNode
    }

    const node = sanitizeAndAssignIds(nodeData as Record<string, unknown>)

    // Count total nodes
    const countNodes = (n: any): number => {
      let c = 1
      if (Array.isArray(n.children)) for (const ch of n.children) c += countNodes(ch)
      return c
    }
    const totalNodes = countNodes(node)

    // --- Auto-replace empty root frame (matches batch_design behavior) ---
    // Uses setActivePageChildren for atomic tree swap — same as batch_design.
    // This avoids updateNode side-effect issues and duplicate key errors.
    let replaced = false
    if (args.parent === null && (nodeData as any).type === 'frame') {
      try {
        const { setActivePageChildren } = await import('@/stores/document-tree-utils')
        const { removeNodeFromTree, insertNodeInTree } = await import('@/stores/document-tree-utils')
        const doc = docStore.document
        const pageId = args.pageId ?? useCanvasStore.getState().activePageId
        const children = getActivePageChildren(doc, pageId)
        const emptyIdx = children.findIndex(
          (n) => n.type === 'frame' && (!('children' in n) || !n.children || n.children.length === 0),
        )
        if (emptyIdx !== -1) {
          const emptyFrame = children[emptyIdx]
          // Inherit position from the empty frame
          if (emptyFrame.x !== undefined) (node as any).x = emptyFrame.x
          if (emptyFrame.y !== undefined) (node as any).y = emptyFrame.y
          // Atomic tree swap: remove empty + insert new in one setState
          let updated = removeNodeFromTree(children, emptyFrame.id)
          updated = insertNodeInTree(updated, null, node, emptyIdx)
          useDocumentStore.setState({
            document: setActivePageChildren(doc, pageId, updated),
            isDirty: true,
          })
          replaced = true
        }
      } catch { /* fallback to normal insert */ }
    }

    if (!replaced) {
      try {
        docStore.addNode(args.parent, node)
      } catch { /* side-effect error */ }
    }

    // Track root-level insert to prevent duplicates
    if (args.parent === null) {
      this.rootInsertId = node.id
    }

    // Run post-processing (same pipeline as MCP batch_design with postProcess=true)
    try {
      await this.postProcessNode(node.id)
    } catch { /* best-effort */ }

    // Auto-zoom to show the new design
    try {
      const { zoomToFitContent } = await import('@/canvas/skia-engine-ref')
      setTimeout(() => zoomToFitContent(), 200)
    } catch { /* ignore */ }

    return {
      success: true,
      data: {
        id: node.id,
        nodesCreated: totalNodes,
        message: `Created ${totalNodes} nodes successfully. Do NOT retry or create again.`,
      },
    }
  }

  /**
   * Post-process an inserted node tree — same pipeline as MCP batch_design:
   * 1. Role resolution (semantic defaults for buttons, cards, etc.)
   * 2. Icon resolution (icon names → SVG paths)
   * 3. Layout sanitization (remove x/y from layout children)
   * 4. Unique ID enforcement
   */
  private async postProcessNode(nodeId: string): Promise<void> {
    const { useDocumentStore, getAllChildren } = await import('@/stores/document-store')
    const docStore = useDocumentStore.getState()
    const node = docStore.getNodeById(nodeId)
    if (!node || !('children' in node) || !node.children?.length) return

    const { flattenNodes } = await import('@/stores/document-tree-utils')

    // Deep clone — Zustand state is immutable, post-processing mutates in-place
    const target = JSON.parse(JSON.stringify(node)) as PenNode

    // Determine canvas width (mobile: 375, desktop: 1200)
    const isMobile = (target as any).width <= 500
    const canvasWidth = isMobile ? 375 : 1200

    // 1. Role resolution (best-effort — some models produce unusual structures)
    try {
      const { resolveTreeRoles, resolveTreePostPass } =
        await import('@/services/ai/role-resolver')
      await import('@/services/ai/role-definitions/index')
      resolveTreeRoles(target, canvasWidth)
      resolveTreePostPass(target, canvasWidth)
    } catch { /* skip role resolution on error */ }

    // 2. Icon + emoji resolution
    try {
      const { applyIconPathResolution, applyNoEmojiIconHeuristic } =
        await import('@/services/ai/icon-resolver')
      const flat = flattenNodes([target])
      for (const n of flat) {
        if (n.type === 'path') applyIconPathResolution(n as any)
        if (n.type === 'text') applyNoEmojiIconHeuristic(n as any)
      }
    } catch { /* skip icon resolution on error */ }

    // 3. Unique IDs
    try {
      const { ensureUniqueNodeIds } = await import('@/services/ai/design-node-sanitization')
      const allChildren = getAllChildren(docStore.document) ?? []
      const usedIds = new Set(flattenNodes(allChildren).map((n) => n.id))
      const idCounters = new Map<string, number>()
      ensureUniqueNodeIds(target, usedIds, idCounters)
    } catch { /* skip ID dedup on error */ }

    // 4. Layout sanitization
    try {
      const { sanitizeLayoutChildPositions } = await import('@/services/ai/design-node-sanitization')
      sanitizeLayoutChildPositions(target, false)
    } catch { /* skip layout sanitization on error */ }

    // Apply the processed node back
    docStore.updateNode(nodeId, { children: (target as any).children } as any)
  }

  private async handleUpdateNode(
    args: { id: string; data: Record<string, unknown> },
  ): Promise<ToolResult> {
    const { useDocumentStore } = await import('@/stores/document-store')
    const docStore = useDocumentStore.getState()
    const existing = docStore.getNodeById(args.id)
    if (!existing) {
      return { success: false, error: `Node not found: ${args.id}` }
    }
    docStore.updateNode(args.id, args.data as Partial<PenNode>)
    return { success: true }
  }

  private async handleDeleteNode(args: { id: string }): Promise<ToolResult> {
    const { useDocumentStore } = await import('@/stores/document-store')
    const docStore = useDocumentStore.getState()
    const existing = docStore.getNodeById(args.id)
    if (!existing) {
      return { success: false, error: `Node not found: ${args.id}` }
    }
    docStore.removeNode(args.id)
    return { success: true }
  }
}
