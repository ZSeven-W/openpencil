// packages/pen-core/src/merge/merge-helpers.ts
//
// Pure 由 node-diff.ts 和 node-merge.ts 共享的索引和行走实用程序。
// Not 从模块的公共表面导出 - 这些是内部助手。

import type { PenDocument, PenNode } from '@zseven-w/pen-types';

/**
 * Information
 * 按索引节点存储，捕获节点本身和合并算法所需的结构上下文（页面、父级、索引）。
 */
export interface IndexedNode {
  /** 对于旧版单页文档为 null（无页面数组） */
  pageId: string | null;
  /** 当节点位于页面顶部（或旧子节点顶部）时为 null */
  parentId: string | null;
  /** 在 parent.children（或顶级数组）中的位置 */
  index: number;
  node: PenNode;
}

/**
 * Walk 文档并生成
 * Map<nodeId, IndexedNode>。 Handles `pages` 和旧版 `children` 形状一致。
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
 * Normalize
 * 将文档放入页面列表中，无论它是使用显式 `pages` 数组还是旧版 `children` 数组。 Legacy 模式使用 `id =
 * null` 生成单个合成页面。
 */
export function getAllPages(doc: PenDocument): Array<{ id: string | null; children: PenNode[] }> {
  if (doc.pages && doc.pages.length > 0) {
    return doc.pages.map((p) => ({ id: p.id, children: p.children }));
  }
  return [{ id: null, children: doc.children ?? [] }];
}

/**
 * Compare
 * 仅由原子字段组成的两个节点（除了 `children` 之外的所有节点）。 Returns 如果它们会产生相同的仅字段差异，则为 true。
 */
export function nodeFieldsEqual(a: PenNode, b: PenNode): boolean {
  const aFields = stripChildren(a);
  const bFields = stripChildren(b);
  return jsonEqual(aFields, bFields);
}

/**
 * Return `node
 * ` 的浅表副本，删除了 `children` 字段。 Used 在我们想要比较或区分节点的原子字段而没有递归子噪声的任何地方。
 *
 */
export function stripChildren<T extends PenNode>(node: T): Omit<T, 'children'> {
  // Use 解构； TS-安全。
  const copy = { ...node } as T & { children?: unknown };
  delete copy.children;
  return copy as Omit<T, 'children'>;
}

/**
 * Deep 通过 JSON
 * 规范化实现值相等。 Used 通过 nodeFieldsEqual 和通过文档字段合并来比较变量值、主题条目等。
 *
 * Important：这是故意简单的。 PenNode 字段值是 JSON 安全的（数字、字符串、布尔值、普通对象、数组）。
 * No、Dates、Map
 * s、Sets、函数等位于 PenDocument。
 */
export function jsonEqual(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (a === null || b === null) return false;
  if (typeof a !== typeof b) return false;
  if (typeof a !== 'object') return false;
  // Both 是非空对象（或数组）
  return JSON.stringify(canonicalize(a)) === JSON.stringify(canonicalize(b));
}

/**
 * Sort 对象键递归，因
 * 此 JSON.stringify 生成确定性字符串，无论插入顺序如何。
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
