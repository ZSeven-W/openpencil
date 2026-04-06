// packages/pen-core/src/merge/node-merge.ts
//
// Pure 3-way merge of PenDocument trees.

import type { PenDocument, PenNode } from '@zseven-w/pen-types';
import {
  indexNodesById,
  nodeFieldsEqual,
  stripChildren,
  jsonEqual,
  type IndexedNode,
} from './merge-helpers.js';

export interface MergeInput {
  base: PenDocument;
  ours: PenDocument;
  theirs: PenDocument;
}

export type NodeConflictReason =
  | 'both-modified-same-field'
  | 'modify-vs-delete'
  | 'add-vs-add-different'
  | 'reparent-conflict';

export interface NodeConflict {
  pageId: string | null;
  nodeId: string;
  reason: NodeConflictReason;
  base: PenNode | null;
  ours: PenNode | null;
  theirs: PenNode | null;
}

export type DocFieldName = 'variables' | 'themes' | 'pages-order' | 'name' | 'version';

export interface DocFieldConflict {
  field: DocFieldName;
  path: string;
  base: unknown;
  ours: unknown;
  theirs: unknown;
}

export interface MergeResult {
  merged: PenDocument;
  nodeConflicts: NodeConflict[];
  docFieldConflicts: DocFieldConflict[];
}

/**
 * Per-id decision the classification step produces. Invariants:
 *   - Every id in the merged-id union is either present in the decisions map
 *     OR omitted entirely. A null `mergedNode` means "explicitly deleted by
 *     the merge"; an absent map entry means "the rebuild step shouldn't see
 *     this id at all" (which currently never happens — every id produces a
 *     decision). The two states are kept distinct so the rebuild step in
 *     Task 9 can assert on its inputs.
 *   - When `mergedNode` is non-null, `mergedFields` excludes the `children`
 *     field. The rebuild step in Task 9 produces children by walking the
 *     decision map, not by reading the merged-node value.
 *   - `index` is a HINT for ordering inside the new parent. The rebuild step
 *     prefers ours' indices when both sides modified the same parent (this
 *     matches the spec's "fall back to ours" tiebreaker).
 */
interface NodeDecision {
  /** The merged node, with `children` left empty (the rebuild step fills them). null = deleted */
  mergedNode: PenNode | null;
  /** Where the merged node should live; null when mergedNode is null */
  pageId: string | null;
  parentId: string | null;
  /** Position hint within parent.children; the rebuild step uses ours' index when available */
  index: number;
}

/**
 * Pure 3-way merge of PenDocument trees.
 *
 * Implementation is split:
 *   - Tasks 7-8: classify each node id, produce node-level conflicts
 *   - Task 9: rebuild the merged tree from decisions
 *   - Task 10: merge doc-level fields (variables, themes, pages-order, name, version)
 *   - Task 11: tests
 */
export function mergeDocuments(input: MergeInput): MergeResult {
  const { base, ours, theirs } = input;
  const baseIdx = indexNodesById(base);
  const oursIdx = indexNodesById(ours);
  const theirsIdx = indexNodesById(theirs);

  const allIds = new Set<string>([
    ...baseIdx.keys(),
    ...oursIdx.keys(),
    ...theirsIdx.keys(),
  ]);

  const decisions = new Map<string, NodeDecision>();
  const nodeConflicts: NodeConflict[] = [];

  for (const id of allIds) {
    const b = baseIdx.get(id);
    const o = oursIdx.get(id);
    const t = theirsIdx.get(id);
    const result = classifyNode(id, b, o, t);
    if (result.conflict) nodeConflicts.push(result.conflict);
    if (result.decision) decisions.set(id, result.decision);
  }

  const merged = rebuildDocument(ours, decisions);

  // TODO Task 10: merge doc-level fields

  return {
    merged,
    nodeConflicts,
    docFieldConflicts: [],
  };
}

/**
 * Rebuild a PenDocument from a decision map. Uses ours' page structure as the
 * scaffold (so users see their own page list). For each merged node, attach it
 * to its merged parent at its merged index.
 *
 * Algorithm:
 *   1. Group decisions by (pageId, parentId).
 *   2. For each page in ours (or the legacy single page):
 *      a. Build the page's top-level children list from decisions where
 *         parentId === null and pageId === thisPageId.
 *      b. Recursively populate each container's children from decisions where
 *         parentId === containerId.
 *   3. Within a parent, sort by `index` ascending; ties broken by id for
 *      determinism.
 */
