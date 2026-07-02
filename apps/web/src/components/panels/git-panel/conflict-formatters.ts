// apps/web/src/components/panels/git-panel/conflict-formatters.ts
//
// Pure helpers for formatting conflict-resolution UI labels. No React, no store
// dependencies — safe to import anywhere and easy to unit-test.

import type { PenDocument, PenNode } from '@/types/pen';
import type { GitConflictBag, GitConflictResolution } from '@/services/git-types';

// ---------------------------------------------------------------------------
// Document-order conflict sorting
// ---------------------------------------------------------------------------

/**
 * A node conflict entry with optional resolution state (matches ConflictBagState's
 * nodeConflicts value shape — typed here to avoid importing from git-store-types
 * which carries Zustand store baggage).
 */
export type NodeConflictEntry = GitConflictBag['nodeConflicts'][number] & {
  resolution?: GitConflictResolution;
};

export type FieldConflictEntry = GitConflictBag['docFieldConflicts'][number] & {
  resolution?: GitConflictResolution;
};

/**
 * Walk a PenNode tree depth-first, invoking `visit` for every node.
 * Visits `node.children` when present (not all PenNode variants have children).
 */
function walkTreeDfs(nodes: PenNode[], visit: (n: PenNode) => void): void {
  for (const node of nodes) {
    visit(node);
    if ('children' in node && node.children && (node.children as PenNode[]).length > 0) {
      walkTreeDfs(node.children as PenNode[], visit);
    }
  }
}

/**
 * Produce an ordered flat list of conflict entries for the conflict-list UI.
 *
 * Ordering rules:
 *   1. Node conflicts in document tree order (depth-first).  The conflict Map
 *      is keyed by `node:<pageId|_>:<nodeId>`, so we derive the nodeId by
 *      splitting on ":" and taking the last segment.
 *   2. Doc-field conflicts are document-level with no tree position.  They are
 *      emitted after the in-tree node conflicts, sorted alphabetically by the
 *      `path` field for a stable, user-readable sequence.
 *   3. Orphan node conflicts — conflicts whose nodeId is not present in the
 *      current document tree (e.g. theirs deleted the node) — are emitted last,
 *      preserving Map insertion order for stability.
 */
export function orderConflicts(
  document: PenDocument,
  nodeConflicts: Map<string, NodeConflictEntry>,
  fieldConflicts: Map<string, FieldConflictEntry>,
): Array<NodeConflictEntry | FieldConflictEntry> {
  const result: Array<NodeConflictEntry | FieldConflictEntry> = [];

  // Build a set of node-conflict entries ordered by document tree position.
  const emitted = new Set<string>();

  // Walk each page separately so we can scope conflict matching by pageId.
  // For single-page docs (no doc.pages), synthesise a virtual page with
  // pageId === null so the key `node:_:<nodeId>` still matches.
  //
  // Node conflict key schema: `node:<pageId|_>:<nodeId>`
  //   pageId stored in entry.pageId (null for single-page docs)
  const pages: Array<{ pageId: string | null; children: PenNode[] }> =
    document?.pages && document.pages.length > 0
      ? document.pages.map((p) => ({ pageId: p.id, children: p.children }))
      : [{ pageId: null, children: document?.children ?? [] }];

  for (const { pageId, children } of pages) {
    walkTreeDfs(children, (node) => {
      // Each node conflict's key is `node:<pageId|_>:<nodeId>`.  We need to
      // find the entry whose nodeId AND pageId both match this node.
      for (const [key, entry] of nodeConflicts) {
        if (emitted.has(key)) continue;
        if (entry.nodeId === node.id && entry.pageId === pageId) {
          result.push(entry);
          emitted.add(key);
          break; // At most one conflict per (pageId, nodeId) pair.
        }
      }
    });
  }

  // Orphan node conflicts: referenced nodeId not found in current tree.
  for (const [key, entry] of nodeConflicts) {
    if (!emitted.has(key)) {
      result.push(entry);
      emitted.add(key);
    }
  }

  // Doc-field conflicts: alphabetical by path.
  const fieldEntries = Array.from(fieldConflicts.values());
  fieldEntries.sort((a, b) => a.path.localeCompare(b.path));
  for (const entry of fieldEntries) {
    result.push(entry);
  }

  return result;
}

// ---------------------------------------------------------------------------
// Reason label mapping
// ---------------------------------------------------------------------------

/** Human-readable label for a node conflict reason code. */
export function formatConflictReason(
  reason: GitConflictBag['nodeConflicts'][number]['reason'],
): string {
  switch (reason) {
    case 'both-modified-same-field':
      return 'Both sides modified the same field';
    case 'modify-vs-delete':
      return 'One side modified, the other deleted';
    case 'add-vs-add-different':
      return 'Both sides added a node with different content';
    case 'reparent-conflict':
      return 'Both sides moved this node to different parents';
    default:
      return 'Unknown conflict';
  }
}

// ---------------------------------------------------------------------------
// JSON pretty-printing
// ---------------------------------------------------------------------------

/**
 * Pretty-print a value as indented JSON. Returns a placeholder string on
 * failure (e.g. circular reference, null value).
 */
export function prettyJson(value: unknown): string {
  if (value === undefined) return '(absent)';
  if (value === null) return 'null';
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return '(unserializable)';
  }
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

/**
 * Parse a JSON string and return `{ ok: true, value }` or `{ ok: false, error }`.
 * Used by the manual JSON editor to give instant parse-error feedback.
 */
export function safeParseJson(
  text: string,
): { ok: true; value: unknown } | { ok: false; error: string } {
  if (text.trim() === '') {
    return { ok: false, error: 'JSON cannot be empty' };
  }
  try {
    const value = JSON.parse(text);
    return { ok: true, value };
  } catch (err) {
    return { ok: false, error: err instanceof Error ? err.message : 'Invalid JSON' };
  }
}

/**
 * Validate that a parsed JSON value is a PenNode-like object with the expected
 * `nodeId`. Returns a validation error string or null when valid.
 *
 * We do a minimal structural check — the backend performs full schema
 * validation on applyMerge, so here we just need enough to give useful
 * feedback in the UI before the IPC round-trip.
 */
export function validateNodeJson(value: unknown, expectedNodeId: string): string | null {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    return 'Value must be a JSON object representing a node';
  }
  const obj = value as Record<string, unknown>;
  if (!obj.id) {
    return 'Node must have an "id" field';
  }
  if (obj.id !== expectedNodeId) {
    return `Node "id" must remain "${expectedNodeId}"`;
  }
  if (!obj.type || typeof obj.type !== 'string') {
    return 'Node must have a "type" string field';
  }
  return null;
}

// ---------------------------------------------------------------------------
// Truncation helpers
// ---------------------------------------------------------------------------

/** Truncate a string for display, adding an ellipsis when it exceeds maxLen. */
export function truncate(s: string, maxLen: number): string {
  if (s.length <= maxLen) return s;
  return s.slice(0, maxLen - 1) + '\u2026';
}
