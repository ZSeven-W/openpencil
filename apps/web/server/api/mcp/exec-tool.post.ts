import { defineEventHandler, readBody, setResponseStatus } from 'h3';
import {
  assignIdsRecursively,
  buildBodyText,
  buildBottomNav,
  buildCardRow,
  buildHeading,
  buildListRow,
  buildMetricRow,
  buildSearchBar,
  buildSectionHeader,
  buildTextButton,
  buildTopNavBar,
  findNodeInTree,
  insertNodeInTree,
  type ElementTree,
} from '@zseven-w/pen-core';
import { getSyncDocument, setSyncDocument } from '../../utils/mcp-sync-state';
import { serverLog } from '../../utils/server-logger';
import type { PenDocument, PenNode } from '../../../src/types/pen';

/**
 * POST /api/mcp/exec-tool — Phase 2 M3 HTTP fallback.
 *
 * Accepts either:
 *   - `{ name: 'add_X_v0', arguments: {...} }` — an element-tool call
 *     the browser couldn't match to a client shim (or for which the
 *     caller wants authoritative server-side state)
 *   - `{ dsl: '...' }` — a batch_design DSL string
 *
 * For known element tools, the endpoint uses the SAME pen-core
 * builders the client shim uses, so the tree shape is byte-identical
 * across paths. The constructed tree is inserted at the root of the
 * currently-active page's live document; the updated doc is
 * broadcast via SSE and returned in the response body so the
 * dispatching client can `applyExternalDocument` immediately without
 * waiting for its own SSE to arrive.
 *
 * For batch_design DSL, the endpoint currently rejects — pen-mcp's
 * `handleBatchDesign` relies on `document-manager` (file I/O,
 * live-canvas URL dance) and would need a Nitro-local adaptation.
 * That's a follow-up once the element-tool shim family shows the
 * bulk of wins in the embedded orchestrator. Client dispatcher
 * surfaces this error verbatim so dev runs see exactly why.
 */

type BuilderFn = (args: unknown) => ElementTree;

/**
 * Server-side builder registry. Keep in sync with
 * `apps/web/src/services/ai/element-tool-shims/index.ts`. Both
 * dispatch through pen-core `buildX` so drift is impossible; this
 * registry exists only to gate unknown tool names.
 */
const SERVER_BUILDERS: Record<string, BuilderFn> = {
  add_card_row_v0: (a) => buildCardRow(a as Parameters<typeof buildCardRow>[0]),
  add_metric_row_v0: (a) => buildMetricRow(a as Parameters<typeof buildMetricRow>[0]),
  add_bottom_nav_v0: (a) => buildBottomNav(a as Parameters<typeof buildBottomNav>[0]),
  add_section_header_v0: (a) => buildSectionHeader(a as Parameters<typeof buildSectionHeader>[0]),
  add_top_nav_bar_v0: (a) => buildTopNavBar(a as Parameters<typeof buildTopNavBar>[0]),
  add_heading_v0: (a) => buildHeading(a as Parameters<typeof buildHeading>[0]),
  add_body_text_v0: (a) => buildBodyText(a as Parameters<typeof buildBodyText>[0]),
  add_text_button_v0: (a) => buildTextButton(a as Parameters<typeof buildTextButton>[0]),
  add_search_bar_v0: (a) => buildSearchBar(a as Parameters<typeof buildSearchBar>[0]),
  add_list_row_v0: (a) => buildListRow(a as Parameters<typeof buildListRow>[0]),
};

interface ExecToolBody {
  name?: string;
  arguments?: unknown;
  dsl?: string;
  /**
   * Fallback parent id when `arguments.parent_id` is not provided.
   * Forwarded by the client dispatcher from the orchestrator's subtask
   * context (`subtask.parentFrameId ?? plan.rootFrame.id`). Without it
   * rootless payloads would land at page root outside the generation's
   * scope — mirrors the client-shim `ctx.defaultParentId` contract.
   */
  default_parent_id?: string;
}

interface ExecToolError {
  ok: false;
  error: string;
}

interface ExecToolSuccess {
  ok: true;
  document: PenDocument;
  insertedNodeId: string;
}

