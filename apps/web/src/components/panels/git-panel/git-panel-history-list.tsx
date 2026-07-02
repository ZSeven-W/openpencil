// apps/web/src/components/panels/git-panel/git-panel-history-list.tsx
//
// Scrollable history timeline. Walks gitStore.log (flat array of commits,
// newest first), groups consecutive autosaves of 3+ into collapsible
// <HistoryAutosaveGroup> blocks, and renders milestone rows as
// <HistoryMilestoneRow>. Clicking a milestone opens an inline detail
// card below the row with restore + copy-hash + "diff coming in Phase 6".
// Clicking an autosave inside a group opens a detail card with
// "restore" and "promote to milestone".
//
// Read-only mode (`readOnly={true}`) hides the restore and promote
// buttons across all rows. Used by GitPanelConflict: mutating actions
// during an in-flight merge are not legal, but the history list still
// provides context about what's in the repo. Copy-hash stays available
// because it is not a mutation.

import { useMemo, useState } from 'react';
import { ChevronDown, ChevronRight } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { useGitStore } from '@/stores/git-store';
import type { GitCommitMeta } from '@/services/git-types';
import { parseAutosaveMessage } from './format-commit-message';
import { GitPanelHistoryDiff } from './git-panel-history-diff';

const AUTOSAVE_GROUP_THRESHOLD = 3;

type TimelineEntry =
  | { kind: 'milestone'; commit: GitCommitMeta }
  | { kind: 'autosave-row'; commit: GitCommitMeta }
  | { kind: 'autosave-group'; commits: GitCommitMeta[] };

/**
 * Group the flat log into timeline entries. Runs of 3+ consecutive
 * autosaves become a single group entry; smaller runs are individual
 * autosave-row entries.
 */
function groupLog(log: GitCommitMeta[]): TimelineEntry[] {
  const result: TimelineEntry[] = [];
  let autosaveBuffer: GitCommitMeta[] = [];

  const flushBuffer = () => {
    if (autosaveBuffer.length >= AUTOSAVE_GROUP_THRESHOLD) {
      result.push({ kind: 'autosave-group', commits: autosaveBuffer });
    } else {
      for (const c of autosaveBuffer) {
        result.push({ kind: 'autosave-row', commit: c });
      }
    }
    autosaveBuffer = [];
  };

  for (const commit of log) {
    if (commit.kind === 'autosave') {
      autosaveBuffer.push(commit);
    } else {
      flushBuffer();
      result.push({ kind: 'milestone', commit });
    }
  }
  flushBuffer();

  return result;
}

export function GitPanelHistoryList({ readOnly = false }: { readOnly?: boolean } = {}) {
  const { t } = useTranslation();
  const log = useGitStore((s) => s.log);

  const entries = useMemo(() => groupLog(log), [log]);
  // Note: git.history.loadMore is defined for a future paginated-load UX
  //       (Phase 4d+); Phase 4c loads a fixed 50-item window.

  if (log.length === 0) {
    return (
      <div className="flex items-center justify-center p-6 text-xs text-muted-foreground">
        {t('git.history.empty')}
      </div>
    );
  }

  return (
    <div className="relative flex flex-col py-1.5">
      {/* Timeline rail — a 1px line running down the icon-slot center. */}
      <div
        className="pointer-events-none absolute left-5 top-0 h-full w-px bg-border/60"
        aria-hidden
      />
      {entries.map((entry, idx) => {
        if (entry.kind === 'milestone') {
          return (
            <HistoryMilestoneRow
              key={entry.commit.hash}
              commit={entry.commit}
              readOnly={readOnly}
            />
          );
        }
        if (entry.kind === 'autosave-row') {
          return (
            <HistoryAutosaveRow key={entry.commit.hash} commit={entry.commit} readOnly={readOnly} />
          );
        }
        // autosave-group
        return (
          <HistoryAutosaveGroup
            key={`group-${idx}-${entry.commits[0]?.hash ?? idx}`}
            commits={entry.commits}
            readOnly={readOnly}
          />
        );
      })}
    </div>
  );
}

