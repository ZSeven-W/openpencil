import { openDocument, resolveDocPath } from '../document-manager';
import { findNodeInTree, findParentInTree, getDocChildren } from '../utils/node-operations';
import { generateId } from '../utils/id';
import { handleBatchDesign } from './batch-design';

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
  const parentRef = args.parent_id ? JSON.stringify(args.parent_id) : 'null';
  const dsl = `${args.binding}=I(${parentRef}, ${JSON.stringify(args.tree)})`;
  const result = await handleBatchDesign({
    operations: dsl,
    filePath: args.filePath,
    pageId: args.pageId,
    postProcess: false,
  });
  if (result.errors && result.errors.length > 0) {
    const summary = result.errors.map((e) => `${e.line.slice(0, 80)}: ${e.error}`).join('; ');
    throw new Error(`Element tool insert failed: ${summary}`);
  }
  const insertedId = result.results[0]?.nodeId;
  if (!insertedId) {
    throw new Error(
      'Element tool insert returned no nodeId; batch_design did not report any result',
    );
  }
  const fp = resolveDocPath(args.filePath);
  const postDoc = await openDocument(fp);
  const postChildren = getDocChildren(postDoc, args.pageId);
  const landed = findNodeInTree(postChildren, insertedId);
  if (!landed) {
    throw new Error(
      `Element tool insert silently failed: inserted node ${insertedId} is not present in the ` +
        `document after insertion. This usually means parent_id escaping did not match the ` +
        `document's actual id (batch_design's DSL parser strips quotes but does not JSON-unescape). ` +
        `parent_id=${JSON.stringify(args.parent_id)}, pageId=${JSON.stringify(args.pageId)}.`,
    );
  }
  // Parent-location verification: node exists in tree but may have landed
  // under the wrong parent. Can happen if batch_design's resolveRef
  // quote-strip produces a literal that matches a DIFFERENT node than the
  // one pre-check validated. Example: doc has both `A"B` (user intent)
  // AND `A\"B` (literal 4-char id with backslash); after JSON.stringify
  // + quote-strip, parser resolves to `A\"B` and inserts under it.
  // ensureParentExists and the "landed in tree" check both pass, but
  // the insert went to the wrong place.
  if (args.parent_id) {
    const actualParent = findParentInTree(postChildren, insertedId);
    const actualParentId = actualParent?.id ?? null;
    if (actualParentId !== args.parent_id) {
      throw new Error(
        `Element tool insert landed under the wrong parent: expected ` +
          `${JSON.stringify(args.parent_id)}, got ${JSON.stringify(actualParentId ?? 'root')}. ` +
          `This is typically a DSL-parser escape mismatch where the resolved parent id ` +
          `happens to collide with a different node's literal id.`,
      );
    }
  }
  return result;
}
