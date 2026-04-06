// packages/pen-core/src/merge/node-merge.ts
//
// Pure 3-way merge of PenDocument trees.

import type { PenDocument, PenNode } from '@zseven-w/pen-types';

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
  /** null when reason === 'add-vs-add-different' */
  base: PenNode | null;
  /** null when ours deleted the node */
  ours: PenNode | null;
  /** null when theirs deleted the node */
  theirs: PenNode | null;
}

export type DocFieldName = 'variables' | 'themes' | 'pages-order' | 'name' | 'version';

export interface DocFieldConflict {
  field: DocFieldName;
  /** dotted path for nested structures, e.g. 'variables.color-1.value' */
  path: string;
  base: unknown;
  ours: unknown;
  theirs: unknown;
}

export interface MergeResult {
  /** best-effort merged document; for unresolved conflicts, ours' value is used as a placeholder */
  merged: PenDocument;
  nodeConflicts: NodeConflict[];
  docFieldConflicts: DocFieldConflict[];
}

/**
 * Pure 3-way merge of PenDocument trees.
 *
 * Implementation in Tasks 7-11.
 */
export function mergeDocuments(_input: MergeInput): MergeResult {
  throw new Error('not implemented');
}
