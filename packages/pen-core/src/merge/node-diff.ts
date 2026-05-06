// packages/pen-core/src/merge/node-diff.ts
//
// One-direction diff：计算将 `base` 转换为 `next` 所需的补丁。

import type { PenNode, PenDocument } from '@zseven-w/pen-types';
import { indexNodesById, nodeFieldsEqual, stripChildren, jsonEqual } from './merge-helpers.js';

export interface NodePatch {
  op: 'add' | 'remove' | 'modify' | 'move';
  /** 对于旧版单页文档为 null（无页面数组） */
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
 * Compute 将
 * `base` 转换为 `next` 所需的补丁。 Walks 两棵树均按节点 id 排列，每次更改都会发出 `add` / `remove` /
 *
 * `modify` / `move` 之一。 Algorithm: 1. Index 两个文档均按节点 ID。 2. For 基∪下一个中的每个 id：
 * - 仅在下一个中 →
 * `add` - 仅在基中
 * → `remove` - 在两者中 → 检查 parent/index
 * （如果已更改 → `move`）和原子字段（如果有更改 →
 * `modify`）；单个 id 可能会产生
 * `move` 和 `modify`。
 *
 */
export function diffDocuments(base: PenDocument, next: PenDocument): NodePatch[] {
  const baseIdx = indexNodesById(base);
  const nextIdx = indexNodesById(next);
  const allIds = new Set<string>([...baseIdx.keys(), ...nextIdx.keys()]);
  const patches: NodePatch[] = [];

  for (const id of allIds) {
    const b = baseIdx.get(id);
    const n = nextIdx.get(id);

    if (!b && n) {
      // Added。
      patches.push({
        op: 'add',
        pageId: n.pageId,
        nodeId: id,
        parentId: n.parentId,
        index: n.index,
        fields: stripChildren(n.node) as Partial<PenNode>,
      });
      continue;
    }

    if (b && !n) {
      // Removed。
      patches.push({
        op: 'remove',
        pageId: b.pageId,
        nodeId: id,
      });
      continue;
    }

    if (b && n) {
      // Present 两者都有。 Check 代表 `move`（父级或页面已更改）和 `modify`（原子字段已更改）。 They
      // 是独立的——一个节点可以产生两种补丁。
      const moved = b.parentId !== n.parentId || b.pageId !== n.pageId || b.index !== n.index;
      if (moved) {
        patches.push({
          op: 'move',
          pageId: n.pageId,
          nodeId: id,
          parentId: n.parentId,
          index: n.index,
        });
      }
      if (!nodeFieldsEqual(b.node, n.node)) {
        const { changed, before } = diffFields(b.node, n.node);
        patches.push({
          op: 'modify',
          pageId: n.pageId,
          nodeId: id,
          fields: changed,
          beforeFields: before,
        });
      }
    }
  }

  return patches;
}

/**
 * Compute
 * 两个节点之间的每个字段增量。 Returns 更改的键（在 `next` 中）以及原始值（从 `base` 中）。 Skips `children`
 * 字段 — 它由递归遍历单独处理。
 */
function diffFields(
  base: PenNode,
  next: PenNode,
): { changed: Partial<PenNode>; before: Partial<PenNode> } {
  const baseStripped = stripChildren(base) as Record<string, unknown>;
  const nextStripped = stripChildren(next) as Record<string, unknown>;
  const allKeys = new Set<string>([...Object.keys(baseStripped), ...Object.keys(nextStripped)]);
  const changed: Record<string, unknown> = {};
  const before: Record<string, unknown> = {};
  for (const key of allKeys) {
    if (!jsonEqual(baseStripped[key], nextStripped[key])) {
      changed[key] = nextStripped[key];
      before[key] = baseStripped[key];
    }
  }
  return {
    changed: changed as Partial<PenNode>,
    before: before as Partial<PenNode>,
  };
}
