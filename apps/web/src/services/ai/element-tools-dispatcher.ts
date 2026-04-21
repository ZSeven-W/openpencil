/**
 * Element-tools dispatcher — Phase 2 M1 scaffold.
 *
 * Takes a parsed `<op_tool>` output shape (see design-parser.ts
 * `tryParseElementToolOutput`) and routes it through the apply pipeline,
 * wrapped in a single history/undo batch so one generation session
 * becomes one user-visible undo entry (not N entries per sub-tool).
 *
 * M1 scope (this file): dispatcher skeleton + batch wrap + route
 * classification. Actual apply is stubbed with a structured
 * "unsupported" result that names which milestone will wire it:
 *
 *   - `add_X_v0` element-tool calls → waiting on M2 client shim
 *     registry (apps/web/src/services/ai/element-tool-shims/) or the
 *     M3 HTTP fallback (POST to Nitro → pen-mcp server).
 *   - `batch_design` DSL calls → waiting on M2 browser-safe DSL
 *     executor (same shape as pen-mcp's `executeLine`, minus node:fs).
 *
 * Why scaffold first: the history-batch invariant (exactly one
 * `startBatch` per generation, exactly one `endBatch` even on error)
 * is the load-bearing contract of Phase 2. Getting it right — and
 * covering it with tests — before actual apply code lands means M2
 * can drop in handlers without re-auditing the history integration.
 *
 * Plan: openpencil-docs/superpowers/plans/2026-04-21-element-tools-phase2-architecture.md
 */

import type { DesignOutputShape } from './design-parser';
import { useDocumentStore } from '@/stores/document-store';
import { useHistoryStore } from '@/stores/history-store';
import { useCanvasStore } from '@/stores/canvas-store';
import { getElementShim } from './element-tool-shims';
import { insertStreamingNode } from './design-canvas-ops';
import type { PenNode } from '@/types/pen';

/**
 * Context passed in by the caller (typically orchestrator-sub-agent)
 * that the dispatcher must honor to keep element-tool inserts consistent
 * with the streaming path's invariants.
 */
export interface DispatchContext {
  /**
   * Parent id to use when the model's `<op_tool>` payload omits
   * `parent_id`. Typically `subtask.parentFrameId ?? plan.rootFrame.id`
   * — the generation's target frame. Matches the streaming
   * renderer's `parentFrameId` (see
   * apps/web/src/services/ai/streaming-design-renderer.ts).
   *
   * Without a default, the dispatcher would insert at page root,
   * which puts element-tool output OUTSIDE the root frame's
   * layout / height-adjustment / cleanup scope — the 2026-04-21
   * Codex review regression.
   *
   * `null` means "insert at page root" (fallback when there is
   * genuinely no generation parent, e.g. smoke tests).
   */
  defaultParentId?: string | null;
}

/**
 * Status kinds a dispatch can produce. `unsupported` means "parser
 * recognized the shape but no apply handler is wired yet" — M1 returns
 * this for every input so callers can assert on it.
 */
export type DispatchStatus = 'applied' | 'failed' | 'unsupported';

export interface DispatchResult {
  /** High-level outcome — `applied` iff M2/M3 successfully mutated the doc. */
  status: DispatchStatus;
  /** Human-facing diagnostic. Safe to surface to the UI / error toast. */
  message: string;
  /** Route tag so the caller can log/metric which branch fired. */
  route: 'element-tool' | 'batch-design-dsl';
  /** Tool name (element-tool route) or the literal 'batch_design'. */
  toolName: string;
  /**
   * Nodes inserted by a successful apply. Empty for failed / unsupported
   * results. Orchestrator uses this list to keep its progress counters
   * + inserted-nodes aggregate in sync with the document store — without
   * it, the renderer-based tally would show 0 nodes for element-tool
   * subtasks even after a successful apply, and the caller would treat
   * the subtask as a failure despite the live canvas having received
   * the insertion. See openpencil-docs
   * superpowers/plans/2026-04-21-element-tools-phase2-architecture.md §10.
   */
  insertedNodes: PenNode[];
}

const PHASE2_PLAN_PATH =
  'openpencil-docs superpowers/plans/2026-04-21-element-tools-phase2-architecture.md';

