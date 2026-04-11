// packages/pen-core/src/merge/index.ts
//
// Public surface for the merge module.

export type { NodePatch } from './node-diff.js';
export { diffDocuments } from './node-diff.js';

export type {
  MergeInput,
  MergeResult,
  NodeConflict,
  NodeConflictReason,
  DocFieldConflict,
  DocFieldName,
} from './node-merge.js';
export { mergeDocuments } from './node-merge.js';
