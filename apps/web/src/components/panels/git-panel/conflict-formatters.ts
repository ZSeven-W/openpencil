// apps/web/src/components/panels/git-panel/conflict-formatters.ts
//
// Pure helpers for formatting conflict-resolution UI labels. No React, no store
// dependencies — safe to import anywhere and easy to unit-test.

import type { GitConflictBag } from '@/services/git-types';

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
