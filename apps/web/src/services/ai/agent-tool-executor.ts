import type { AgentEvent, ToolResult, AuthLevel } from '@/types/agent';
import type { PenNode } from '@/types/pen';
import { createEmptyDocument, DEFAULT_FRAME_ID, useDocumentStore } from '@/stores/document-store';
import { detectAppendIntent } from './append-intent-detector';

type ToolCallEvent = Extract<AgentEvent, { type: 'tool_call' }>;

/** Auth levels that mutate the document and should be wrapped in an undo batch. */
const WRITE_LEVELS: Set<AuthLevel> = new Set(['create', 'modify', 'delete']);

/**
 * Client-side tool executor.
 *
 * Receives `tool_call` events from the SSE stream, dispatches them against the
 * live Zustand document store, wraps write operations in an undo batch, and
 * POSTs the result back to the server to unblock the agent loop.
 */
export class AgentToolExecutor {
  private sessionId: string;
  private designGenerated = false;
  private layoutPhase: 'idle' | 'layout_done' | 'content_started' = 'idle';
  private layoutRootId: string | null = null;

  constructor(sessionId: string) {
    this.sessionId = sessionId;
  }

  async execute(toolCall: ToolCallEvent): Promise<ToolResult> {
    const { id, name, args, level } = toolCall;
    const isWrite = WRITE_LEVELS.has(level);

    if (isWrite) {
      const { useHistoryStore } = await import('@/stores/history-store');
      const { useDocumentStore } = await import('@/stores/document-store');
      useHistoryStore.getState().startBatch(useDocumentStore.getState().document);
    }

    let result: ToolResult;
    try {
      result = await this.dispatch(name, args);
    } catch (err) {
      result = { success: false, error: err instanceof Error ? err.message : JSON.stringify(err) };
    }

    if (isWrite) {
      const { useHistoryStore } = await import('@/stores/history-store');
      const { useDocumentStore } = await import('@/stores/document-store');
      useHistoryStore.getState().endBatch(useDocumentStore.getState().document);
    }

    // Post result back to server to unblock the agent loop.
    // Retry once on failure — if the POST is lost, the agent hangs.
    const payload = JSON.stringify({ sessionId: this.sessionId, toolCallId: id, result });
    const postHeaders = { 'Content-Type': 'application/json' };
    try {
      const res = await fetch('/api/ai/agent?action=result', {
        method: 'POST',
        headers: postHeaders,
        body: payload,
      });
      if (!res.ok) throw new Error(`Status ${res.status}`);
    } catch {
      // Retry once
      try {
        const retryRes = await fetch('/api/ai/agent?action=result', {
          method: 'POST',
          headers: postHeaders,
          body: payload,
        });
        if (!retryRes.ok) throw new Error(`Status ${retryRes.status}`);
      } catch (retryErr) {
        console.error(`[AgentToolExecutor] Failed to post tool result ${id}:`, retryErr);
        throw retryErr instanceof Error ? retryErr : new Error(`Failed to post tool result ${id}`);
      }
    }

    return result;
  }

  // ---------------------------------------------------------------------------
  // Tool dispatch
  // ---------------------------------------------------------------------------

  private async dispatch(name: string, args: unknown): Promise<ToolResult> {
    switch (name) {
      case 'batch_get':
        return this.handleBatchGet(args as { ids?: string[]; patterns?: string[] });
      case 'snapshot_layout':
        return this.handleSnapshotLayout(args as { pageId?: string });
      case 'generate_design':
        return this.handleGenerateDesign(args as { prompt: string; canvasWidth?: number });
      case 'plan_layout':
        return this.handlePlanLayout(args as { prompt: string; newRoot?: boolean });
      case 'batch_insert':
        return this.handleBatchInsert(args as { parentId: string | null; nodes: unknown[] });
      case 'insert_node':
        return this.handleInsertNode(
          args as {
            parent?: string | null;
            after?: string;
            data: Record<string, unknown>;
            pageId?: string;
          },
        );
      case 'update_node':
        return this.handleUpdateNode(args as { id: string; data: Record<string, unknown> });
      case 'move_node':
        return this.handleMoveNode(args as { id: string; parent: string; index?: number });
      case 'delete_node':
        return this.handleDeleteNode(args as { id: string });
      case 'find_empty_space':
        return this.handleFindEmptySpace(
          args as { width: number; height: number; pageId?: string },
        );
      case 'get_selection':
        return this.handleGetSelection();
      default:
        return { success: false, error: `Unknown tool: ${name}` };
    }
  }

