// apps/desktop/git/merge-session.ts
//
// In-单个存储库的飞行合并状态。 Stored 上 RepoSession.inflightMerge
// 在冲突解决循环期间。 The 冲突 ID 编解码器也存在于此
// 因为它是引擎和测试共享的唯一字符串格式。

import type { MergeResult, NodeConflict, DocFieldConflict } from '@zseven-w/pen-core';
import type { PenNode } from '@zseven-w/pen-types';

/**
 * The 用户解决单个冲突
 * 的选择。 Mirrors 规范的 ConflictResolution 联合。
 */
export type ConflictResolution =
  | { kind: 'ours' }
  | { kind: 'theirs' }
  | { kind: 'manual-node'; node: PenNode } // for node conflicts
  | { kind: 'manual-field'; value: unknown }; // for doc-field conflicts

/**
 * Wire-通过 IPC
 * 返回格式冲突包。 The 渲染器将其包装在 Maps 中以进行分辨率跟踪。 Each 冲突有一个稳定的 id，渲染器通过
 * resolveConflict() 传回。
 */
export interface ConflictBag {
  nodeConflicts: Array<NodeConflict & { id: string }>;
  docFieldConflicts: Array<DocFieldConflict & { id: string }>;
}

export interface InflightMerge {
  /** The 调用 branchMerge 时的当前 HEAD 提交。 */
  oursCommit: string;
  /** 我们正在合并的 The 分支提示。 */
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
// Node conflict: `node:${pageId ?? '_'}:${nodeId}`
// Doc-field conflict: `field:${field}:${path}`
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
