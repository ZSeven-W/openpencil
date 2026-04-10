// apps/web/src/components/panels/git-panel/git-panel-ready.tsx
//
// Ready-state body composer. Orchestrates the four sub-components that
// make up the ready state: header, save-required alert (conditional),
// commit input, and history list. Also triggers loadLog whenever we
// enter the ready state so the history list has something to show.

import { useEffect } from 'react';
import { useGitStore } from '@/stores/git-store';
import { GitPanelHeader } from './git-panel-header';
import { GitPanelSaveRequiredAlert } from './git-panel-save-required-alert';
import { GitPanelCommitInput } from './git-panel-commit-input';
import { GitPanelHistoryList } from './git-panel-history-list';

export function GitPanelReady() {
  const state = useGitStore((s) => s.state);
  const loadLog = useGitStore((s) => s.loadLog);

  // Load the log whenever we enter the ready state. The history list reads
  // useGitStore.log reactively, so it updates automatically when this call
  // completes. Re-fires if the panel transitions from a non-ready kind
  // (e.g. error → ready) without unmounting.
  useEffect(() => {
    if (state.kind !== 'ready') return;
    void loadLog({ ref: 'main', limit: 50 });
  }, [state.kind, loadLog]);

  return (
    <div className="flex h-full flex-col">
      <GitPanelHeader />
      <GitPanelSaveRequiredAlert />
      <GitPanelCommitInput />
      <div className="flex-1 overflow-y-auto">
        <GitPanelHistoryList />
      </div>
    </div>
  );
}