  private markContentStarted(): void {
    this.layoutPhase = 'content_started';
  }

  // ---------------------------------------------------------------------------
  // generate_design — calls the SAME internal pipeline as the chat design flow
  // ---------------------------------------------------------------------------

  /**
   * Generate a design using the EXISTING internal pipeline (orchestrator → sub-agents
   * → insertStreamingNode). This is the same path that works with M2.5 and all models
   * through the standard chat interface. The agent just provides the prompt.
   */
  private async handleGenerateDesign(args: {
    prompt?: string;
    description?: string;
    canvasWidth?: number;
  }): Promise<ToolResult> {
    // Prevent duplicate calls — some models call generate_design multiple times
    if (this.designGenerated) {
      return {
        success: true,
        data: { message: 'Design already created. Do not call generate_design again.' },
      };
    }

    // Some models use 'description' instead of 'prompt'
    const prompt = args.prompt || args.description;
    if (!prompt) {
      return {
        success: false,
        error:
          'Missing prompt parameter. You MUST call generate_design with: {"prompt": "your design description"}. ' +
          'Copy the full design task into the prompt field. Example: generate_design({"prompt": "a mobile login screen with email, password fields and login button"})',
      };
    }

    const { useAIStore } = await import('@/stores/ai-store');
    const currentModel = useAIStore.getState().model;

    // Full orchestrator pipeline — same path for CLI and builtin providers
    const { generateDesign } = await import('@/services/ai/design-generator');
    const { useDocumentStore: docStoreModule } = await import('@/stores/document-store');
    const { useAgentSettingsStore } = await import('@/stores/agent-settings-store');
    const { getCanvasSize } = await import('@/canvas/skia-engine-ref');
    const { getGenerationRemappedIds } = await import('@/services/ai/design-canvas-ops');

    const docStore = docStoreModule.getState();
    const canvasSize = getCanvasSize();

    const agentSettings = useAgentSettingsStore.getState();
    let designModel = 'default';
    let designProvider: string | undefined;

    // For builtin providers, resolve to the actual model name and provider type
    if (currentModel.startsWith('builtin:')) {
      const bpId = currentModel.split(':')[1];
      const actualModel = currentModel.split(':').slice(2).join(':');
      const bp = agentSettings.builtinProviders.find((p: any) => p.id === bpId);
      if (bp) {
        designProvider = 'builtin';
        designModel = actualModel || currentModel;
      }
    }

    if (!designProvider) {
      const currentProvider = useAIStore
        .getState()
        .modelGroups.find((g) => g.models.some((m) => m.value === currentModel))?.provider;

      if (currentProvider) {
        designProvider = currentProvider;
        designModel = currentModel;
      } else {
        const providers = agentSettings.providers ?? {};
        for (const [key, cfg] of Object.entries(providers)) {
          if (cfg.isConnected && cfg.models?.length) {
            designProvider = key;
            designModel = cfg.models[0].value;
            break;
          }
        }
      }
    }

    // Snapshot current node IDs so we can clean up partial nodes on failure
    const nodeIdsBefore = new Set(docStore.getFlatNodes().map((n) => n.id));
    const remappedIdsBefore = new Map(getGenerationRemappedIds());

    // Match the CLI pipeline's generateDesign call exactly
    const { useAIStore: aiStore } = await import('@/stores/ai-store');
    const concurrency = aiStore.getState().concurrency;
    const doc = docStore.document;
    let designMd: any;
    try {
      const { useDesignMdStore } = await import('@/stores/design-md-store');
      designMd = useDesignMdStore?.getState()?.designMd;
    } catch {
      /* store may not exist */
    }

    // Store orchestrator progress in a dedicated store field (not in message content)
    // so it survives agent text streaming after tool completion.
    const { useAIStore: progressStore } = await import('@/stores/ai-store');
    progressStore.getState().setAgentOrchestrationSteps(null); // reset

    const updateProgress = (text: string) => {
      progressStore.getState().setAgentOrchestrationSteps(text);
    };

    const activePageId = (await import('@/stores/canvas-store')).useCanvasStore.getState()
      .activePageId;
    const appendContext = detectAppendIntent(prompt, doc, activePageId);

    let result: { nodes: unknown[] };
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
            ...(appendContext ? { appendContext } : {}),
          },
        },
        {
          onApplyPartial: () => {},
          onTextUpdate: updateProgress,
          animated: true,
        },
      );
    } catch (err) {
      // Clean up partial nodes inserted before the failure
      const currentNodes = docStore.getFlatNodes();
      const newNodes = currentNodes.filter((n) => !nodeIdsBefore.has(n.id));
      for (const n of newNodes) {
        try {
          docStore.removeNode(n.id);
        } catch {
          /* ignore */
        }
      }

      const remappedIdsAfter = getGenerationRemappedIds();
      const hadDefaultReplacementBefore = [...remappedIdsBefore.values()].includes(
        DEFAULT_FRAME_ID,
      );
      const hasDefaultReplacementAfter = [...remappedIdsAfter.values()].includes(DEFAULT_FRAME_ID);
      if (hasDefaultReplacementAfter && !hadDefaultReplacementBefore) {
        this.restoreDefaultFrame();
      }

      // Clear stale progress so checklist doesn't show partial steps from a failed run
      progressStore.getState().setAgentOrchestrationSteps(null);
      return {
        success: false,
        error: `Design generation failed: ${err instanceof Error ? err.message : JSON.stringify(err)}`,
      };
    }

    // Auto-zoom
    try {
      const { zoomToFitContent } = await import('@/canvas/skia-engine-ref');
      setTimeout(() => zoomToFitContent(), 300);
    } catch {
      /* ignore */
    }

    // Mark all steps as done in the stored progress
    const currentSteps = progressStore.getState().agentOrchestrationSteps;
    if (currentSteps) {
      const allDone = currentSteps.replace(/status="(streaming|pending)"/g, 'status="done"');
      progressStore.getState().setAgentOrchestrationSteps(allDone);
    }

    this.designGenerated = true;

    return {
      success: true,
      data: {
        nodeCount: result.nodes.length,
      },
    };
  }

  private restoreDefaultFrame(): void {
    const docStore = useDocumentStore.getState();
    const emptyDoc = createEmptyDocument();
    const defaultFrame = emptyDoc.pages?.[0]?.children.find((n) => n.id === DEFAULT_FRAME_ID);
    if (!defaultFrame) return;

    docStore.updateNode(DEFAULT_FRAME_ID, {
      ...defaultFrame,
      id: DEFAULT_FRAME_ID,
    } as Partial<PenNode>);
  }

  // ---------------------------------------------------------------------------
  // Read tools
  // ---------------------------------------------------------------------------

  private async handleBatchGet(args: { ids?: string[]; patterns?: string[] }): Promise<ToolResult> {
    const { useDocumentStore } = await import('@/stores/document-store');
    const docStore = useDocumentStore.getState();

    if (!args.ids?.length && !args.patterns?.length) {
      const children = docStore.document.children ?? [];
      const nodes = children.map((n) => ({
        id: n.id,
        name: n.name,
        type: n.type,
      }));
      return { success: true, data: nodes };
    }

    const results: Record<string, unknown>[] = [];
    const seen = new Set<string>();

    if (args.ids?.length) {
      for (const id of args.ids) {
        if (seen.has(id)) continue;
        const node = docStore.getNodeById(id);
        if (node) {
          seen.add(id);
          results.push({ ...node });
        }
      }
    }

    if (args.patterns?.length) {
      const flat = docStore.getFlatNodes();
      for (const pattern of args.patterns) {
        const regex = new RegExp(pattern, 'i');
        for (const node of flat) {
          if (seen.has(node.id)) continue;
          if (regex.test(node.name ?? '') || regex.test(node.type)) {
            seen.add(node.id);
            results.push({ ...node });
          }
        }
      }
    }

    return { success: true, data: results };
  }

  private async handleSnapshotLayout(args: { pageId?: string }): Promise<ToolResult> {
    const { useDocumentStore, getActivePageChildren, getAllChildren } =
      await import('@/stores/document-store');
    const { useCanvasStore } = await import('@/stores/canvas-store');
    const doc = useDocumentStore.getState().document;
    const pageId = args.pageId ?? useCanvasStore.getState().activePageId;
    const children = getActivePageChildren(doc, pageId);
    const allChildren = getAllChildren(doc);

    // Prefer SkiaEngine's layout-computed bounds (absX/absY/absW/absH) over raw
    // stored values. getNodeBounds returns stored width/height which may be defaults
    // (100) for auto-layout frames — misleading for the model.
    const { getSkiaEngineRef } = await import('@/canvas/skia-engine-ref');
    const engine = getSkiaEngineRef();
    const renderNodeMap = new Map<string, { x: number; y: number; w: number; h: number }>();
    if (engine?.renderNodes) {
      for (const rn of engine.renderNodes) {
        renderNodeMap.set(rn.node.id, { x: rn.absX, y: rn.absY, w: rn.absW, h: rn.absH });
      }
    }

    const { getNodeBounds } = await import('@/stores/document-tree-utils');

    const buildLayout = (
      nodes: typeof children,
      maxDepth: number,
      depth = 0,
    ): {
      id: string;
      name?: string;
      type: string;
      x: number;
      y: number;
      width: number;
      height: number;
      children?: unknown[];
    }[] =>
      nodes.map((node) => {
        // Use layout-computed bounds from SkiaEngine, fall back to stored values
        const computed = renderNodeMap.get(node.id);
        const b = computed ?? getNodeBounds(node, allChildren);
        const entry: {
          id: string;
          name?: string;
          type: string;
          x: number;
          y: number;
          width: number;
          height: number;
          children?: unknown[];
        } = {
          id: node.id,
          name: node.name,
          type: node.type,
          x: Math.round(b.x),
          y: Math.round(b.y),
          width: Math.round(b.w),
          height: Math.round(b.h),
        };
        if ('children' in node && node.children?.length && depth < maxDepth) {
          entry.children = buildLayout(node.children, maxDepth, depth + 1);
        }
        return entry;
      });

    return { success: true, data: buildLayout(children, 3) };
  }

  private async handleFindEmptySpace(args: {
    width: number;
    height: number;
    pageId?: string;
  }): Promise<ToolResult> {
    const { useDocumentStore, getActivePageChildren, getAllChildren } =
      await import('@/stores/document-store');
    const { useCanvasStore } = await import('@/stores/canvas-store');
    const { getNodeBounds } = await import('@/stores/document-tree-utils');

    const doc = useDocumentStore.getState().document;
    const pageId = args.pageId ?? useCanvasStore.getState().activePageId;
    const children = getActivePageChildren(doc, pageId);
    const allChildren = getAllChildren(doc);
    const padding = 50;

    if (children.length === 0) {
      return { success: true, data: { x: 0, y: 0 } };
    }

    let minY = Infinity;
    let maxX = -Infinity;
    for (const node of children) {
      const b = getNodeBounds(node, allChildren);
      if (b.x + b.w > maxX) maxX = b.x + b.w;
      if (b.y < minY) minY = b.y;
    }

    return { success: true, data: { x: maxX + padding, y: minY } };
  }

  private async handleGetSelection(): Promise<ToolResult> {
    const { useCanvasStore } = await import('@/stores/canvas-store');
    const { useDocumentStore } = await import('@/stores/document-store');
    const selectedIds = useCanvasStore.getState().selection.selectedIds;
    if (selectedIds.length === 0) {
      return { success: true, data: [] };
    }
    const docStore = useDocumentStore.getState();
    const nodes = selectedIds
      .map((id) => docStore.getNodeById(id))
      .filter(Boolean)
      .map((n) => ({ ...n }));
    return { success: true, data: nodes };
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
  private async handleInsertNode(args: {
    parent?: string | null;
    after?: string;
    data: Record<string, unknown>;
    pageId?: string;
  }): Promise<ToolResult> {
    const { nanoid } = await import('nanoid');

    // Some models send data as a JSON string instead of an object — parse it
    let nodeData = args.data;
    if (typeof nodeData === 'string') {
      try {
        nodeData = JSON.parse(nodeData);
      } catch {
        return { success: false, error: 'Invalid node data: could not parse JSON string' };
      }
    }

    // Recursively assign IDs and sanitize invalid properties
    const sanitizeAndAssignIds = (data: Record<string, unknown>): PenNode => {
      const n = { ...data, id: nanoid() } as any;
      // Convert 'border' → 'strokes' (common model mistake)
      if (n.border && !n.strokes) {
        n.strokes = [n.border];
        delete n.border;
      }
      // Ensure children is a valid array
      if (n.children && !Array.isArray(n.children)) {
        delete n.children;
      }
      if (Array.isArray(n.children)) {
        n.children = n.children
          .filter((child: unknown) => child != null && typeof child === 'object')
          .map((child: Record<string, unknown>) => sanitizeAndAssignIds(child));
      }
      return n as PenNode;
    };

    const node = sanitizeAndAssignIds(nodeData as Record<string, unknown>);

    // Count total nodes
    const countNodes = (n: any): number => {
      let c = 1;
      if (Array.isArray(n.children)) for (const ch of n.children) c += countNodes(ch);
      return c;
    };
    const totalNodes = countNodes(node);

    const { useDocumentStore } = await import('@/stores/document-store');
    const { findParentInTree, getActivePageChildren } =
      await import('@/stores/document-tree-utils');
    const { useCanvasStore } = await import('@/stores/canvas-store');
    const docStore = useDocumentStore.getState();

    // Resolve "after" → parent + index
    let parentId: string | null = args.parent ?? null;
    let insertIndex: number | undefined;

    if (args.after) {
      const doc = docStore.document;
      const activePageId = useCanvasStore.getState().activePageId;
      const children = getActivePageChildren(doc, activePageId);
      const parent = findParentInTree(children, args.after);
      if (parent) {
        parentId = parent.id;
        const siblings =
          'children' in parent && Array.isArray(parent.children) ? parent.children : [];
        const siblingIdx = siblings.findIndex((n) => n.id === args.after);
        if (siblingIdx >= 0) insertIndex = siblingIdx + 1;
      }
    }

    // When inserting into an existing parent, use addNode directly — simple and reliable.
    // Only fall back to insertStreamingNode for root-level generation (parent=null)
    // where we need the full pipeline (replace empty frame, role resolution, etc.).
    if (parentId && docStore.getNodeById(parentId)) {
      const parentNode = docStore.getNodeById(parentId)!;
      // Strip absolute x/y if parent has auto-layout — let the layout engine position
      if ('layout' in parentNode && parentNode.layout && parentNode.layout !== 'none') {
        if ('x' in node) delete (node as { x?: number }).x;
        if ('y' in node) delete (node as { y?: number }).y;
      }
      docStore.addNode(parentId, node, insertIndex);
    } else {
      // Root-level insert or unknown parent — use streaming pipeline
      const { insertStreamingNode, resetGenerationRemapping, setGenerationCanvasWidth } =
        await import('@/services/ai/design-canvas-ops');
      resetGenerationRemapping();
      const isMobile = (node as any).width && (node as any).width <= 500;
      setGenerationCanvasWidth(isMobile ? 375 : 1200);
      const insertRecursive = (n: PenNode, pid: string | null) => {
        const ch = 'children' in n && Array.isArray(n.children) ? [...n.children] : [];
        const nodeForInsert = { ...n } as PenNode;
        if (ch.length > 0) (nodeForInsert as any).children = [];
        insertStreamingNode(nodeForInsert, pid);
        const actualId = nodeForInsert.id;
        for (const child of ch) insertRecursive(child, actualId);
      };
      insertRecursive(node, parentId);
    }
    this.markContentStarted();

    // Auto-zoom to show the new design
    try {
      const { zoomToFitContent } = await import('@/canvas/skia-engine-ref');
      setTimeout(() => zoomToFitContent(), 300);
    } catch {
      /* ignore */
    }

    return {
      success: true,
      data: {
        id: node.id,
        nodesCreated: totalNodes,
        message: `Created ${totalNodes} nodes successfully.`,
      },
    };
  }

  private async handleUpdateNode(args: {
    id: string;
    data: Record<string, unknown>;
  }): Promise<ToolResult> {
    const { useDocumentStore } = await import('@/stores/document-store');
    const docStore = useDocumentStore.getState();
    const existing = docStore.getNodeById(args.id);
    if (!existing) {
      return { success: false, error: `Node not found: ${args.id}` };
    }
    docStore.updateNode(args.id, args.data as Partial<PenNode>);
    return { success: true };
  }

  private async handleMoveNode(args: {
    id: string;
    parent: string;
    index?: number;
  }): Promise<ToolResult> {
    const { useDocumentStore } = await import('@/stores/document-store');
    const docStore = useDocumentStore.getState();
    if (!docStore.getNodeById(args.id)) {
      return { success: false, error: `Node not found: ${args.id}` };
    }
    if (!docStore.getNodeById(args.parent)) {
      return { success: false, error: `Parent not found: ${args.parent}` };
    }
    docStore.moveNode(args.id, args.parent, args.index ?? -1);
    return { success: true };
  }

  private async handleDeleteNode(args: { id: string }): Promise<ToolResult> {
    const { useDocumentStore } = await import('@/stores/document-store');
    const docStore = useDocumentStore.getState();
    const existing = docStore.getNodeById(args.id);
    if (!existing) {
      return { success: false, error: `Node not found: ${args.id}` };
    }
    docStore.removeNode(args.id);
    return { success: true };
  }

  // ---------------------------------------------------------------------------
  // plan_layout — heuristic layout planning (no API call)
  // ---------------------------------------------------------------------------

  private async handlePlanLayout(args: { prompt: string; newRoot?: boolean }): Promise<ToolResult> {
    if (this.layoutPhase !== 'idle' && this.layoutRootId && !args.newRoot) {
      return {
        success: false,
        data: { rootFrameId: this.layoutRootId },
        error:
          `Layout already exists for this session (rootFrameId: ${this.layoutRootId}). ` +
          'Use batch_insert or insert_node to add content to the existing frame. ' +
          'Only call plan_layout again with {"prompt": "...", "newRoot": true} if you intentionally want another root frame or artboard.',
      };
    }

    const { detectDesignType } = await import('@/services/ai/design-type-presets');
    const { useDocumentStore, getActivePageChildren } = await import('@/stores/document-store');
    const { useCanvasStore } = await import('@/stores/canvas-store');
    const { nanoid } = await import('nanoid');

    const preset = detectDesignType(args.prompt);
    const docStore = useDocumentStore.getState();

    // Reuse existing root frame if one exists (avoid duplicate root frames),
    // unless the caller explicitly asked for a brand-new root via newRoot.
    const activePageId = useCanvasStore.getState().activePageId;
    const pageChildren = getActivePageChildren(docStore.document, activePageId);
    const existingFrame = args.newRoot
      ? undefined
      : pageChildren.find((n: any) => n.type === 'frame');
    let rootId: string;
    if (existingFrame) {
      rootId = existingFrame.id;
      // Resize existing frame to match preset
      docStore.updateNode(rootId, {
        width: preset.width,
        height: preset.rootHeight || preset.height,
        layout: 'vertical',
        gap: 0,
        fill: [{ type: 'solid' as const, color: '#F8FAFC' }],
      } as any);
    } else {
      rootId = nanoid(10);
      const rootNode = {
        id: rootId,
        type: 'frame' as const,
        name: 'Page',
        x: 50,
        y: 50,
        width: preset.width,
        height: preset.rootHeight || preset.height,
        layout: 'vertical' as const,
        gap: 0,
        fill: [{ type: 'solid' as const, color: '#F8FAFC' }],
        children: [],
      };
      docStore.addNode(null, rootNode as any);
    }

    this.layoutPhase = 'layout_done';
    this.layoutRootId = rootId;

    return {
      success: true,
      data: {
        rootFrameId: rootId,
        width: preset.width,
        height: preset.rootHeight || preset.height,
        sections: preset.defaultSections,
        message: `Root frame created (${preset.width}x${preset.rootHeight || preset.height}). Now generate PenNode JSON for each section and use batch_insert or insert_node to add them. Sections: ${preset.defaultSections.join(', ')}`,
      },
    };
  }

  // ---------------------------------------------------------------------------
  // batch_insert — insert multiple nodes at once (no API call)
  // ---------------------------------------------------------------------------

  private async handleBatchInsert(args: {
    parentId: string | null;
    nodes: unknown[];
  }): Promise<ToolResult> {
    if (!args.nodes?.length) {
      return { success: false, error: 'No nodes provided' };
    }

    const { insertStreamingNode, applyPostStreamingTreeHeuristics } =
      await import('@/services/ai/design-generator');
    const { startNewAnimationBatch, markNodesForAnimation } =
      await import('@/services/ai/design-animation');
    const { addAgentIndicatorRecursive } = await import('@/canvas/agent-indicator');
    const { assignAgentIdentities } = await import('@/services/ai/agent-identity');

    // Limit batch size — model should call batch_insert multiple times with ≤9 nodes each
    const MAX_NODES = 9;
    const nodes = args.nodes.slice(0, MAX_NODES);

    // Random agent identity for breathing glow
    const [identity] = assignAgentIdentities(1);

    startNewAnimationBatch();

    let inserted = 0;
    let rootId: string | null = null;

    console.info(
      '[batch_insert] received',
      args.nodes.length,
      'nodes (limit',
      MAX_NODES,
      '), parentId:',
      args.parentId,
    );
    for (const raw of nodes) {
      const node = raw as Record<string, unknown>;
      if (!node.id || !node.type) {
        console.warn(
          '[batch_insert] skipping node without id/type:',
          JSON.stringify(node).slice(0, 200),
        );
        continue;
      }

      // Resolve _parent: use explicit _parent, or fall back to parentId arg
      const parentTarget = (node._parent as string | null) ?? args.parentId ?? null;
      delete node._parent;

      // Breathing glow indicator with random color + name
      addAgentIndicatorRecursive(node as any, identity.color, identity.name);
      markNodesForAnimation([node as any]);
      insertStreamingNode(node as any, parentTarget);

      if (!rootId) rootId = node.id as string;
      inserted++;

      // Streaming effect: pause every 3 nodes for visual feedback
      if (inserted % 3 === 0) {
        await new Promise((r) => setTimeout(r, 80));
      }
    }

    // Apply role defaults + post-pass fixes
    const effectiveRoot = args.parentId ?? rootId;
    if (effectiveRoot) {
      applyPostStreamingTreeHeuristics(effectiveRoot);
    }

    if (inserted > 0) {
      this.markContentStarted();
    }

    // Auto-zoom
    try {
      const { zoomToFitContent } = await import('@/canvas/skia-engine-ref');
      setTimeout(() => zoomToFitContent(), 300);
    } catch {
      /* ignore */
    }

    return {
      success: true,
      data: {
        inserted,
        message: `Inserted ${inserted} nodes. Design elements are now on the canvas.`,
      },
    };
  }
}
