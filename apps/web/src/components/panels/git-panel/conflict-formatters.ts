// apps/web/src/components/panels/git-panel/conflict-formatters.ts
//
// Pure 帮助程序，用于格式化冲突解决 UI 标签。 No React，无商店
// 依赖项——可以安全地导入到任何地方并且易于单元测试。

import type { PenDocument, PenNode } from '@/types/pen';
import type { GitConflictBag, GitConflictResolution } from '@/services/git-types';

// ---------------------------------------------------------------------------
// Document-顺序冲突排序
// ---------------------------------------------------------------------------

/**
 * 具有可选解决状态的节点冲
 * 突条目（与 ConflictBagState 的 nodeConflicts 值形状匹配 - 在此处键入以避免从带有 Zustand 存储包的
 * git-store-types 导入）。
 */
export type NodeConflictEntry = GitConflictBag['nodeConflicts'][number] & {
  resolution?: GitConflictResolution;
};

export type FieldConflictEntry = GitConflictBag['docFieldConflicts'][number] & {
  resolution?: GitConflictResolution;
};

/**
 * Walk 一个
 * PenNode 深度优先树，为每个节点调用 `visit`。 Visits `node.children`（如果存在）（并非所有
 PenNode 变体都有子代）。
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
 * Produce 冲突列表
 *
 * UI 的冲突条目的有序平面列表。 Ordering 规则： 1. Node 文档树顺序冲突（深度优先）。 The 冲突 Map 由
 * `node:<pageI
 * d|_>:<nodeId>`
 * 键入，因此我们通过拆分“：”并获取最后一段来导出 nodeId。 2. Doc 字段冲突是文档级别的，没有树位置。 They
 * 在树内节点冲突后发出，按 `path` 字段的字母顺序排序，以获得稳定的、用户可读的序列。 3. Orphan 节点冲突 —
 * 当前文档树中不存在 nodeId 的冲突（例如，它们删除了该节点） — 最后发出，保留
 * Map 插入顺序以保持稳定性。
 *
 *
 *
 */
export function orderConflicts(
  document: PenDocument,
  nodeConflicts: Map<string, NodeConflictEntry>,
  fieldConflicts: Map<string, FieldConflictEntry>,
): Array<NodeConflictEntry | FieldConflictEntry> {
  const result: Array<NodeConflictEntry | FieldConflictEntry> = [];

  // Build 一组按文档树位置排序的节点冲突条目。
  const emitted = new Set<string>();

  // Walk 每个页面单独，这样我们就可以通过 pageId 来确定冲突匹配范围。
  // For 单页文档（无 doc.pages），合成一个虚拟页面
  // pageId === null 因此键 `node:_:<nodeId>` 仍然匹配。
//
  // Node 冲突键架构：`node:<pageId|_>:<nodeId>`
  // pageId 存储在 entry.pageId（单页文档为空）
  const pages: Array<{ pageId: string | null; children: PenNode[] }> =
    document?.pages && document.pages.length > 0
      ? document.pages.map((p) => ({ pageId: p.id, children: p.children }))
      : [{ pageId: null, children: document?.children ?? [] }];

  for (const { pageId, children } of pages) {
    walkTreeDfs(children, (node) => {
      // Each 节点冲突的键是 `node:<pageId|_>:<nodeId>`。 We 需要找到 nodeId AND pageId
      // 都匹配该节点的条目。
      for (const [key, entry] of nodeConflicts) {
        if (emitted.has(key)) continue;
        if (entry.nodeId === node.id && entry.pageId === pageId) {
          result.push(entry);
          emitted.add(key);
          break; // At 每对 (pageId, nodeId) 最多有一个冲突。
        }
      }
    });
  }

  // Orphan 节点冲突：在当前树中找不到引用的 nodeId。
  for (const [key, entry] of nodeConflicts) {
    if (!emitted.has(key)) {
      result.push(entry);
      emitted.add(key);
    }
  }

  // Doc 字段冲突：按路径按字母顺序排列。
  const fieldEntries = Array.from(fieldConflicts.values());
  fieldEntries.sort((a, b) => a.path.localeCompare(b.path));
  for (const entry of fieldEntries) {
    result.push(entry);
  }

  return result;
}

// ---------------------------------------------------------------------------
// Reason 标签映射
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
// JSON 漂亮的打印
// ---------------------------------------------------------------------------

/**
 * Pretty -
 * 将值打印为缩进的 JSON。 Returns 失败时的占位符字符串（例如循环引用、空值）。
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
// Validation 帮助者
// ---------------------------------------------------------------------------

/**
 * Parse a JSON
 * string and return `{ ok: true, value }` or `{ ok: false, error }`. Used by the
 manual JSON editor to give instant parse-error feedback.
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
 * Validate
 * 表明解析的 JSON 值是一个类似 PenNode 的对象，具有预期的 `nodeId`。 Returns a validation error
 *
 * string or null when valid. We 进行最小的结构检查 - 后端对 applyMerge
 * 执行完整的模式验证，因此
 * 这里我们只需要足够的信息即可在 IPC 往返之前在 UI 中提供有用的反馈。
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
// Truncation 帮助者
// ---------------------------------------------------------------------------

/** Truncate a string for display, adding an ellipsis when it exceeds maxLen. */
export function truncate(s: string, maxLen: number): string {
  if (s.length <= maxLen) return s;
  return s.slice(0, maxLen - 1) + '\u2026';
}
