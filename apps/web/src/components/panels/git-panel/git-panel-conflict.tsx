// apps/web/src/components/panels/git-panel/git-panel-conflict.tsx
//
// Body shown when state.kind === 'conflict'. Mirrors how GitPanelReady
// composes the ready state: the panel header sits on top (with branch
// switching disabled mid-merge), the destructive conflict banner is
// next, and the history list takes the remaining scrollable space as
// read-only context. There is deliberately no commit input — committing
// during a conflict is not a legal action until Phase 7 lands manual
// resolution.

import { useEffect } from 'react';
import { useGitStore } from '@/stores/git-store';
import { GitPanelHeader } from './git-panel-header';
import { GitPanelConflictBanner } from './git-panel-conflict-banner';
import { GitPanelHistoryList } from './git-panel-history-list';

export function GitPanelConflict() {
  const state = useGitStore((s) => s.state);
  const loadLog = useGitStore((s) => s.loadLog);

  // Load the log whenever we enter the conflict state, so the read-only
  // history has context to show. Mirrors the GitPanelReady log effect,
  // keyed on state.kind so it re-fires on transitions into conflict.
  useEffect(() => {
    if (state.kind !== 'conflict') return;
    void loadLog({ ref: 'main', limit: 50 });
  }, [state.kind, loadLog]);

  return (
    <div className="flex h-full flex-col">
      <GitPanelHeader />
      <GitPanelConflictBanner />
      <div className="flex-1 overflow-y-auto">
        <GitPanelHistoryList readOnly />
      </div>
    </div>
  );
}
