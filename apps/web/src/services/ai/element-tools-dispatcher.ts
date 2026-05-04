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

import { parseJsonlToTree, type DesignOutputShape } from './design-parser';
import { useDocumentStore } from '@/stores/document-store';
import { useHistoryStore } from '@/stores/history-store';
import { useCanvasStore } from '@/stores/canvas-store';
import { getElementShim, SUPPORTED_EMBEDDED_ELEMENT_TOOLS } from './element-tool-shims';
import { insertStreamingNode } from './design-canvas-ops';
// Import from the pen-mcp package's browser-safe `./dsl` subpath (not
// the top-level barrel). The barrel re-exports node-only modules —
// `document-manager`, `log-utils`, `theme-presets` — that pull in
// `node:fs` / `node:path`. Vite / esbuild resolve the barrel before
// tree-shaking can drop those branches, so the browser build fails
// on the unresolved node built-ins. The `./dsl` subpath points
// directly at `tools/batch-design-dsl.ts`, which is the only module
// this dispatcher actually needs and is kept rigorously browser-safe
// (enforced by `packages/pen-mcp/src/__tests__/batch-design-dsl-browser-safe.test.ts`).
import { runBatchDesignDsl } from '@zseven-w/pen-mcp/dsl';
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

export interface BatchDispatchResult {
  /** Status summary: 'applied' iff every shape applied; 'partial' if some
   *  applied and some failed; 'all-failed' if none did; 'empty' for no shapes. */
  status: 'applied' | 'partial' | 'all-failed' | 'empty';
  /** Per-shape result, in emit order — same index maps to input shapes. */
  results: DispatchResult[];
  /** Union of all inserted nodes across the batch. Orchestrator uses
   *  this for progress accounting (sum of insertedNodes[]). */
  insertedNodes: PenNode[];
}

/**
 * Apply multiple `<op_tool>` tags in a single history batch. One
 * startBatch/endBatch pair wraps the whole loop so N tags collapse
 * to one undo unit — independent of how many actual store writes
 * each shape performs (element-tool shim: 1 addNode; HTTP fallback:
 * 1 applyExternalDocument; DSL HTTP: 1 applyExternalDocument with
 * possibly-many child nodes inside the returned doc).
 *
 * Error handling: a failing shape does NOT abort the batch. The
 * per-shape `DispatchResult` captures success/failure; caller
 * inspects `status` for rollup. This matches the pen-mcp
 * handleBatchDesign's "collect-errors-keep-going" philosophy —
 * partial progress is better than an atomic all-or-nothing because
 * the AI's next call may refer to nodes from successful earlier
 * tags in the same response.
 *
 * ctx.defaultParentId applies to every shape identically; callers
 * that need per-shape parent routing should wire that via shape's
 * own `parent_id` arg.
 */