function rebuildDocument(
  scaffold: PenDocument,
  decisions: Map<string, NodeDecision>,
): PenDocument {
  // Group by parent for fast child lookup.
  const byParent = new Map<string, NodeDecision[]>();
  for (const decision of decisions.values()) {
    if (!decision.mergedNode) continue;
    const key = `${decision.pageId ?? '_'}::${decision.parentId ?? '_'}`;
    let bucket = byParent.get(key);
    if (!bucket) {
      bucket = [];
      byParent.set(key, bucket);
    }
    bucket.push(decision);
  }

  function buildChildren(pageId: string | null, parentId: string | null): PenNode[] {
    const key = `${pageId ?? '_'}::${parentId ?? '_'}`;
    const bucket = byParent.get(key);
    if (!bucket) return [];
    // Stable sort by (index, id).
    const sorted = [...bucket].sort((a, b) => {
      if (a.index !== b.index) return a.index - b.index;
      const ai = a.mergedNode ? a.mergedNode.id : '';
      const bi = b.mergedNode ? b.mergedNode.id : '';
      return ai.localeCompare(bi);
    });
    return sorted.map((decision) => {
      const node = decision.mergedNode!;
      const children = buildChildren(pageId, node.id);
      // Only attach a children array if the original node type supports it
      // (containers). Following the existing PenDocument convention, attach
      // the children only if the node already had `children` or if there are
      // any children to attach.
      if (children.length > 0 || (node as { children?: PenNode[] }).children !== undefined) {
        return { ...node, children } as PenNode;
      }
      return node;
    });
  }

  // Use ours' page structure as the scaffold.
  if (scaffold.pages && scaffold.pages.length > 0) {
    const newPages = scaffold.pages.map((page) => ({
      id: page.id,
      name: page.name,
      children: buildChildren(page.id, null),
    }));
    return {
      ...scaffold,
      pages: newPages,
      children: [],
    };
  }
  // Legacy single-page mode.
  return {
    ...scaffold,
    children: buildChildren(null, null),
  };
}

/**
 * Classify a single node id across the three trees and produce both a decision
 * (the merged node value to use, or null for delete) and an optional conflict.
 *
 * The 7-grid:
 *   ✓✓✓ → field-level merge (may produce conflict)
 *   ✓✓✗ → modify-vs-delete OR clean delete
 *   ✓✗✓ → modify-vs-delete OR clean delete (symmetric)
 *   ✓✗✗ → both deleted; no merged node
 *   ✗✓✓ → add-vs-add (equal → take ours; different → conflict)
 *   ✗✓✗ → only ours added; take ours
 *   ✗✗✓ → only theirs added; take theirs
 */
function classifyNode(
  id: string,
  b: IndexedNode | undefined,
  o: IndexedNode | undefined,
  t: IndexedNode | undefined,
): { decision: NodeDecision | null; conflict: NodeConflict | null } {
  // ✓✓✓
  if (b && o && t) {
    return mergeThreeWay(id, b, o, t);
  }
  // ✓✓✗ — theirs deleted
  if (b && o && !t) {
    if (nodeFieldsEqual(b.node, o.node)) {
      // ours unchanged, theirs deleted → delete
      return { decision: null, conflict: null };
    }
    // ours modified, theirs deleted → conflict
    return {
      decision: {
        mergedNode: o.node,
        pageId: o.pageId,
        parentId: o.parentId,
        index: o.index,
      },
      conflict: {
        pageId: o.pageId,
        nodeId: id,
        reason: 'modify-vs-delete',
        base: b.node,
        ours: o.node,
        theirs: null,
      },
    };
  }
  // ✓✗✓ — ours deleted
  if (b && !o && t) {
    if (nodeFieldsEqual(b.node, t.node)) {
      return { decision: null, conflict: null };
    }
    return {
      decision: {
        mergedNode: t.node,
        pageId: t.pageId,
        parentId: t.parentId,
        index: t.index,
      },
      conflict: {
        pageId: t.pageId,
        nodeId: id,
        reason: 'modify-vs-delete',
        base: b.node,
        ours: null,
        theirs: t.node,
      },
    };
  }
  // ✓✗✗ — both deleted
  if (b && !o && !t) {
    return { decision: null, conflict: null };
  }
  // ✗✓✓ — both added
  if (!b && o && t) {
    if (nodeFieldsEqual(o.node, t.node)) {
      return {
        decision: {
          mergedNode: o.node,
          pageId: o.pageId,
          parentId: o.parentId,
          index: o.index,
        },
        conflict: null,
      };
    }
    return {
      decision: {
        mergedNode: o.node,
        pageId: o.pageId,
        parentId: o.parentId,
        index: o.index,
      },
      conflict: {
        pageId: o.pageId,
        nodeId: id,
        reason: 'add-vs-add-different',
        base: null,
        ours: o.node,
        theirs: t.node,
      },
    };
  }
  // ✗✓✗ — ours only
  if (!b && o && !t) {
    return {
      decision: {
        mergedNode: o.node,
        pageId: o.pageId,
        parentId: o.parentId,
        index: o.index,
      },
      conflict: null,
    };
  }
  // ✗✗✓ — theirs only
  if (!b && !o && t) {
    return {
      decision: {
        mergedNode: t.node,
        pageId: t.pageId,
        parentId: t.parentId,
        index: t.index,
      },
      conflict: null,
    };
  }
  // Unreachable: id appeared in the union but is in none of the three indices.
  return { decision: null, conflict: null };
}

