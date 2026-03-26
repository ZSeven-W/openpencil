import type { AgentEvent, ToolResult, AuthLevel } from '@zseven-w/agent'

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

  constructor(sessionId: string) {
    this.sessionId = sessionId
  }

  async execute(toolCall: ToolCallEvent): Promise<void> {
    const { id, name, args, level } = toolCall
    const isWrite = WRITE_LEVELS.has(level)

    // Wrap write operations in an undo batch so the entire tool call is a
    // single undo step (the store methods call pushState internally, which
    // becomes a no-op while a batch is active).
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

    // POST result back to the server to unblock the agent loop
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
      // Return top-level children summary when no filters given
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

    // Search by IDs
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

    // Search by name patterns
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

    // Compute combined bounding box, then place to the right (matching MCP default "right" direction)
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

  private async handleInsertNode(
    args: { parent: string | null; data: Record<string, unknown>; pageId?: string },
  ): Promise<ToolResult> {
    const { useDocumentStore } = await import('@/stores/document-store')
    const { nanoid } = await import('nanoid')
    const docStore = useDocumentStore.getState()
    const node = { ...args.data, id: args.data.id ?? nanoid() } as import('@/types/pen').PenNode
    docStore.addNode(args.parent, node)
    return { success: true, data: { id: node.id } }
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
    docStore.updateNode(args.id, args.data as Partial<import('@/types/pen').PenNode>)
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
