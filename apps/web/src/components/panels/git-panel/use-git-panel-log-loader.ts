// apps/web/src/components/panels/git-panel/use-git-panel-log-loader.ts
//
// Phase 7b: shared hook that loads the commit log for the current branch
// whenever state.kind matches one of the active-repo kinds. Replaces the
// hardcoded `ref: 'main'` that GitPanelReady and GitPanelConflict used to
// pass — the log should always follow state.repo.currentBranch.
//
// Callers just invoke the hook; it fires loadLog on mount and whenever
// state.kind or state.repo.currentBranch changes.

import { useEffect } from 'react';
import { useGitStore } from '@/stores/git-store';

/**
 * Fires `loadLog({ ref: state.repo.currentBranch, limit: 50 })` whenever
 * the panel is in one of the given `kinds`. Re-fires when state.kind or
 * currentBranch changes so branch switches and conflict → ready transitions
 * always show the right log.
 */
export function useGitPanelLogLoader(kinds: ReadonlyArray<string>): void {
  const state = useGitStore((s) => s.state);
  const loadLog = useGitStore((s) => s.loadLog);

  const currentBranch =
    state.kind === 'ready' || state.kind === 'conflict' || state.kind === 'needs-tracked-file'
      ? state.repo.currentBranch
      : null;

  useEffect(() => {
    if (!kinds.includes(state.kind)) return;
    if (currentBranch === null) return;
    void loadLog({ ref: currentBranch, limit: 50 });
  }, [state.kind, currentBranch, loadLog, kinds]);
}
