// apps/web/src/components/panels/git-panel/git-panel-conflict-item.tsx
//
// Thin dispatcher component that renders either a GitPanelNodeConflictCard or
// a GitPanelFieldConflictCard based on the conflict kind. The conflict list
// renders these uniformly without needing to switch on kind itself.

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
