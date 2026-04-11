// apps/web/src/components/panels/git-panel/git-panel-conflict.tsx
//
// Body shown when state.kind === 'conflict'. Mirrors how GitPanelReady
// composes the ready state: the panel header sits on top (with branch
// switching disabled mid-merge), the destructive conflict banner is
// next, and the history list takes the remaining scrollable space as
// read-only context. There is deliberately no commit input — committing
// during a conflict is not a legal action until Phase 7 lands manual
// resolution.
//
// Phase 7b: polling for non-.op unresolved files lives here so it only
// runs while the conflict workspace is visible.

import { useEffect, useRef } from 'react';
import { useGitStore } from '@/stores/git-store';
import { useGitPanelLogLoader } from './use-git-panel-log-loader';
import { GitPanelHeader } from './git-panel-header';
import { GitPanelConflictBanner } from './git-panel-conflict-banner';
import { GitPanelHistoryList } from './git-panel-history-list';

const CONFLICT_KINDS = ['conflict'] as const;
const POLL_INTERVAL_MS = 3000;

export function GitPanelConflict() {
  const state = useGitStore((s) => s.state);
  const refreshStatus = useGitStore((s) => s.refreshStatus);

  // Phase 7b: load log for the current branch (not hardcoded 'main').
  useGitPanelLogLoader(CONFLICT_KINDS);

  // Phase 7b: poll refreshStatus every 3s while there are unresolved
  // non-.op files. This allows the banner to update if the user resolves
  // them in an external editor without remounting the panel.
  //
  // Lifecycle rules:
  //   - poll only when state.kind === 'conflict' AND unresolvedFiles.length > 0
  //   - skip overlapping polls via in-flight ref
  //   - stop polling on first error (surface error once, then stop)
  //   - cleanup on unmount
  const inFlightRef = useRef<boolean>(false);
  const pollErrorRef = useRef<string | null>(null);

  const unresolvedCount = state.kind === 'conflict' ? state.unresolvedFiles.length : 0;
  const shouldPoll = state.kind === 'conflict' && unresolvedCount > 0;

  useEffect(() => {
    if (!shouldPoll) return;
    // Reset the error sentinel on each new poll session (e.g. unresolvedCount
    // went 0 → non-zero after a state refresh).
    pollErrorRef.current = null;

    const id = setInterval(async () => {
      // Skip if a refresh is already in flight.
      if (inFlightRef.current) return;
      // Stop if we already hit an error this session.
      if (pollErrorRef.current !== null) return;

      inFlightRef.current = true;
      try {
        await refreshStatus();
      } catch (err) {
        pollErrorRef.current = err instanceof Error ? err.message : String(err);
      } finally {
        inFlightRef.current = false;
      }
    }, POLL_INTERVAL_MS);

    return () => {
      clearInterval(id);
      inFlightRef.current = false;
    };
  }, [shouldPoll, refreshStatus]);

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