/**
 * Dispatch a parsed element-tool output inside a single history batch.
 *
 * Invariants (M1, locked by unit tests):
 *   1. `historyStore.startBatch(baseDoc)` fires exactly once, BEFORE any
 *      route handler runs
 *   2. `historyStore.endBatch(currentDoc)` fires exactly once, AFTER the
 *      route handler returns — including when it throws. (We rely on
 *      `try/finally`; do not add early returns between start and end.)
 *   3. Returning `unsupported` from a route handler does NOT throw;
 *      callers get a typed result and can surface it however they want.
 *
 * The caller (orchestrator-sub-agent.ts) passes the shape from
 * `tryParseElementToolOutput`. Dispatcher does not re-parse.
 */
export async function dispatchElementToolCall(
  shape: DesignOutputShape,
  ctx: DispatchContext = {},
): Promise<DispatchResult> {
  const historyStore = useHistoryStore.getState();
  const baseDoc = useDocumentStore.getState().document;

  historyStore.startBatch(baseDoc);
  try {
    if (shape.kind === 'element-tool') {
      return await applyElementTool(shape.name, shape.arguments, ctx);
    }
    return await applyBatchDesignDsl(shape.dsl, ctx);
  } finally {
    const finalDoc = useDocumentStore.getState().document;
    historyStore.endBatch(finalDoc);
  }
}

/**
 * Apply a single `add_X_v0` element-tool call.
 *
 * Phase 2 M2 path (this function):
 *   1. Look up the tool name in the client shim registry
 *      (`element-tool-shims/index.ts`). Shims delegate to
 *      `@zseven-w/pen-core/element-builders` — same builders the
 *      pen-mcp handlers use, so the shape is drift-free by
 *      construction.
 *   2. If a shim exists, build the PenNode tree and call
 *      `document-store.addNode(null, node)` to insert at root of the
 *      active page. The store write triggers `pushState`, which is
 *      suppressed by the surrounding `startBatch` so multiple tool
 *      calls collapse into a single undo entry.
 *   3. If no shim exists, try the M3 HTTP fallback (POST to
 *      `/api/mcp/exec-tool`). Returns the resulting document in the
 *      response body so the client applies it without waiting for
 *      the SSE broadcast.
 *   4. If HTTP fallback also fails, return `unsupported` with a clear
 *      diagnostic so the UI can show actionable text.
 *
 * Shim miss is NOT an error — the M3 fallback is the documented
 * recovery path. We only return `unsupported` when both shim and
 * fallback fail.
 */
