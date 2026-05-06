// apps/web/src/components/panels/git-panel/git-panel-conflict-item.tsx
//
// Thin 调度程序组件，呈现 GitPanelNodeConflictCard 或
// 基于冲突类型的 GitPanelFieldConflictCard。 The 冲突列表
// 统一渲染这些，而不需要打开种类本身。

import { useGitStore } from '@/stores/git-store';
import { GitPanelNodeConflictCard } from './git-panel-node-conflict-card';
import { GitPanelFieldConflictCard } from './git-panel-field-conflict-card';
import type { GitConflictResolution } from '@/services/git-types';

export type ConflictItemKind = 'node' | 'field';

export interface NodeConflictItemData {
  kind: 'node';
  id: string;
  pageId: string | null;
  nodeId: string;
  reason:
    | 'both-modified-same-field'
    | 'modify-vs-delete'
    | 'add-vs-add-different'
    | 'reparent-conflict';
  base: unknown;
  ours: unknown;
  theirs: unknown;
  resolution?: GitConflictResolution;
}

export interface FieldConflictItemData {
  kind: 'field';
  id: string;
  field: string;
  path: string;
  base: unknown;
  ours: unknown;
  theirs: unknown;
  resolution?: GitConflictResolution;
}

export type ConflictItemData = NodeConflictItemData | FieldConflictItemData;

export interface GitPanelConflictItemProps {
  item: ConflictItemData;
}

export function GitPanelConflictItem({ item }: GitPanelConflictItemProps) {
  const resolveConflict = useGitStore((s) => s.resolveConflict);

  function handleResolve(choice: GitConflictResolution) {
    void resolveConflict(item.id, choice);
  }

  if (item.kind === 'node') {
    return <GitPanelNodeConflictCard conflict={item} onResolve={handleResolve} />;
  }

  return <GitPanelFieldConflictCard conflict={item} onResolve={handleResolve} />;
}
