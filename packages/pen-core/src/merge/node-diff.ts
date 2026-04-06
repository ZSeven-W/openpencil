// packages/pen-core/src/merge/node-diff.ts
//
// One-direction diff: compute the patches needed to transform `base` into `next`.

import type { PenNode, PenDocument } from '@zseven-w/pen-types';
import { indexNodesById, nodeFieldsEqual, stripChildren, jsonEqual } from './merge-helpers.js';

export interface NodePatch {
  op: 'add' | 'remove' | 'modify' | 'move';
  /** null for legacy single-page documents (no pages array) */
  pageId: string | null;
  nodeId: string;
  /** for `add` / `move`: target parent id; null = top-level on the page */
  parentId?: string | null;
  /** for `add` / `move`: target index within parent's children */
  index?: number;
  /** for `modify`: only the atomic fields that changed (never includes `children`) */
  fields?: Partial<PenNode>;
  /** for `modify`: pre-change values of those same fields, used by 3-way merge */
  beforeFields?: Partial<PenNode>;
}

/**
 * Compute the patches needed to transform `base` into `next`. Walks both trees
 * by node id and emits one of `add` / `remove` / `modify` / `move` per change.
 *
 * Algorithm:
 *   1. Index both documents by node id.
 *   2. For each id in base ∪ next:
 *      - in next only → `add`
 *      - in base only → `remove`
 *      - in both → check parent/index (if changed → `move`) and atomic fields
 *        (if any changed → `modify`); a single id may produce both a `move`
 *        and a `modify`.
 */
export function diffDocuments(base: PenDocument, next: PenDocument): NodePatch[] {
  const baseIdx = indexNodesById(base);
  const nextIdx = indexNodesById(next);
  const allIds = new Set<string>([...baseIdx.keys(), ...nextIdx.keys()]);
  const patches: NodePatch[] = [];

  for (const id of allIds) {
    const b = baseIdx.get(id);
    const n = nextIdx.get(id);

    if (!b && n) {
      // Added.
      patches.push({
        op: 'add',
        pageId: n.pageId,
        nodeId: id,
        parentId: n.parentId,
        index: n.index,
        fields: stripChildren(n.node) as Partial<PenNode>,
      });
      continue;
    }

    if (b && !n) {
      // Removed.
      patches.push({
        op: 'remove',
        pageId: b.pageId,
        nodeId: id,
      });
      continue;
    }

    if (b && n) {
      // Present in both. Check for `move` (parent or page changed) and `modify`
      // (atomic fields changed). They are independent — one node may produce
      // both kinds of patches.
      const moved = b.parentId !== n.parentId || b.pageId !== n.pageId || b.index !== n.index;
      if (moved) {
        patches.push({
          op: 'move',
          pageId: n.pageId,
          nodeId: id,
          parentId: n.parentId,
          index: n.index,
        });
      }
      if (!nodeFieldsEqual(b.node, n.node)) {
        const { changed, before } = diffFields(b.node, n.node);
        patches.push({
          op: 'modify',
          pageId: n.pageId,
          nodeId: id,
          fields: changed,
          beforeFields: before,
        });
      }
    }
  }

  return patches;
}

/**
 * Compute the per-field delta between two nodes. Returns the keys that changed
 * (in `next`) along with the original values (from `base`). Skips the `children`
 * field — it's handled separately by the recursive walk.
 */
function diffFields(
  base: PenNode,
  next: PenNode,
): { changed: Partial<PenNode>; before: Partial<PenNode> } {
  const baseStripped = stripChildren(base) as Record<string, unknown>;
  const nextStripped = stripChildren(next) as Record<string, unknown>;
  const allKeys = new Set<string>([...Object.keys(baseStripped), ...Object.keys(nextStripped)]);
  const changed: Record<string, unknown> = {};
  const before: Record<string, unknown> = {};
  for (const key of allKeys) {
    if (!jsonEqual(baseStripped[key], nextStripped[key])) {
      changed[key] = nextStripped[key];
      before[key] = baseStripped[key];
    }
  }
  return {
    changed: changed as Partial<PenNode>,
    before: before as Partial<PenNode>,
  };
}