async function applyElementTool(
  name: string,
  args: Record<string, unknown>,
  ctx: DispatchContext,
): Promise<DispatchResult> {
  const shim = getElementShim(name);
  if (shim) {
    let shimResult: ReturnType<typeof shim>;
    try {
      shimResult = shim(args);
    } catch (err) {
      return {
        status: 'failed',
        route: 'element-tool',
        toolName: name,
        message:
          `Element tool "${name}" shim failed to build tree: ` +
          `${err instanceof Error ? err.message : String(err)}`,
        insertedNodes: [],
      };
    }

    const { node, parentId, pageId, filePath } = shimResult;
    const docStore = useDocumentStore.getState();

    // filePath is a pen-mcp-only concern — it addresses a .op file on
    // disk. The browser has no file I/O surface; addNode only mutates
    // the in-memory document-store. Silently dropping filePath would
    // report `applied` while the named file stays untouched, which
    // matches the 2026-04-21 stop-hook regression. Fail loudly and
    // point the caller at the path that actually supports it.
    if (filePath !== null) {
      return {
        status: 'failed',
        route: 'element-tool',
        toolName: name,
        message:
          `Element tool "${name}" named filePath="${filePath}" but the client shim ` +
          `has no file I/O surface (it only writes to the in-memory live canvas). ` +
          `Route this call through pen-mcp's stdio / HTTP transport, which maps ` +
          `to the file-aware pen-mcp handler, or omit filePath to target the ` +
          `currently-synced live canvas.`,
        insertedNodes: [],
      };
    }

    // pageId is honored only when it matches the currently-active page.
    // document-store.addNode doesn't expose a cross-page insert path,
    // and silently retargeting to the active page would make the
    // "applied" result a lie. Surface the mismatch so the caller can
    // either switch pages first or let the HTTP fallback handle it.
    if (pageId !== null) {
      const activePageId = useCanvasStore.getState().activePageId;
      if (pageId !== activePageId) {
        return {
          status: 'failed',
          route: 'element-tool',
          toolName: name,
          message:
            `Element tool "${name}" named pageId="${pageId}" but the active page is ` +
            `${activePageId === null ? '<none>' : `"${activePageId}"`}. The client shim ` +
            `only inserts on the active page; switch pages first or route this call ` +
            `through the HTTP fallback (which supports explicit pageId).`,
          insertedNodes: [],
        };
      }
    }

    // Validate parent_id before touching the store. insertNodeInTree
    // silently returns the original tree when parentId is missing,
    // producing a "success"-looking response with an orphaned node —
    // the same trap pen-mcp handlers guard against. Mirror that here.
    if (parentId !== null && !docStore.getNodeById(parentId)) {
      return {
        status: 'failed',
        route: 'element-tool',
        toolName: name,
        message:
          `Element tool "${name}" named parent_id="${parentId}" but no node with ` +
          `that id exists in the active document. Check the id or omit parent_id ` +
          `to insert at page root.`,
        insertedNodes: [],
      };
    }

    // Resolve effective parent: explicit parent_id > caller-supplied
    // default (subtask.parentFrameId ?? plan.rootFrame.id) > page root.
    // Without the ctx default, rootless subtask output would land at
    // page root with prepend semantics, breaking root-frame layout /
    // height adjustment / cleanup and reordering multi-section output
    // (the 2026-04-21 Codex review regression).
    const defaultParent = ctx.defaultParentId ?? null;

    // When we're about to use ctx.defaultParentId as the fallback
    // target, the default id MUST exist. Otherwise
    // insertStreamingNode's internal fallback chain silently
    // retargets to the module-global generationRootFrameId (which
    // may be stale from a prior generation or unset entirely) —
    // call looks applied but the subtree ends up in a different
    // frame, or at page root, or orphaned. Fail fast with a clear
    // diagnostic so stale orchestrator bindings surface immediately
    // instead of manifesting as misplaced canvas content.
    if (parentId === null && defaultParent !== null && !docStore.getNodeById(defaultParent)) {
      return {
        status: 'failed',
        route: 'element-tool',
        toolName: name,
        message:
          `Element tool "${name}" fell back to dispatcher defaultParentId="${defaultParent}" ` +
          `but no node with that id exists in the active document. The orchestrator's ` +
          `subtask parent binding is stale — fix it upstream or supply an explicit parent_id ` +
          `in the tool payload.`,
        insertedNodes: [],
      };
    }

    const effectiveParent = parentId ?? defaultParent;

    try {
      // Use insertStreamingNode (not raw addNode) so element-tool
      // output goes through the same canonical path as the streaming
      // renderer: id collision guard, parent remap, layout-aware child
      // normalization, phone-placeholder guards, overlay z-order rules,
      // and auto expandRootFrameHeight. Append semantics (index=Infinity)
      // are handled internally — element tools are generation-order
      // content, not overlays, so they stack after existing siblings.
      insertStreamingNode(node, effectiveParent);
    } catch (err) {
      return {
        status: 'failed',
        route: 'element-tool',
        toolName: name,
        message:
          `Element tool "${name}" insertStreamingNode threw: ` +
          `${err instanceof Error ? err.message : String(err)}`,
        insertedNodes: [],
      };
    }

    return {
      status: 'applied',
      route: 'element-tool',
      toolName: name,
      message:
        effectiveParent === null
          ? `Element tool "${name}" applied via client shim (page root).`
          : `Element tool "${name}" applied via client shim under parent "${effectiveParent}"` +
            (parentId === null ? ' (dispatcher default)' : '') +
            '.',
      insertedNodes: [node],
    };
  }

  return fallbackViaHttp('element-tool', name, ctx, { name, arguments: args });
}

/**
 * Apply a `batch_design` DSL string emitted inside an `<op_tool>` wrapper.
 *
 * Phase 2 M3 path: no client-side parser — DSL parsing is server-side
 * only because it requires pen-mcp's `splitOperations` +
 * `executeLine` which are Node-only (via document-manager /
 * `openDocument` file I/O). Dispatcher forwards the DSL string to
 * the `/api/mcp/exec-tool` Nitro endpoint, which hands it to
 * `handleBatchDesign` and returns the updated document.
 *
 * Client applies the response doc via `applyExternalDocument` so
 * the canvas reflects the change without waiting for the SSE
 * broadcast (which would be async relative to the batch wrap).
 */
async function applyBatchDesignDsl(dsl: string, ctx: DispatchContext): Promise<DispatchResult> {
  return fallbackViaHttp('batch-design-dsl', 'batch_design', ctx, { dsl });
}

