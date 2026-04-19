import { readFile, writeFile } from 'node:fs/promises';
import {
  LIVE_CANVAS_PATH,
  invalidateCache,
  openDocument,
  resolveDocPath,
  saveDocument,
} from '../document-manager';
import { findNodeInTree, findParentInTree, getDocChildren } from '../utils/node-operations';
import { generateId } from '../utils/id';
import { handleBatchDesign } from './batch-design';

/**
 * Simulate how batch-design.ts:resolveRef will interpret a JSON.stringify'd
 * parent_id. The parser does `raw.replace(/^"|"$/g, '')` — strip one leading
 * and one trailing `"` — and does NOT JSON-unescape. So any id containing
 * `"` or `\` will round-trip to a different literal than the caller
 * intended. Return value === original parent_id iff the DSL round-trip is
 * lossless.
 */
function simulateDslParentResolve(parentId: string): string {
  return JSON.stringify(parentId).replace(/^"|"$/g, '');
}

/**
 * Validate that `parent_id` refers to an existing node before passing it to
 * handleBatchDesign. batch_design's underlying `insertNodeInTree` silently
 * returns the original tree when the parent is missing, producing a
 * success-looking response (binding + nodeId) with an orphaned node that
 * never lands on disk. Element tools (add_scroll_row_v0 / add_bottom_nav_v0
 * / add_activity_ring_v0) must fail fast with a clear error instead.
 *
 * Skips validation when parent_id is falsy (root-level insertion is always
 * valid). Throws a descriptive Error otherwise.
 */
export async function ensureParentExists(params: {
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}): Promise<void> {
  if (!params.parent_id) return;
  const fp = resolveDocPath(params.filePath);
  const doc = await openDocument(fp);
  const children = getDocChildren(doc, params.pageId);
  const found = findNodeInTree(children, params.parent_id);
  if (!found) {
    throw new Error(
      `parent_id "${params.parent_id}" not found in document${
        params.pageId ? ` (pageId=${params.pageId})` : ''
      }. Pass a valid parent node id or omit parent_id for root-level insertion.`,
    );
  }
}

/**
 * Walk a node subtree and stamp every node with a fresh id. batch_design's
 * downstream DSL only assigns an id to the TOP-level inserted node —
 * nested children arrive unchanged, leaving them unreferenceable by any
 * later tree operation.
 */
export function assignIdsRecursively(node: Record<string, unknown>): void {
  if (typeof node.id !== 'string') node.id = generateId();
  const children = node.children;
  if (Array.isArray(children)) {
    for (const child of children) {
      if (child && typeof child === 'object') {
        assignIdsRecursively(child as Record<string, unknown>);
      }
    }
  }
}

/**
 * Build the canonical scroll-row wrapper taught in
 * `packages/pen-ai-skills/skills/phases/generation/overflow.md` §HORIZONTAL
 * SCROLL ROWS: outer wrapper (fill_container + clipContent + vertical) >
 * inner row (fit_content + horizontal + gap + padding=[0,20]) > children.
 *
 * Shared by all three narrow row tools (add_card_row_v0 /
 * add_metric_row_v0 / add_nav_chip_row_v0). Each tool only differs in the
 * per-item node builder; the wrapper is identical.
 */
export function buildScrollWrapper(opts: {
  rowName: string;
  innerChildren: Record<string, unknown>[];
  gap: number;
}): Record<string, unknown> {
  return {
    type: 'frame',
    name: opts.rowName,
    role: 'scroll-row-wrapper',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    clipContent: true,
    children: [
      {
        type: 'frame',
        name: 'Scroll Inner Row',
        role: 'scroll-row',
        width: 'fit_content',
        height: 'fit_content',
        layout: 'horizontal',
        gap: opts.gap,
        padding: [0, 20],
        children: opts.innerChildren,
      },
    ],
  };
}

/**
 * Insert a fully-built node subtree via handleBatchDesign's single-insert
 * DSL. Centralizes the parent_ref serialization + batch_design call
 * shape used by every element tool.
 *
 * Safety invariants (enforced here, not per-handler):
 *   1. parent_id is JSON.stringify'd so ids containing quotes / backslashes
 *      cannot escape the DSL quoting and inject additional operations
 *   2. per-item batch_design errors are re-thrown rather than silently
 *      collected in the result.errors array
 *   3. **post-insert verification**: re-read the document and confirm the
 *      inserted nodeId is actually present. Guards against silent no-op
 *      paths in the DSL parser — notably `resolveRef` (batch-design.ts)
 *      does simplistic `/^"|"$/g` quote-stripping and does NOT JSON-
 *      unescape, so parent_ids containing `"` or `\` pass JSON.stringify
 *      cleanly but the parser extracts a different literal, causing
 *      insertNodeInTree to silently return the original tree. The
 *      read-back check is the single source of truth for "did the
 *      insert actually land?" regardless of parser subtleties.
 */
