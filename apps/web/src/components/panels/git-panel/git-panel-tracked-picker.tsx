// apps/web/src/components/panels/git-panel/git-panel-tracked-picker.tsx
//
// Phase 4b: tracked-file picker. Shown when openRepo or cloneRepo
// returns a folder-mode repo with multiple .op files. The user picks
// which file the Git panel should track. Two action buttons:
//   - 跟踪此文件 (track only): bindTrackedFile, panel transitions to ready
//   - 跟踪并打开 (track and open): bindTrackedFile + load the file into
//     the editor via the loadOpFileFromPath helper
//
// The zero-candidates edge case renders a small empty card prompting
// the user to close the panel (and the underlying repo session).
//
// The exactly-one-candidate path is handled in the store's openRepo /
// cloneRepo actions (auto-bind), so this component never has to show
// a single-row picker.

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { useGitStore } from '@/stores/git-store';
import { loadOpFileFromPath } from '@/utils/load-op-file';
import type { GitCandidateFileInfo } from '@/services/git-types';

export function GitPanelTrackedPicker() {
  const { t } = useTranslation();
  const state = useGitStore((s) => s.state);
  const bindTrackedFile = useGitStore((s) => s.bindTrackedFile);
  const closePanel = useGitStore((s) => s.closePanel);
  const closeRepo = useGitStore((s) => s.closeRepo);
  // Phase 7b: exitTrackedFilePicker drives the back/cancel navigation rule
  // (back → ready when rebinding, cancel → no-file when first open).
  const exitTrackedFilePicker = useGitStore((s) => s.exitTrackedFilePicker);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);

  // Defensive guard — git-panel.tsx's body switch only mounts us in the
  // needs-tracked-file branch, but if a state transition races us we
  // render null instead of crashing.
  if (state.kind !== 'needs-tracked-file') return null;

  const candidates = state.repo.candidateFiles;
  // Phase 7b: determine back/cancel label based on whether a tracked file
  // is already bound. isRebind=true → entered from ready → back label.
  // isRebind=false → first post-open/clone screen → cancel label.
  const isRebind = state.repo.trackedFilePath !== null;
  const backLabel = isRebind ? t('git.picker.back') : t('git.picker.backClose');

  // Edge case: zero candidates
  if (candidates.length === 0) {
    return (
      <div className="flex flex-col items-center gap-3 p-6 text-center">
        <div className="text-sm font-medium text-foreground">{t('git.picker.empty.heading')}</div>
        <div className="text-xs text-muted-foreground max-w-[280px]">
          {t('git.picker.empty.body')}
        </div>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={async () => {
            await closeRepo();
            closePanel();
          }}
        >
          {t('git.picker.empty.close')}
        </Button>
      </div>
    );
  }

  // Sort by lastCommitAt desc, with null last. Tiebreak on relativePath
  // ascending for ANY equal primary key (two nulls OR two equal non-null
  // timestamps) so the sort is total and stable.
  const sorted = [...candidates].sort((a, b) => {
    // Primary key: lastCommitAt desc, nulls last
    if (a.lastCommitAt !== b.lastCommitAt) {
      if (a.lastCommitAt === null) return 1;
      if (b.lastCommitAt === null) return -1;
      return b.lastCommitAt - a.lastCommitAt;
    }
    // Equal primary key (both null OR both the same non-null timestamp):
    // fall back to relativePath asc as the tiebreak.
    return a.relativePath.localeCompare(b.relativePath);
  });

  const handleBindOnly = async () => {
    if (!selectedPath) return;
    await bindTrackedFile(selectedPath);
  };
  const handleBindAndOpen = async () => {
    if (!selectedPath) return;
    await bindTrackedFile(selectedPath);
    const ok = await loadOpFileFromPath(selectedPath);
    void ok;
  };

  return (
    <div className="flex flex-col gap-3 p-4">
      <div className="text-xs text-foreground">
        {t('git.picker.heading', { count: candidates.length })}
      </div>
      <div className="flex flex-col gap-1.5">
        {sorted.map((c) => (
          <TrackedPickerRow
            key={c.path}
            candidate={c}
            selected={selectedPath === c.path}
            onSelect={() => setSelectedPath(c.path)}
            t={t}
          />
        ))}
      </div>
      <div className="flex items-center justify-between gap-2 pt-1">
        {/* Phase 7b: back/cancel affordance — navigates back to ready when
            rebinding, or closes the transient session when first opened. */}
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={() => void exitTrackedFilePicker()}
        >
          {backLabel}
        </Button>
        <div className="flex items-center gap-2">
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={!selectedPath}
            onClick={() => void handleBindOnly()}
          >
            {t('git.picker.bindButton')}
          </Button>
          <Button
            type="button"
            variant="default"
            size="sm"
            disabled={!selectedPath}
            onClick={() => void handleBindAndOpen()}
          >
            {t('git.picker.bindAndOpenButton')}
          </Button>
        </div>
      </div>
    </div>
  );
}

interface TrackedPickerRowProps {
  candidate: GitCandidateFileInfo;
  selected: boolean;
  onSelect: () => void;
  t: (key: string, opts?: Record<string, unknown>) => string;
}

function TrackedPickerRow({ candidate, selected, onSelect, t }: TrackedPickerRowProps) {
  const milestoneLabel =
    candidate.milestoneCount === 0
      ? t('git.picker.noHistory')
      : t('git.picker.milestoneCount', { count: candidate.milestoneCount });

  return (
    <button
      type="button"
      onClick={onSelect}
      className={`flex flex-col items-start gap-0.5 rounded-md border p-2 text-left transition-colors ${
        selected ? 'border-primary bg-primary/5' : 'border-border hover:bg-accent'
      }`}
    >
      <div className="flex w-full items-center justify-between gap-2">
        <span className="text-xs font-medium text-foreground truncate">
          {candidate.relativePath}
        </span>
        <span className="text-[10px] text-muted-foreground shrink-0">{milestoneLabel}</span>
      </div>
      {candidate.lastCommitMessage && (
        <div className="text-[10px] text-muted-foreground truncate w-full">
          {t('git.picker.lastCommit', {
            message: candidate.lastCommitMessage,
            time: formatRelativeTime(candidate.lastCommitAt, t),
          })}
        </div>
      )}
    </button>
  );
}

/**
 * Format a unix timestamp (seconds OR milliseconds) as a localized
 * relative time string. Returns an empty string for null timestamps.
 */
function formatRelativeTime(
  ts: number | null,
  t: (key: string, opts?: Record<string, unknown>) => string,
): string {
  if (ts === null) return '';
  const tsMs = ts < 1e12 ? ts * 1000 : ts;
  const diffMin = Math.floor((Date.now() - tsMs) / 60000);
  if (diffMin < 1) return t('git.relativeTime.justNow');
  if (diffMin < 60) return t('git.relativeTime.minutesAgo', { count: diffMin });
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return t('git.relativeTime.hoursAgo', { count: diffHr });
  const diffDay = Math.floor(diffHr / 24);
  return t('git.relativeTime.daysAgo', { count: diffDay });
}
