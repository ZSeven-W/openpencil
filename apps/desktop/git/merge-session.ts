// apps/desktop/git/merge-session.ts
//
// In-flight merge state for a single repo. Stored on RepoSession.inflightMerge
// during the conflict resolution loop. The conflict id codec lives here too
// because it's the only string format both the engine and the tests share.

import type { MergeResult, NodeConflict, DocFieldConflict } from '@zseven-w/pen-core';
import type { PenNode } from '@zseven-w/pen-types';

/**
 * The user's choice for resolving a single conflict. Mirrors the spec's
 * ConflictResolution union.
 */
export type ConflictResolution =
  | { kind: 'ours' }
  | { kind: 'theirs' }
  | { kind: 'manual-node'; node: PenNode } // for node conflicts
  | { kind: 'manual-field'; value: unknown }; // for doc-field conflicts

/**
 * Wire-format conflict bag returned across IPC. The renderer wraps this in
 * Maps for resolution tracking. Each conflict has a stable id that the
 * renderer passes back via resolveConflict().
 */
export interface ConflictBag {
  nodeConflicts: Array<NodeConflict & { id: string }>;
  docFieldConflicts: Array<DocFieldConflict & { id: string }>;
}

export interface InflightMerge {
  /** The current HEAD commit at the time branchMerge was invoked. */
  oursCommit: string;
  /** The branch tip we're merging in. */
  theirsCommit: string;
  /** Common ancestor commit. */
  baseCommit: string;

  /** Raw output from pen-core's mergeDocuments. */
  mergeResult: MergeResult;

  /** O(1) lookup of conflict by id. Built once at branchMerge time. */
  conflictMap: Map<string, NodeConflict | DocFieldConflict>;

  /** Accumulated user choices. Empty until resolveConflict is called. */
  resolutions: Map<string, ConflictResolution>;

  /** Default commit message for applyMerge. The renderer can override later. */
  defaultMessage: string;
}

// ---------------------------------------------------------------------------
// Conflict id codec
//
// Encoding rules (matches spec line 836-841 verbatim):
//   Node conflict: `node:${pageId ?? '_'}:${nodeId}`
//   Doc-field conflict: `field:${field}:${path}`
//
// Stable, deterministic, both engine and renderer agree.
// ---------------------------------------------------------------------------

export function encodeNodeConflictId(conflict: NodeConflict): string {
  return `node:${conflict.pageId ?? '_'}:${conflict.nodeId}`;
}

export function encodeDocFieldConflictId(conflict: DocFieldConflict): string {
  return `field:${conflict.field}:${conflict.path}`;
}

export type ParsedConflictId =
  | { kind: 'node'; pageId: string | null; nodeId: string }
  | { kind: 'field'; field: string; path: string };

/**
 * Parse a conflict id back into its components. Used by resolveConflict to
 * locate the conflict in session state. Throws if the id is malformed —
 * callers should treat that as a programming error (the renderer always
 * passes back ids the engine just emitted).
 */
export function parseConflictId(id: string): ParsedConflictId {
  if (id.startsWith('node:')) {
    const rest = id.slice('node:'.length);
    const colonIdx = rest.indexOf(':');
    if (colonIdx === -1) {
      throw new Error(`Malformed node conflict id: ${id}`);
    }
    const rawPage = rest.slice(0, colonIdx);
    const nodeId = rest.slice(colonIdx + 1);
    return {
      kind: 'node',
      pageId: rawPage === '_' ? null : rawPage,
      nodeId,
    };
  }
  if (id.startsWith('field:')) {
    const rest = id.slice('field:'.length);
    const colonIdx = rest.indexOf(':');
    if (colonIdx === -1) {
      throw new Error(`Malformed field conflict id: ${id}`);
    }
    return {
      kind: 'field',
      field: rest.slice(0, colonIdx),
      path: rest.slice(colonIdx + 1),
    };
  }
  throw new Error(`Unknown conflict id prefix: ${id}`);
}

/**
 * Build a wire-format ConflictBag from a MergeResult by attaching ids. Used
 * by branchMerge before stashing the InflightMerge in session state.
 *
 * Returns BOTH the bag AND the conflict map (id → conflict) so the caller
 * can hydrate the InflightMerge in one pass without re-walking the result.
 */
export function buildConflictBag(result: MergeResult): {
  bag: ConflictBag;
  conflictMap: Map<string, NodeConflict | DocFieldConflict>;
} {
  const conflictMap = new Map<string, NodeConflict | DocFieldConflict>();
  const nodeConflicts = result.nodeConflicts.map((c) => {
    const id = encodeNodeConflictId(c);
    conflictMap.set(id, c);
    return { ...c, id };
  });
  const docFieldConflicts = result.docFieldConflicts.map((c) => {
    const id = encodeDocFieldConflictId(c);
    conflictMap.set(id, c);
    return { ...c, id };
  });
  return { bag: { nodeConflicts, docFieldConflicts }, conflictMap };
}
