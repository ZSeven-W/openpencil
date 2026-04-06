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

  const allIds = new Set<string>([...baseIdx.keys(), ...oursIdx.keys(), ...theirsIdx.keys()]);

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
  const docFieldConflicts: DocFieldConflict[] = [];

  // Merge doc-level scalar fields (name, version)
  mergeDocScalarField(base, ours, theirs, 'name', merged, docFieldConflicts);
  mergeDocScalarField(base, ours, theirs, 'version', merged, docFieldConflicts);

  // Merge variables (object map)
  merged.variables = mergeRecord(
    base.variables,
    ours.variables,
    theirs.variables,
    'variables',
    docFieldConflicts,
  );
  if (merged.variables && Object.keys(merged.variables).length === 0) {
    delete merged.variables;
  }

  // Merge themes (object map of string[])
  merged.themes = mergeRecord(base.themes, ours.themes, theirs.themes, 'themes', docFieldConflicts);
  if (merged.themes && Object.keys(merged.themes).length === 0) {
    delete merged.themes;
  }

  // Merge pages order (only if both sides have pages)
  if (base.pages && ours.pages && theirs.pages) {
    const reorderConflict = detectPagesOrderConflict(base.pages, ours.pages, theirs.pages);
    if (reorderConflict) {
      docFieldConflicts.push(reorderConflict);
    }
  }

  return {
    merged,
    nodeConflicts,
    docFieldConflicts,
  };
}

/**
 * 3-way merge for a top-level scalar string field on PenDocument (name/version).
 */
function mergeDocScalarField(
  base: PenDocument,
  ours: PenDocument,
  theirs: PenDocument,
  field: 'name' | 'version',
  merged: PenDocument,
  conflicts: DocFieldConflict[],
): void {
  const bv = base[field];
  const ov = ours[field];
  const tv = theirs[field];
  if (jsonEqual(ov, bv) && jsonEqual(tv, bv)) {
    if (bv !== undefined) merged[field] = bv as string;
    return;
  }
  if (jsonEqual(ov, bv)) {
    if (tv !== undefined) merged[field] = tv as string;
    return;
  }
  if (jsonEqual(tv, bv)) {
    if (ov !== undefined) merged[field] = ov as string;
    return;
  }
  if (jsonEqual(ov, tv)) {
    if (ov !== undefined) merged[field] = ov as string;
    return;
  }
  // Both changed differently — conflict; take ours as placeholder.
  if (ov !== undefined) merged[field] = ov as string;
  conflicts.push({
    field,
    path: field,
    base: bv,
    ours: ov,
    theirs: tv,
  });
}

/**
 * 3-way merge for an object record (variables, themes). Walks every key from
 * the union and applies the per-key merge rules. Returns the merged record.
 */
function mergeRecord<V>(
  base: Record<string, V> | undefined,
  ours: Record<string, V> | undefined,
  theirs: Record<string, V> | undefined,
  fieldName: 'variables' | 'themes',
  conflicts: DocFieldConflict[],
): Record<string, V> | undefined {
  if (!base && !ours && !theirs) return undefined;
  const b = base ?? {};
  const o = ours ?? {};
  const t = theirs ?? {};
  const allKeys = new Set<string>([...Object.keys(b), ...Object.keys(o), ...Object.keys(t)]);
  const merged: Record<string, V> = {};
  for (const key of allKeys) {
    const bv = b[key];
    const ov = o[key];
    const tv = t[key];
    const inBase = key in b;
    const inOurs = key in o;
    const inTheirs = key in t;

    // Use the same 7-grid logic as nodes.
    if (inBase && inOurs && inTheirs) {
      // ✓✓✓
      if (jsonEqual(ov, bv) && jsonEqual(tv, bv)) {
        merged[key] = bv;
      } else if (jsonEqual(ov, bv)) {
        merged[key] = tv;
      } else if (jsonEqual(tv, bv)) {
        merged[key] = ov;
      } else if (jsonEqual(ov, tv)) {
        merged[key] = ov;
      } else {
        merged[key] = ov;
        conflicts.push({
          field: fieldName,
          path: `${fieldName}.${key}`,
          base: bv,
          ours: ov,
          theirs: tv,
        });
      }
    } else if (inBase && inOurs && !inTheirs) {
      // ✓✓✗ — theirs deleted
      if (jsonEqual(ov, bv)) {
        // ours unchanged → delete
      } else {
        // ours modified → conflict; take ours
        merged[key] = ov;
        conflicts.push({
          field: fieldName,
          path: `${fieldName}.${key}`,
          base: bv,
          ours: ov,
          theirs: undefined,
        });
      }
    } else if (inBase && !inOurs && inTheirs) {
      // ✓✗✓ — ours deleted
      if (jsonEqual(tv, bv)) {
        // theirs unchanged → delete
      } else {
        // theirs modified → conflict; take theirs
        merged[key] = tv;
        conflicts.push({
          field: fieldName,
          path: `${fieldName}.${key}`,
          base: bv,
          ours: undefined,
          theirs: tv,
        });
      }
    } else if (inBase && !inOurs && !inTheirs) {
      // ✓✗✗ — both deleted
    } else if (!inBase && inOurs && inTheirs) {
      // ✗✓✓ — both added
      if (jsonEqual(ov, tv)) {
        merged[key] = ov;
      } else {
        merged[key] = ov;
        conflicts.push({
          field: fieldName,
          path: `${fieldName}.${key}`,
          base: undefined,
          ours: ov,
          theirs: tv,
        });
      }
    } else if (!inBase && inOurs && !inTheirs) {
      merged[key] = ov;
    } else if (!inBase && !inOurs && inTheirs) {
      merged[key] = tv;
    }
  }
  return merged;
}

/**
 * Detect a `pages-order` conflict. Returns a DocFieldConflict if both sides
 * reordered pages differently from base; otherwise returns null. The merge
 * leaves the pages in ours' order (the rebuildDocument step already did this).
 */
function detectPagesOrderConflict(
  basePages: PenDocument['pages'],
  oursPages: PenDocument['pages'],
  theirsPages: PenDocument['pages'],
): DocFieldConflict | null {
  if (!basePages || !oursPages || !theirsPages) return null;
  const baseOrder = basePages.map((p) => p.id);
  const oursOrder = oursPages.map((p) => p.id);
  const theirsOrder = theirsPages.map((p) => p.id);
  const oursReordered = !sequenceEqual(baseOrder, oursOrder);
  const theirsReordered = !sequenceEqual(baseOrder, theirsOrder);
  if (!oursReordered || !theirsReordered) return null;
  if (sequenceEqual(oursOrder, theirsOrder)) return null;
  return {
    field: 'pages-order',
    path: 'pages-order',
    base: baseOrder,
    ours: oursOrder,
    theirs: theirsOrder,
  };
}

function sequenceEqual(a: string[], b: string[]): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    if (a[i] !== b[i]) return false;
  }
  return true;
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
function rebuildDocument(scaffold: PenDocument, decisions: Map<string, NodeDecision>): PenDocument {
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