export default defineEventHandler(async (event): Promise<ExecToolSuccess | ExecToolError> => {
  const body = (await readBody(event).catch(() => null)) as ExecToolBody | null;
  if (!body) {
    setResponseStatus(event, 400);
    return { ok: false, error: 'Missing or malformed JSON body' };
  }

  if (body.dsl) {
    // Reject with actionable diagnostic — client dispatcher surfaces this to the UI.
    setResponseStatus(event, 501);
    return {
      ok: false,
      error:
        'batch_design DSL via /api/mcp/exec-tool is not yet wired (Phase 2 M3 covers ' +
        'element tools only; full DSL support is a follow-up). Use a specific add_*_v0 ' +
        'tool or unset VITE_ENABLE_ELEMENT_TOOLS to fall back to the legacy JSONL path.',
    };
  }

  if (!body.name) {
    setResponseStatus(event, 400);
    return { ok: false, error: 'Missing "name" (or "dsl") in request body' };
  }

  const builder = SERVER_BUILDERS[body.name];
  if (!builder) {
    setResponseStatus(event, 404);
    return {
      ok: false,
      error:
        `Element tool "${body.name}" has no server-side builder. Phase 2 M3 currently ` +
        `covers: ${Object.keys(SERVER_BUILDERS).join(', ')}. Extend SERVER_BUILDERS in ` +
        `apps/web/server/api/mcp/exec-tool.post.ts (and the matching shim in ` +
        `apps/web/src/services/ai/element-tool-shims/index.ts) to add more.`,
    };
  }

  // Split meta fields out of the payload before the builder sees them.
  // Builder signatures only accept tool-specific params; leaving
  // parent_id/pageId in would either be silently ignored or rejected
  // as unknown. Mirrors the client shim wrap() contract.
  const rawArgs = (body.arguments ?? {}) as Record<string, unknown>;
  const parentId =
    typeof rawArgs.parent_id === 'string' && rawArgs.parent_id.length > 0
      ? rawArgs.parent_id
      : null;
  const targetPageId =
    typeof rawArgs.pageId === 'string' && rawArgs.pageId.length > 0 ? rawArgs.pageId : null;
  // `live://canvas` is pen-mcp's explicit sentinel for "the live
  // canvas" (document-manager.ts::resolveDocPath treats it the same
  // as an undefined filePath). Normalize to null so the non-live-
  // canvas rejection below only triggers for real file paths.
  const LIVE_CANVAS_SENTINEL = 'live://canvas';
  const rawFilePath =
    typeof rawArgs.filePath === 'string' && rawArgs.filePath.length > 0 ? rawArgs.filePath : null;
  const targetFilePath = rawFilePath === LIVE_CANVAS_SENTINEL ? null : rawFilePath;
  const builderArgs = (() => {
    const { parent_id: _pid, pageId: _pg, filePath: _fp, ...rest } = rawArgs;
    return rest;
  })();

  // filePath is only meaningful for pen-mcp's file-backed handlers
  // (they read/write .op files via document-manager). This endpoint
  // targets the in-memory live document (mcp-sync-state). Silently
  // dropping filePath would report success while the named file stays
  // untouched — surface the mismatch with 501 so the caller routes
  // through pen-mcp's stdio/HTTP transport instead.
  if (targetFilePath !== null) {
    setResponseStatus(event, 501);
    return {
      ok: false,
      error:
        `filePath="${targetFilePath}" is not supported by /api/mcp/exec-tool — this ` +
        `endpoint mutates the in-memory live canvas only. Route through pen-mcp's ` +
        `stdio / HTTP transport (which owns .op file I/O via document-manager), or ` +
        `omit filePath to target the currently-synced live canvas.`,
    };
  }

  let tree: ElementTree;
  try {
    tree = builder(builderArgs);
  } catch (err) {
    setResponseStatus(event, 400);
    return {
      ok: false,
      error: `Builder "${body.name}" rejected arguments: ${err instanceof Error ? err.message : String(err)}`,
    };
  }
  assignIdsRecursively(tree);
  const insertedNodeId = String(tree.id);

  const { doc: current } = getSyncDocument();
  if (!current) {
    setResponseStatus(event, 409);
    return {
      ok: false,
      error:
        'No live canvas document is currently synced. Open a document before invoking ' +
        '/api/mcp/exec-tool.',
    };
  }

  const next = structuredClone(current) as PenDocument;

  // Resolve target page: explicit pageId > first page > legacy children[]
  const pageList = Array.isArray(next.pages) ? next.pages : [];
  let targetPage: { id?: string; children: PenNode[] } | null = null;
  if (targetPageId !== null) {
    const match = pageList.find((p) => p.id === targetPageId);
    if (!match) {
      setResponseStatus(event, 404);
      return {
        ok: false,
        error: `pageId "${targetPageId}" not found in the live document.`,
      };
    }
    targetPage = match;
  } else if (pageList.length > 0) {
    targetPage = pageList[0];
  }

  // Resolve target parent: explicit parent_id > caller-supplied
  // default_parent_id > page root. Explicit parent_id must exist on
  // the chosen page (not silently inserted at root). default_parent_id
  // also must exist (caller is committing to a real target frame).
  const pageChildren: PenNode[] = targetPage
    ? targetPage.children
    : ((next.children ?? []) as PenNode[]);
  if (parentId !== null) {
    const found = findNodeInTree(pageChildren, parentId);
    if (!found) {
      setResponseStatus(event, 404);
      return {
        ok: false,
        error: `parent_id "${parentId}" not found in ${
          targetPageId !== null ? `page "${targetPageId}"` : 'the live document'
        }.`,
      };
    }
  }
  const defaultParentId =
    typeof body.default_parent_id === 'string' && body.default_parent_id.length > 0
      ? body.default_parent_id
      : null;
  if (parentId === null && defaultParentId !== null) {
    const defaultParentNode = findNodeInTree(pageChildren, defaultParentId);
    if (!defaultParentNode) {
      setResponseStatus(event, 404);
      return {
        ok: false,
        error:
          `default_parent_id "${defaultParentId}" not found in ${
            targetPageId !== null ? `page "${targetPageId}"` : 'the live document'
          }. Fix the orchestrator's subtask parent binding or omit default_parent_id ` +
          `to fall through to page-root insertion.`,
      };
    }
  }
  const effectiveParent = parentId ?? defaultParentId;

  // Append (index=Infinity) to match the streaming path's generation-
  // order semantics. Default document-store.addNode prepends (index=0)
  // because new user-created nodes want to be on top of the layer
  // panel; AI-generation wants each element to stack after earlier
  // siblings so multi-call output renders top-to-bottom as emitted.
  const updatedChildren = insertNodeInTree(
    pageChildren,
    effectiveParent,
    tree as unknown as PenNode,
    Infinity,
  );
  if (targetPage) {
    targetPage.children = updatedChildren;
  } else {
    next.children = updatedChildren;
  }

  setSyncDocument(next);
  serverLog.info(
    `[exec-tool] applied ${body.name} → node ${insertedNodeId}` +
      (effectiveParent !== null
        ? ` under ${parentId !== null ? 'parent' : 'default parent'} ${effectiveParent}`
        : '') +
      (targetPageId !== null ? ` on page ${targetPageId}` : ''),
  );

  return { ok: true, document: next, insertedNodeId };
});
