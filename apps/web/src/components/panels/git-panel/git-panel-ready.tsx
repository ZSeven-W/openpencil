// apps/web/src/components/panels/git-panel/git-panel-ready.tsx
//
// Ready-state body composer. Orchestrates the four sub-components that
// make up the ready state: header, save-required alert (conditional),
// commit input, and history list. Also triggers loadLog whenever we
// enter the ready state so the history list has something to show.

import { useGitPanelLogLoader } from './use-git-panel-log-loader';
import { GitPanelHeader } from './git-panel-header';
import { GitPanelSaveRequiredAlert } from './git-panel-save-required-alert';
import { GitPanelCommitInput } from './git-panel-commit-input';
import { GitPanelHistoryList } from './git-panel-history-list';

// Phase 7b: stable constant outside the component so the array identity
// does not change on every render, avoiding spurious loadLog re-fires.
const READY_KINDS = ['ready'] as const;

export function GitPanelReady() {
  // Phase 7b: load log for the current branch (not hardcoded 'main').
  useGitPanelLogLoader(READY_KINDS);

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