function HistoryMilestoneRow({ commit, readOnly }: { commit: GitCommitMeta; readOnly: boolean }) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);
  const restoreCommit = useGitStore((s) => s.restoreCommit);

  const timeAgo = formatCompactTime(commit.author.timestamp);
  const authorShort = commit.author.name.split(/\s+/)[0] ?? commit.author.name;

  return (
    <div className="relative">
      <button
        type="button"
        onClick={() => setExpanded((v) => !v)}
        className="relative flex w-full items-center gap-3 px-3 py-[7px] text-left transition-colors hover:bg-accent/40"
      >
        <span className="relative z-10 flex w-4 shrink-0 items-center justify-center">
          <span
            className="block h-[7px] w-[7px] rounded-full bg-foreground shadow-[0_0_0_3px_hsl(var(--background))]"
            aria-hidden
          />
        </span>
        <span className="flex-1 truncate text-[12px] font-medium text-foreground">
          {commit.message}
        </span>
        <span className="shrink-0 font-mono text-[10px] tabular-nums text-muted-foreground/80">
          {authorShort} · {timeAgo}
        </span>
      </button>
      {expanded && (
        <div className="relative bg-muted/30">
          <div className="flex flex-col gap-2 pl-10 pr-4 py-3">
            <div className="text-[11px] font-medium text-foreground">
              {t('git.history.milestoneDetailTitle')}
            </div>
            {/* Phase 7b: inline diff block (replaces diffComingSoon placeholder) */}
            <GitPanelHistoryDiff commit={commit} />
            <div className="flex items-center gap-1.5 pt-1">
              {!readOnly && (
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => void restoreCommit(commit.hash)}
                  className="h-6 rounded-md px-2.5 text-[11px]"
                >
                  {t('git.history.restoreButton')}
                </Button>
              )}
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => {
                  if (typeof navigator !== 'undefined' && navigator.clipboard) {
                    void navigator.clipboard.writeText(commit.hash);
                    // git.history.copiedToast exists for a future "Copied!" feedback toast (Phase 4d).
                  }
                }}
                className="h-6 rounded-md px-2.5 text-[11px]"
              >
                {t('git.history.copyHashButton')}
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function HistoryAutosaveRow({ commit, readOnly }: { commit: GitCommitMeta; readOnly: boolean }) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);
  const restoreCommit = useGitStore((s) => s.restoreCommit);
  const promoteAutosave = useGitStore((s) => s.promoteAutosave);
  const authorIdentity = useGitStore((s) => s.authorIdentity);
  const parsed = parseAutosaveMessage(commit.message);
  const timeLabel = parsed?.time ?? '—';

  return (
    <div className="relative">
      <button
        type="button"
        onClick={() => setExpanded((v) => !v)}
        className="relative flex w-full items-center gap-3 px-3 py-[5px] text-left transition-colors hover:bg-accent/40"
      >
        <span className="relative z-10 flex w-4 shrink-0 items-center justify-center">
          <span
            className="block h-[5px] w-[5px] rounded-full border border-muted-foreground/80 bg-background shadow-[0_0_0_3px_hsl(var(--background))]"
            aria-hidden
          />
        </span>
        <span className="flex-1 truncate text-[11px] text-muted-foreground">
          {t('git.history.autosaveLabel', { time: timeLabel })}
        </span>
      </button>
      {expanded && (
        <div className="bg-muted/30">
          <div className="flex flex-col gap-2 pl-10 pr-4 py-2.5">
            {/* Phase 7b: inline diff block for autosave rows */}
            <GitPanelHistoryDiff commit={commit} />
            {!readOnly && (
              <div className="flex items-center gap-1.5 pt-1">
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => void restoreCommit(commit.hash)}
                  className="h-6 rounded-md px-2.5 text-[11px]"
                >
                  {t('git.history.restoreButton')}
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="h-6 rounded-md px-2.5 text-[11px]"
                  onClick={() => {
                    // git.history.promoteSuccessToast exists for a future success toast (Phase 4d).
                    // Phase 4c: promote reuses the autosave's own message as the
                    // milestone message (parsed autosave messages are "auto: HH:MM",
                    // which is ugly but wired). A dedicated message prompt is
                    // deferred until the inline author/message form is generalized
                    // (Phase 4d+). Author falls back to the sentinel used by the
                    // autosave subscriber so promote never blocks on missing
                    // identity; the user can clean up the name afterward.
                    const author = authorIdentity ?? {
                      name: 'Unknown',
                      email: 'unknown@local',
                    };
                    void promoteAutosave(commit.hash, commit.message, author);
                  }}
                >
                  {t('git.history.promoteButton')}
                </Button>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

function HistoryAutosaveGroup({
  commits,
  readOnly,
}: {
  commits: GitCommitMeta[];
  readOnly: boolean;
}) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);

  return (
    <div className="relative">
      <button
        type="button"
        onClick={() => setExpanded((v) => !v)}
        className="relative flex w-full items-center gap-3 px-3 py-[5px] text-left transition-colors hover:bg-accent/40"
      >
        <span className="relative z-10 flex w-4 shrink-0 items-center justify-center bg-background">
          {expanded ? (
            <ChevronDown size={11} className="text-muted-foreground" aria-hidden />
          ) : (
            <ChevronRight size={11} className="text-muted-foreground" aria-hidden />
          )}
        </span>
        <span className="text-[11px] text-muted-foreground">
          {t(
            commits.length === 1
              ? 'git.history.autosaveGroup_one'
              : 'git.history.autosaveGroup_other',
            { count: commits.length },
          )}
        </span>
      </button>
      {expanded && (
        <div className="flex flex-col">
          {commits.map((c) => (
            <HistoryAutosaveRow key={c.hash} commit={c} readOnly={readOnly} />
          ))}
        </div>
      )}
    </div>
  );
}

/**
 * Format an author timestamp as a compact relative time. Accepts a Unix
 * timestamp in seconds (git's native format) or milliseconds. Returns
 * strings like "3m", "1h", "yesterday", or an ISO date.
 */
function formatCompactTime(ts: number): string {
  const tsMs = ts < 1e12 ? ts * 1000 : ts;
  const diffMin = Math.floor((Date.now() - tsMs) / 60000);
  if (diffMin < 1) return 'now';
  if (diffMin < 60) return `${diffMin}m`;
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return `${diffHr}h`;
  const diffDay = Math.floor(diffHr / 24);
  if (diffDay === 1) return 'yesterday';
  if (diffDay < 7) return `${diffDay}d`;
  return new Date(tsMs).toLocaleDateString();
}