/**
 * POST to the M3 fallback endpoint. The endpoint either successfully
 * applies the tool (returning the updated doc in `document`) or
 * returns a structured error which we surface verbatim to the user.
 *
 * If the fetch itself fails (network error, endpoint missing, CORS,
 * etc.), we fall back to `unsupported` — this is the canonical M1
 * behavior so callers never see a silently-dropped tool call.
 */
async function fallbackViaHttp(
  route: DispatchResult['route'],
  toolName: string,
  ctx: DispatchContext,
  body: Record<string, unknown>,
): Promise<DispatchResult> {
  try {
    // Forward the caller-supplied default parent so the server-side
    // endpoint inserts into the same frame the streaming path would
    // (subtask.parentFrameId) when the model omits parent_id. Without
    // this the HTTP path would append to the first page at index 0,
    // same pathology the client-side path guards against via ctx.
    const fullBody =
      ctx.defaultParentId != null ? { ...body, default_parent_id: ctx.defaultParentId } : body;
    const res = await fetch('/api/mcp/exec-tool', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(fullBody),
    });
    if (!res.ok) {
      const errText = await res.text().catch(() => `HTTP ${res.status}`);
      return {
        status: 'failed',
        route,
        toolName,
        message: `HTTP fallback for "${toolName}" returned ${res.status}: ${errText}`,
        insertedNodes: [],
      };
    }
    const data = (await res.json()) as {
      ok?: boolean;
      document?: unknown;
      insertedNodeId?: string;
      error?: string;
    };
    if (data.ok === true && data.document && typeof data.document === 'object') {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const doc = data.document as any;
      useDocumentStore.getState().applyExternalDocument(doc);

      // Surface the inserted node to the orchestrator so renderer-based
      // tallies (progressEntry.nodeCount / progress.totalNodes /
      // onApplyPartial) reflect the real apply. The endpoint stamps
      // `insertedNodeId` on the response; we fish the node back out of
      // the returned doc rather than relying on the orchestrator to
      // walk the store after the fact (doc is already structured here,
      // and keeps the dispatcher's contract self-contained).
      const insertedNodes: PenNode[] = [];
      if (typeof data.insertedNodeId === 'string') {
        const found = findNodeByIdInDoc(doc, data.insertedNodeId);
        if (found) insertedNodes.push(found);
      }
      return {
        status: 'applied',
        route,
        toolName,
        message: `Applied via HTTP fallback (${toolName}).`,
        insertedNodes,
      };
    }
    return {
      status: 'unsupported',
      route,
      toolName,
      message:
        data.error ??
        `HTTP fallback for "${toolName}" returned ok=false without an error message. ` +
          `See ${PHASE2_PLAN_PATH}.`,
      insertedNodes: [],
    };
  } catch (err) {
    return {
      status: 'unsupported',
      route,
      toolName,
      message:
        `HTTP fallback for "${toolName}" unreachable ` +
        `(${err instanceof Error ? err.message : String(err)}). ` +
        `Unset VITE_ENABLE_ELEMENT_TOOLS to fall back to the legacy JSONL path, ` +
        `or see ${PHASE2_PLAN_PATH}.`,
      insertedNodes: [],
    };
  }
}

/**
 * Walk a PenDocument (pages or legacy children[]) for a node with the
 * given id. Local helper — pen-core's `findNodeInTree` operates on a
 * children array; we'd have to pass the right one per doc shape, so
 * it's simpler to inline the traversal here.
 */
function findNodeByIdInDoc(
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  doc: any,
  id: string,
): PenNode | null {
  const pageChildren: PenNode[][] = Array.isArray(doc?.pages)
    ? doc.pages.map((p: { children?: PenNode[] }) => p?.children ?? [])
    : [];
  const roots: PenNode[][] = pageChildren.length > 0 ? pageChildren : [doc?.children ?? []];
  for (const list of roots) {
    for (const node of list) {
      const hit = findInSubtree(node, id);
      if (hit) return hit;
    }
  }
  return null;
}

function findInSubtree(node: PenNode, id: string): PenNode | null {
  if (node?.id === id) return node;
  const children = (node as { children?: PenNode[] })?.children;
  if (Array.isArray(children)) {
    for (const child of children) {
      const hit = findInSubtree(child, id);
      if (hit) return hit;
    }
  }
  return null;
}
