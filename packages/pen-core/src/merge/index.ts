// packages/pen-core/src/merge/index.ts
//
// Public 合并模块的表面。

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
