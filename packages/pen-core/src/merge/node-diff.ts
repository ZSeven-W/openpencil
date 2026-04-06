// packages/pen-core/src/merge/node-diff.ts
//
// One-direction diff: compute the patches needed to transform `base` into `next`.

import type { PenNode, PenDocument } from '@zseven-w/pen-types';

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
 * Implementation in Task 5.
 */
export function diffDocuments(_base: PenDocument, _next: PenDocument): NodePatch[] {
  throw new Error('not implemented');
}
