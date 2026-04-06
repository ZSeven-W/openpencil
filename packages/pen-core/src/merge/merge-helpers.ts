// packages/pen-core/src/merge/merge-helpers.ts
//
// Pure indexing and walking utilities shared by node-diff.ts and node-merge.ts.
// Not exported from the module's public surface — these are internal helpers.

import type { PenDocument, PenNode } from '@zseven-w/pen-types';

/**
 * Information stored per indexed node, capturing both the node itself and
 * the structural context that the merge algorithm needs (page, parent, index).
 */
export interface IndexedNode {
  /** null for legacy single-page documents (no pages array) */
  pageId: string | null;
  /** null when the node sits at the top of a page (or top of legacy children) */
  parentId: string | null;
  /** position within parent.children (or top-level array) */
  index: number;
  node: PenNode;
}

/**
 * Walk a document and produce a Map<nodeId, IndexedNode>. Handles both
 * `pages` and legacy `children` shapes uniformly.
 */
export function indexNodesById(doc: PenDocument): Map<string, IndexedNode> {
  const out = new Map<string, IndexedNode>();
  for (const page of getAllPages(doc)) {
    walk(page.children, page.id, null, out);
  }
  return out;
}

function walk(
  nodes: PenNode[],
  pageId: string | null,
  parentId: string | null,
  out: Map<string, IndexedNode>,
): void {
  nodes.forEach((node, index) => {
    out.set(node.id, { pageId, parentId, index, node });
    const children = (node as { children?: PenNode[] }).children;
    if (children && children.length > 0) {
      walk(children, pageId, node.id, out);
    }
  });
}

/**
 * Normalize a document into a list of pages, regardless of whether it uses
 * the explicit `pages` array or the legacy `children` array. Legacy mode
 * produces a single synthetic page with `id = null`.
 */
export function getAllPages(doc: PenDocument): Array<{ id: string | null; children: PenNode[] }> {
  if (doc.pages && doc.pages.length > 0) {
    return doc.pages.map((p) => ({ id: p.id, children: p.children }));
  }
  return [{ id: null, children: doc.children ?? [] }];
}

/**
 * Compare two nodes by atomic fields only (everything except `children`).
 * Returns true if they would produce the same fields-only diff.
 */
export function nodeFieldsEqual(a: PenNode, b: PenNode): boolean {
  const aFields = stripChildren(a);
  const bFields = stripChildren(b);
  return jsonEqual(aFields, bFields);
}

/**
 * Return a shallow copy of `node` with the `children` field removed.
 * Used everywhere we want to compare or diff a node's atomic fields without
 * the recursive children noise.
 */
export function stripChildren<T extends PenNode>(node: T): Omit<T, 'children'> {
  // Use destructuring; TS-safe.
  const copy = { ...node } as T & { children?: unknown };
  delete copy.children;
  return copy as Omit<T, 'children'>;
}

/**
 * Deep value-equality via JSON canonicalization. Used by nodeFieldsEqual and
 * by the doc-field merge for comparing variable values, theme entries, etc.
 *
 * Important: this is intentionally simple. PenNode field values are JSON-safe
 * (numbers, strings, booleans, plain objects, arrays). No Dates, Maps, Sets,
 * functions, etc. live in PenDocument.
 */
export function jsonEqual(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (a === null || b === null) return false;
  if (typeof a !== typeof b) return false;
  if (typeof a !== 'object') return false;
  // Both are non-null objects (or arrays)
  return JSON.stringify(canonicalize(a)) === JSON.stringify(canonicalize(b));
}

/**
 * Sort object keys recursively so JSON.stringify produces a deterministic string
 * regardless of insertion order.
 */
function canonicalize(value: unknown): unknown {
  if (value === null || typeof value !== 'object') return value;
  if (Array.isArray(value)) return value.map(canonicalize);
  const sortedKeys = Object.keys(value as Record<string, unknown>).sort();
  const out: Record<string, unknown> = {};
  for (const key of sortedKeys) {
    out[key] = canonicalize((value as Record<string, unknown>)[key]);
  }
  return out;
}
