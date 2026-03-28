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
      result = { success: false, error: (err instanceof Error ? err.message : JSON.stringify(err)) }
    }

    if (isWrite) {
      const { useHistoryStore } = await import('@/stores/history-store')
      const { useDocumentStore } = await import('@/stores/document-store')
      useHistoryStore.getState().endBatch(useDocumentStore.getState().document)
    }

    // Post result back to server to unblock the agent loop.
    // Retry once on failure — if the POST is lost, the agent hangs.
    const payload = JSON.stringify({ sessionId: this.sessionId, toolCallId: id, result })
    const postHeaders = { 'Content-Type': 'application/json' }
    try {
      const res = await fetch('/api/ai/agent?action=result', { method: 'POST', headers: postHeaders, body: payload })
      if (!res.ok) throw new Error(`Status ${res.status}`)
    } catch {
      // Retry once
      try {
        await fetch('/api/ai/agent?action=result', { method: 'POST', headers: postHeaders, body: payload })
      } catch (retryErr) {
        console.error(`[AgentToolExecutor] Failed to post tool result ${id}:`, retryErr)
      }
    }
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
      case 'generate_design':
        return this.handleGenerateDesign(args as { prompt: string; canvasWidth?: number })
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
  // generate_design — calls the SAME internal pipeline as the chat design flow
  // ---------------------------------------------------------------------------

  /**
   * Generate a design using the EXISTING internal pipeline (orchestrator → sub-agents
   * → insertStreamingNode). This is the same path that works with M2.5 and all models
   * through the standard chat interface. The agent just provides the prompt.
   */
  private async handleGenerateDesign(
    args: { prompt?: string; description?: string; canvasWidth?: number },
  ): Promise<ToolResult> {
    // Some models use 'description' instead of 'prompt'
    const prompt = args.prompt || args.description
    if (!prompt) return { success: false, error: 'Missing prompt or description' }
    if (this.rootInsertId) {
      return {
        success: true,
        data: { message: `Design already created. Use update_node to modify.` },
      }
    }

    const { generateDesign } = await import('@/services/ai/design-generator')
    const { useDocumentStore } = await import('@/stores/document-store')
    const { useAgentSettingsStore } = await import('@/stores/agent-settings-store')
    const { getCanvasSize } = await import('@/canvas/skia-engine-ref')

    const docStore = useDocumentStore.getState()
    const canvasSize = getCanvasSize()

    // Find a provider for the internal pipeline:
    // 1. First try connected CLI providers (Claude Code, Codex, etc.)
    // 2. Fall back to the current builtin provider (API key)
    const agentSettings = useAgentSettingsStore.getState()
    const providers = agentSettings.providers ?? {}
    let designModel = 'default'
    let designProvider: string | undefined

    for (const [key, cfg] of Object.entries(providers)) {
      if (cfg.isConnected && cfg.models?.length) {
        designProvider = key
        designModel = cfg.models[0].value
        break
      }
    }

    // If no CLI provider connected, use builtin provider via the 'builtin' provider path.
    // The chat endpoint reads builtin credentials from the request body (set by ai-service.ts).
    if (!designProvider) {
      const { useAIStore } = await import('@/stores/ai-store')
      const currentModel = useAIStore.getState().model
      if (currentModel.startsWith('builtin:')) {
        const parts = currentModel.split(':')
        const bpId = parts[1]
        const bp = agentSettings.builtinProviders.find((p) => p.id === bpId)
        if (bp?.apiKey) {
          designProvider = 'builtin' as any
          designModel = parts.slice(2).join(':')
        }
      }
    }

    // Mark as in-progress BEFORE calling generateDesign to prevent duplicate calls.
    // If the first call fails midway, partial nodes are cleaned up below.
    this.rootInsertId = 'generating'

    // Snapshot current node IDs so we can clean up partial nodes on failure
    const nodeIdsBefore = new Set(docStore.getFlatNodes().map(n => n.id))

    // Match the CLI pipeline's generateDesign call exactly
    const { useAIStore: aiStore } = await import('@/stores/ai-store')
    const concurrency = aiStore.getState().concurrency
    const doc = docStore.document
    let designMd: any
    try {
      const { useDesignMdStore } = await import('@/stores/design-md-store')
      designMd = useDesignMdStore?.getState()?.designMd
    } catch { /* store may not exist */ }

    let result: { nodes: unknown[] }
    try {
      result = await generateDesign(
        {
          prompt,
          model: designModel,
          provider: designProvider as any,
          concurrency,
          context: {
            canvasSize,
            documentSummary: `Document has ${docStore.getFlatNodes().length} nodes`,
            variables: doc.variables,
            themes: doc.themes,
            designMd,
          },
        },
        {
          onApplyPartial: () => {},
          onTextUpdate: () => {},
          animated: true,
        },
      )
    } catch (err) {
      // Clean up partial nodes inserted before the failure
      const currentNodes = docStore.getFlatNodes()
      const newNodes = currentNodes.filter(n => !nodeIdsBefore.has(n.id))
      for (const n of newNodes) {
        try { docStore.removeNode(n.id) } catch { /* ignore */ }
      }
      this.rootInsertId = null // Allow retry
      return { success: false, error: `Design generation failed: ${(err instanceof Error ? err.message : JSON.stringify(err))}` }
    }

    this.rootInsertId = 'generated'

    // Auto-zoom
    try {
      const { zoomToFitContent } = await import('@/canvas/skia-engine-ref')
      setTimeout(() => zoomToFitContent(), 300)
    } catch { /* ignore */ }

    return {
      success: true,
      data: {
        nodeCount: result.nodes.length,
        message: `Design generated with ${result.nodes.length} nodes via internal pipeline. Do NOT retry.`,
      },
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

    const { nanoid } = await import('nanoid')

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

    // Use insertStreamingNode — the SAME function the CLI streaming pipeline uses.
    // MUST call resetGenerationRemapping() first to initialize generation state
    // (preExistingNodeIds, generationRootFrameId, generationRemappedIds).
    // Without this, insertStreamingNode's ID dedup and parent resolution break.
    const { insertStreamingNode, resetGenerationRemapping, setGenerationCanvasWidth } =
      await import('@/services/ai/design-canvas-ops')
    resetGenerationRemapping()
    // Set canvas width for role resolution (mobile: 375, desktop: 1200)
    const isMobile = (node as any).width && (node as any).width <= 500
    setGenerationCanvasWidth(isMobile ? 375 : 1200)
    const insertRecursive = (n: PenNode, parentId: string | null) => {
      const children = ('children' in n && Array.isArray(n.children)) ? [...n.children] : []
      const nodeForInsert = { ...n } as PenNode
      if (children.length > 0) {
        ;(nodeForInsert as any).children = []
      }
      insertStreamingNode(nodeForInsert, parentId)
      // Use nodeForInsert.id (not n.id) — ensureUniqueNodeIds inside
      // insertStreamingNode may have renamed it, and replaceEmptyFrame
      // maps from the renamed ID to root-frame via generationRemappedIds.
      const actualId = nodeForInsert.id
      for (const child of children) {
        insertRecursive(child, actualId)
      }
    }
    insertRecursive(node, args.parent)

    // Track root-level insert to prevent duplicates
    if (args.parent === null) {
      this.rootInsertId = node.id
    }

    // Auto-zoom to show the new design
    try {
      const { zoomToFitContent } = await import('@/canvas/skia-engine-ref')
      setTimeout(() => zoomToFitContent(), 300)
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