export async function dispatchElementToolCalls(
  shapes: DesignOutputShape[],
  ctx: DispatchContext = {},
): Promise<BatchDispatchResult> {
  if (shapes.length === 0) {
    return { status: 'empty', results: [], insertedNodes: [] };
  }
  const historyStore = useHistoryStore.getState();
  const baseDoc = useDocumentStore.getState().document;
  historyStore.startBatch(baseDoc);
  const results: DispatchResult[] = [];
  try {
    for (const shape of shapes) {
      try {
        if (shape.kind === 'element-tool') {
          results.push(await applyElementTool(shape.name, shape.arguments, ctx));
        } else {
          results.push(await applyBatchDesignDsl(shape.dsl, ctx));
        }
      } catch (err) {
        // applyX shouldn't throw (they catch + return failed); defend
        // anyway so the loop never aborts mid-batch.
        results.push({
          status: 'failed',
          route: shape.kind === 'element-tool' ? 'element-tool' : 'batch-design-dsl',
          toolName: shape.kind === 'element-tool' ? shape.name : 'batch_design',
          message: `Unexpected throw during dispatch: ${err instanceof Error ? err.message : String(err)}`,
          insertedNodes: [],
        });
      }
    }
  } finally {
    const finalDoc = useDocumentStore.getState().document;
    historyStore.endBatch(finalDoc);
  }
  const insertedNodes = results.flatMap((r) => r.insertedNodes);
  const appliedCount = results.filter((r) => r.status === 'applied').length;
  const status: BatchDispatchResult['status'] =
    appliedCount === results.length ? 'applied' : appliedCount === 0 ? 'all-failed' : 'partial';
  return { status, results, insertedNodes };
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
 *   2. If a shim exists, build the PenNode tree via pen-core and
 *      call `insertStreamingNode` (not raw addNode) so the insert
 *      goes through the same canonical path the streaming
 *      renderer uses: id collision guard, parent remap, layout-
 *      aware child normalization, phone-placeholder guards,
 *      overlay z-order rules, auto expandRootFrameHeight, and
 *      append semantics. The surrounding `startBatch` suppresses
 *      `pushState` so multiple tool calls collapse to one undo.
 *   3. If the tool name is NOT in the embedded shim/server registry
 *      (SUPPORTED_EMBEDDED_ELEMENT_TOOLS), short-circuit with an
 *      actionable `unsupported` result — do not waste an HTTP
 *      roundtrip on a tool the endpoint is also guaranteed to 404.
 *      elements.md names 42 tools (for external MCP clients that
 *      talk to pen-mcp's full handler set); embedded coverage is
 *      a subset until more builders are extracted to pen-core.
 *   4. If the name IS supported but the browser shim lookup missed
 *      (shouldn't happen today since the two registries are kept
 *      byte-identical, but keeps the code correct if they drift),
 *      try the M3 HTTP fallback. Nitro endpoint uses the same
 *      pen-core builders + returns the resulting document so the
 *      client applies it via applyExternalDocument immediately.
 *
 * Shim miss for a SUPPORTED tool is NOT an error — HTTP fallback
 * is the documented recovery path. Miss for an UNSUPPORTED tool
 * is surfaced up without a roundtrip, carrying the canonical
 * supported list in the message.
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

  // Shim miss. Pre-check whether the tool name is even in the
  // embedded registry (shim + Nitro SERVER_BUILDERS are kept in
  // sync by convention). If not, skip the HTTP roundtrip — the
  // endpoint is guaranteed to 404 for the same name, so we can
  // deliver a better diagnostic immediately with the canonical
  // supported list. Prevents the AI / user from seeing a raw HTTP
  // error when the real story is "this tool is in elements.md's
  // 42-tool catalog but not yet wired into the embedded runtime."
  if (!SUPPORTED_EMBEDDED_ELEMENT_TOOLS.includes(name)) {
    return {
      status: 'unsupported',
      route: 'element-tool',
      toolName: name,
      message:
        `Element tool "${name}" is not wired into the embedded orchestrator ` +
        `(shim + Nitro builders cover ${SUPPORTED_EMBEDDED_ELEMENT_TOOLS.length} ` +
        `tools so far: ${SUPPORTED_EMBEDDED_ELEMENT_TOOLS.join(', ')}). Fall back to ` +
        `batch_design for this element, or extend the shim + Nitro registries ` +
        `together. External MCP clients (Claude Code / Codex / Gemini CLI) can ` +
        `still call the full 42-tool pen-mcp set via stdio / HTTP transport.`,
      insertedNodes: [],
    };
  }

  return fallbackViaHttp('element-tool', name, ctx, { name, arguments: args });
}

/**
 * Apply a `batch_design` DSL string emitted inside an `<op_tool>` wrapper.
 *
 * Runs the pure DSL executor (`runBatchDesignDsl` from
 * `@zseven-w/pen-mcp`) DIRECTLY in the browser against the live
 * document-store snapshot. No HTTP round-trip. The executor is
 * browser-safe — its transitive import tree excludes node:fs /
 * document-manager (guarded by `batch-design-dsl-browser-safe.test.ts`
 * in pen-mcp).
 *
 * Falls back to `fallbackViaHttp` only when the in-browser attempt
 * throws an unexpected exception (e.g. a future DSL op adds a
 * server-only dependency). This preserves the Phase 2 M3 contract
 * ("batch_design MUST execute") while removing the HTTP latency on
 * the common path.
 *
 * `ctx.defaultParentId` is intentionally NOT honored here: the DSL
 * is the AI's verbatim instruction set, and rewriting `null` parents
 * to the generation root would silently change the AI's intent.
 * Element-tool calls still honor it (via `applyElementToolCall`).
 */
/**
 * Detect whether `operations` looks like flat JSONL nodes rather than the
 * `foo=I("parent",{...})` assignment-based DSL. Weak / mid-tier models
 * (GPT-5.5 standard tier observed) emit their full design as raw JSONL
 * but stuff it inside `<op_tool>{name:"batch_design",arguments:{operations}}`
 * because the ELEMENT_TOOL_OUTPUT_FORMAT prompt teaches `<op_tool>`-only
 * output but doesn't show DSL syntax. The DSL parser then rejects every
 * line. Detect this case at dispatch time and route through the JSONL
 * apply path instead of failing.
 *
 * Signature: starts with `{` AND contains JSONL-shape keys (`_parent`
 * or a `type:"frame|text|…"` field) within the first chunk.
 */
function looksLikeJsonl(operations: string): boolean {
  const trimmed = operations.trim();
  if (!trimmed.startsWith('{')) return false;
  return /"_parent"\s*:|"type"\s*:\s*"(frame|text|rectangle|ellipse|icon_font|image|path|line|polygon|group)"/.test(
    trimmed.slice(0, 800),
  );
}

/**
 * Apply JSONL-shaped operations as flat node tree under the dispatch
 * context's default parent. Mirrors what raw-JSONL-output sub-agents
 * produce, but reached via the `<op_tool>{name:"batch_design"}` wrapper
 * that mid-tier models emit when they don't know the DSL syntax.
 *
 * Verifies each root via `getNodeById` AFTER the addNode call —
 * `useDocumentStore.addNode` returns void and falls through silently
 * (`insertNodeInTree` no-ops) if the parent id can't be resolved (e.g.
 * `defaultParentId` is stale or empty). Without the post-insert check
 * we'd report `status: 'applied'` while leaving the doc unchanged.
 */
function applyBatchDesignAsJsonl(jsonl: string, ctx: DispatchContext): DispatchResult {
  const tree = parseJsonlToTree(jsonl);
  if (!tree || tree.length === 0) {
    return {
      status: 'failed',
      route: 'batch-design-dsl',
      toolName: 'batch_design',
      message:
        'batch_design.operations parsed as JSONL but produced no nodes (empty/malformed input)',
      insertedNodes: [],
    };
  }
  const store = useDocumentStore.getState();
  const parentId = ctx.defaultParentId ?? null;

  // Collect every id the JSONL tree wants to introduce (roots + nested
  // descendants). `insertNodeInTree` does NOT dedupe — it appends — so a
  // collision with an existing live node would create two nodes sharing
  // the same id. `getNodeById` returns the FIRST match (the pre-existing
  // one) which makes the post-insert verification look successful, and a
  // later rollback `removeNode(id)` would delete the pre-existing node
  // instead of the freshly-added duplicate. Detect collisions before
  // mutating the doc so we can fail cleanly without side effects.
  const treeIds = new Set<string>();
  const collectIds = (n: PenNode) => {
    treeIds.add(n.id);
    if ('children' in n && Array.isArray((n as PenNode & { children?: PenNode[] }).children)) {
      for (const child of (n as PenNode & { children: PenNode[] }).children) collectIds(child);
    }
  };
  for (const root of tree) collectIds(root);
  const collisions: string[] = [];
  for (const id of treeIds) {
    if (store.getNodeById(id)) collisions.push(id);
  }
  if (collisions.length > 0) {
    const list = `${collisions.slice(0, 4).join(', ')}${collisions.length > 4 ? '…' : ''}`;
    return {
      status: 'failed',
      route: 'batch-design-dsl',
      toolName: 'batch_design',
      message:
        `batch_design.operations JSONL has ${collisions.length} id collision(s) with existing ` +
        `doc node(s): ${list}. Refusing to insert — doc state preserved.`,
      insertedNodes: [],
    };
  }

  const inserted: PenNode[] = [];
  const failedIds: string[] = [];
  for (const root of tree) {
    store.addNode(parentId, root);
    const live = useDocumentStore.getState().getNodeById(root.id);
    if (live) {
      inserted.push(live);
    } else {
      failedIds.push(root.id);
    }
  }
  // Any failure to land a root is a hard failure — even a single missing
  // root usually means defaultParentId is stale, the model emitted a
  // dangling _parent reference, or doc state has drifted. Returning
  // `applied` for partial inserts would lie to the orchestrator (it
  // checks `status === 'applied'` to decide whether to retry); returning
  // `failed` with non-empty `insertedNodes` would also bypass retry,
  // because the caller in orchestrator-sub-agent.ts gates retry on
  // `result.nodes.length === 0` — the partial inserts get reported as
  // the subtask's `nodes` and look "non-empty enough" to skip the retry
  // / minimal-skills fallback chain. Hard-rollback the partial inserts
  // so the doc returns to its pre-dispatch state, the surrounding
  // history batch nets to zero, and the retry condition fires.
  if (failedIds.length > 0) {
    if (inserted.length > 0) {
      // Safe: the collision precheck above guaranteed none of these ids
      // existed in the doc pre-dispatch, so removeNode(id) targets only
      // nodes this dispatch added.
      const rollbackStore = useDocumentStore.getState();
      for (const node of inserted) {
        try {
          rollbackStore.removeNode(node.id);
        } catch {
          /* already removed by an upstream cleanup — ignore */
        }
      }
    }
    const failedList = `${failedIds.slice(0, 4).join(', ')}${failedIds.length > 4 ? '…' : ''}`;
    if (inserted.length === 0) {
      return {
        status: 'failed',
        route: 'batch-design-dsl',
        toolName: 'batch_design',
        message:
          `batch_design.operations parsed as JSONL (${tree.length} root(s)) but none landed — ` +
          `parent id '${parentId ?? 'page-root'}' may be stale or unresolved. ` +
          `Failed root ids: ${failedList}`,
        insertedNodes: [],
      };
    }
    return {
      status: 'failed',
      route: 'batch-design-dsl',
      toolName: 'batch_design',
      message:
        `batch_design.operations partial failure: ${inserted.length}/${tree.length} root(s) ` +
        `landed, ${failedIds.length} did not (parent id '${parentId ?? 'page-root'}'). ` +
        `Rolled back partial inserts so retry can re-run the subtask cleanly. ` +
        `Failed root ids: ${failedList}`,
      insertedNodes: [],
    };
  }
  const totalLive = countDescendants(inserted);
  return {
    status: 'applied',
    route: 'batch-design-dsl',
    toolName: 'batch_design',
    message: `Applied batch_design.operations as JSONL (${tree.length} root node(s); ${totalLive} total).`,
    insertedNodes: inserted,
  };
}

function countDescendants(nodes: PenNode[]): number {
  let n = 0;
  for (const node of nodes) {
    n += 1;
    if ('children' in node && Array.isArray(node.children)) {
      n += countDescendants(node.children as PenNode[]);
    }
  }
  return n;
}

async function applyBatchDesignDsl(dsl: string, ctx: DispatchContext): Promise<DispatchResult> {
  // Auto-detect when the model stuffed JSONL into the DSL string field
  // (mid-tier models ignore the DSL-only contract because the prompt
  // doesn't show DSL syntax). Re-route to JSONL apply so we don't lose
  // the entire response to "Cannot parse operation" DSL errors.
  if (looksLikeJsonl(dsl)) {
    return applyBatchDesignAsJsonl(dsl, ctx);
  }
  try {
    const { document } = useDocumentStore.getState();
    const cloned = structuredClone(document) as typeof document;
    // Route the DSL to whichever page the user is viewing. Without
    // this, `runBatchDesignDsl` falls back to `doc.pages[0]` — on a
    // multi-page document that means the generation lands on page 1
    // while the UI shows the user's active page, producing invisible
    // output. `activePageId` may be `null` on single-page docs, in
    // which case the executor's default is correct (there's only one
    // page).
    const activePageId = useCanvasStore.getState().activePageId ?? undefined;
    // runBatchDesignDsl mutates the doc in place + returns per-op
    // results + errors. No image-search fetcher supplied — browser
    // G() ops leave src empty, which the apps/web image pipeline
    // (scanAndFillImages) will pick up and enrich after insert.
    const { results, errors } = await runBatchDesignDsl(cloned, dsl, {
      pageId: activePageId,
    });
    if (errors.length > 0) {
      return {
        status: 'failed',
        route: 'batch-design-dsl',
        toolName: 'batch_design',
        message:
          `batch_design DSL had ${errors.length} failing operation(s): ` +
          errors.map((e) => `${e.line.slice(0, 80)}: ${e.error}`).join('; '),
        insertedNodes: [],
      };
    }

    // Apply the mutated doc back to the live store. applyExternalDocument
    // snapshots for history so the dispatcher's surrounding startBatch/
    // endBatch wrapper still produces exactly one undo entry.
    useDocumentStore.getState().applyExternalDocument(cloned);

    const insertedNodes: PenNode[] = [];
    for (const r of results) {
      if (!r.nodeId) continue;
      const node = findNodeByIdInDoc(cloned, r.nodeId);
      if (node) insertedNodes.push(node);
    }
    return {
      status: 'applied',
      route: 'batch-design-dsl',
      toolName: 'batch_design',
      message: `Applied batch_design DSL in-browser (${results.length} op result(s)).`,
      insertedNodes,
    };
  } catch (err) {
    // In-browser executor threw (should be rare — usually caller
    // error in the DSL). Fall back to HTTP so the server can log +
    // return a structured error. ctx is forwarded so the server
    // still honors defaultParentId for the server-side element-tool
    // paths that share the endpoint.
    const message = err instanceof Error ? err.message : String(err);
    // Forward active pageId so the server-side path applies on the
    // same page the user is viewing — same rationale as the in-browser
    // path above. If activePageId is null (single-page doc), omit
    // rather than send an explicit null so the server's default (first
    // page) still works. The `pageId` field is read by
    // server/api/mcp/exec-tool.post.ts:283 for the DSL branch.
    const activePageId = useCanvasStore.getState().activePageId;
    const dslBody: Record<string, unknown> =
      typeof activePageId === 'string' && activePageId.length > 0
        ? { dsl, pageId: activePageId }
        : { dsl };
    const httpResult = await fallbackViaHttp('batch-design-dsl', 'batch_design', ctx, dslBody);
    // Note: HTTP may succeed even if in-browser threw. Preserve its
    // result as-is; only annotate the message when both paths fail.
    if (httpResult.status === 'applied') return httpResult;
    return {
      ...httpResult,
      message: `In-browser DSL failed (${message}); HTTP fallback also: ${httpResult.message}`,
    };
  }
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
      insertedNodeIds?: string[];
      error?: string;
    };
    if (data.ok === true && data.document && typeof data.document === 'object') {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const doc = data.document as any;
      useDocumentStore.getState().applyExternalDocument(doc);

      // Surface inserted nodes to the orchestrator so renderer-based
      // tallies (progressEntry.nodeCount / progress.totalNodes /
      // onApplyPartial) reflect the real apply. Prefer the array
      // form because batch_design can insert multiple top-level
      // nodes; fall back to the legacy single-id field for servers
      // that haven't been upgraded.
      const idList = Array.isArray(data.insertedNodeIds)
        ? data.insertedNodeIds
        : typeof data.insertedNodeId === 'string'
          ? [data.insertedNodeId]
          : [];
      const insertedNodes: PenNode[] = [];
      for (const id of idList) {
        if (typeof id !== 'string' || id.length === 0) continue;
        const found = findNodeByIdInDoc(doc, id);
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