export async function insertElementTree(args: {
  binding: string;
  tree: Record<string, unknown>;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  // ---- Pre-check: reject parent_ids that cannot safely round-trip through
  // batch_design's DSL parser. This runs BEFORE any disk write so we don't
  // have to roll back. See simulateDslParentResolve JSDoc for the exact
  // rule. Covers ids containing `"` or `\` (nanoid never produces these,
  // but imported / migrated documents might).
  if (args.parent_id && simulateDslParentResolve(args.parent_id) !== args.parent_id) {
    throw new Error(
      `parent_id ${JSON.stringify(args.parent_id)} contains characters (quote or backslash) ` +
        `that cannot be safely passed through batch_design's DSL parser. The parser strips one ` +
        `leading/trailing quote and does NOT JSON-unescape, so the resolved id would differ ` +
        `from the value you provided. Rename the parent node to remove these characters, or ` +
        `omit parent_id to insert at root.`,
    );
  }

  const resolvedFp = resolveDocPath(args.filePath);
  const isLive = resolvedFp === LIVE_CANVAS_PATH;

  // ---- Snapshot for rollback (file-backed docs only; live canvas can't
  // atomically roll back because pushLiveDocument is a one-way push).
  let fileSnapshot: string | null = null;
  if (!isLive) {
    try {
      fileSnapshot = await readFile(resolvedFp, 'utf-8');
    } catch {
      // File may not exist yet — treat as empty snapshot (restore by delete)
      fileSnapshot = null;
    }
  }

  const parentRef = args.parent_id ? JSON.stringify(args.parent_id) : 'null';
  const dsl = `${args.binding}=I(${parentRef}, ${JSON.stringify(args.tree)})`;
  const result = await handleBatchDesign({
    operations: dsl,
    filePath: args.filePath,
    pageId: args.pageId,
    postProcess: false,
  });

  const rollback = async (reason: string): Promise<never> => {
    if (!isLive && fileSnapshot !== null) {
      // Restore disk content first so on-disk state is correct even if
      // the subsequent live-canvas sync fails.
      await writeFile(resolvedFp, fileSnapshot, 'utf-8');
      invalidateCache(resolvedFp);
      // saveDocument is DUAL-WRITE for file-backed paths (document-manager.ts:411):
      // it writes to disk AND calls pushLiveDocument. Our earlier
      // handleBatchDesign call already pushed the bad insert to the live
      // canvas; we must push the restored doc so the renderer state
      // matches disk again. If no sync URL is active, pushLiveDocument
      // is a no-op, so this is safe in all scenarios.
      try {
        const restoredDoc = await openDocument(resolvedFp);
        await saveDocument(resolvedFp, restoredDoc);
      } catch (syncErr) {
        // Disk is the source of truth and is already restored; surface
        // the sync failure so the caller knows live canvas may be stale
        // but don't swallow the original insert failure.
        throw new Error(
          `${reason} (document restored to pre-insert state on disk, but re-syncing live canvas ` +
            `after rollback failed: ${syncErr instanceof Error ? syncErr.message : String(syncErr)}. ` +
            `Live canvas may show stale state until next refresh.)`,
        );
      }
    }
    throw new Error(
      `${reason} ${isLive ? '(live canvas cannot be atomically rolled back — the bad insert may still be visible until next refresh)' : '(document restored to pre-insert state on disk and re-synced to live canvas)'}`,
    );
  };

  if (result.errors && result.errors.length > 0) {
    const summary = result.errors.map((e) => `${e.line.slice(0, 80)}: ${e.error}`).join('; ');
    await rollback(`Element tool insert failed: ${summary}`);
  }
  const insertedId = result.results[0]?.nodeId;
  if (!insertedId) {
    await rollback(
      'Element tool insert returned no nodeId; batch_design did not report any result.',
    );
  }

  // ---- Post-check: verify insertion actually landed in the expected
  // location. With the pre-check above these paths should be unreachable
  // for well-formed parent_ids, but we keep them as defense-in-depth for
  // future DSL-parser changes or silent-failure modes we haven't thought of.
  const postDoc = await openDocument(resolvedFp);
  const postChildren = getDocChildren(postDoc, args.pageId);
  const landed = findNodeInTree(postChildren, insertedId as string);
  if (!landed) {
    await rollback(
      `Element tool insert silently failed: inserted node ${insertedId} is not present in the ` +
        `document after insertion. parent_id=${JSON.stringify(args.parent_id)}, pageId=${JSON.stringify(args.pageId)}.`,
    );
  }
  if (args.parent_id) {
    const actualParent = findParentInTree(postChildren, insertedId as string);
    const actualParentId = actualParent?.id ?? null;
    if (actualParentId !== args.parent_id) {
      await rollback(
        `Element tool insert landed under the wrong parent: expected ` +
          `${JSON.stringify(args.parent_id)}, got ${JSON.stringify(actualParentId ?? 'root')}.`,
      );
    }
  }
  return result;
}