/**
 * Three-way present case: do field-level merge between b, o, t and detect both
 * `both-modified-same-field` and `reparent-conflict` cases.
 */
function mergeThreeWay(
  id: string,
  b: IndexedNode,
  o: IndexedNode,
  t: IndexedNode,
): { decision: NodeDecision | null; conflict: NodeConflict | null } {
  // First: detect reparent conflict.
  const oursMoved = o.parentId !== b.parentId || o.pageId !== b.pageId;
  const theirsMoved = t.parentId !== b.parentId || t.pageId !== b.pageId;
  let mergedParentId = b.parentId;
  let mergedPageId = b.pageId;
  let reparentConflict = false;
  if (oursMoved && theirsMoved) {
    if (o.parentId === t.parentId && o.pageId === t.pageId) {
      // Both moved to the same place — auto-merge to that.
      mergedParentId = o.parentId;
      mergedPageId = o.pageId;
    } else {
      // Diverging moves.
      reparentConflict = true;
      mergedParentId = o.parentId;
      mergedPageId = o.pageId;
    }
  } else if (oursMoved) {
    mergedParentId = o.parentId;
    mergedPageId = o.pageId;
  } else if (theirsMoved) {
    mergedParentId = t.parentId;
    mergedPageId = t.pageId;
  }

  // Second: per-field 3-way merge of atomic fields.
  const baseFields = stripChildren(b.node) as Record<string, unknown>;
  const oursFields = stripChildren(o.node) as Record<string, unknown>;
  const theirsFields = stripChildren(t.node) as Record<string, unknown>;
  const allKeys = new Set<string>([
    ...Object.keys(baseFields),
    ...Object.keys(oursFields),
    ...Object.keys(theirsFields),
  ]);

  const mergedFields: Record<string, unknown> = {};
  let fieldConflict = false;
  for (const key of allKeys) {
    const bv = baseFields[key];
    const ov = oursFields[key];
    const tv = theirsFields[key];
    const oursChanged = !jsonEqual(ov, bv);
    const theirsChanged = !jsonEqual(tv, bv);
    if (!oursChanged && !theirsChanged) {
      if (bv !== undefined) mergedFields[key] = bv;
      continue;
    }
    if (oursChanged && !theirsChanged) {
      if (ov !== undefined) mergedFields[key] = ov;
      continue;
    }
    if (!oursChanged && theirsChanged) {
      if (tv !== undefined) mergedFields[key] = tv;
      continue;
    }
    // Both changed.
    if (jsonEqual(ov, tv)) {
      if (ov !== undefined) mergedFields[key] = ov;
      continue;
    }
    // Conflict — take ours as placeholder.
    fieldConflict = true;
    if (ov !== undefined) mergedFields[key] = ov;
  }

  const mergedNode = mergedFields as unknown as PenNode;

  if (reparentConflict) {
    return {
      decision: {
        mergedNode,
        pageId: mergedPageId,
        parentId: mergedParentId,
        index: o.index,
      },
      conflict: {
        pageId: mergedPageId,
        nodeId: id,
        reason: 'reparent-conflict',
        base: b.node,
        ours: o.node,
        theirs: t.node,
      },
    };
  }

  if (fieldConflict) {
    return {
      decision: {
        mergedNode,
        pageId: mergedPageId,
        parentId: mergedParentId,
        index: o.index,
      },
      conflict: {
        pageId: mergedPageId,
        nodeId: id,
        reason: 'both-modified-same-field',
        base: b.node,
        ours: o.node,
        theirs: t.node,
      },
    };
  }

  return {
    decision: {
      mergedNode,
      pageId: mergedPageId,
      parentId: mergedParentId,
      index: o.index,
    },
    conflict: null,
  };
}
