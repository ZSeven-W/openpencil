// apps/web/src/components/panels/git-panel/git-panel-history-diff.tsx
//
// Phase 7b: inline diff block shown under an expanded history row. Replaces
// the "Diff view coming in Phase 6" placeholder. Loads the diff summary
// and patch list on expand, then renders:
//   - loading state while the IPC is in flight
//   - error state if computeDiff() throws
//   - initial-commit state when commit.parentHashes[0] is absent
//   - diff summary (framesChanged, nodesAdded, nodesRemoved, nodesModified)
//   - compact patch list (op + nodeId)
//
// This is intentionally an inline block, not a full-screen route.

import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useGitStore } from '@/stores/git-store';
import type { GitCommitMeta } from '@/services/git-types';

interface DiffSummary {
  framesChanged: number;
  nodesAdded: number;
  nodesRemoved: number;
  nodesModified: number;
}

interface DiffPatch {
  op: string;
  nodeId?: string;
  [key: string]: unknown;
}

interface DiffState {
  status: 'loading' | 'ready' | 'error' | 'initial-commit';
  summary?: DiffSummary;
  patches?: DiffPatch[];
  errorMessage?: string;
}

interface GitPanelHistoryDiffProps {
  commit: GitCommitMeta;
}

export function GitPanelHistoryDiff({ commit }: GitPanelHistoryDiffProps) {
  const { t } = useTranslation();
  const computeDiff = useGitStore((s) => s.computeDiff);
  const [diff, setDiff] = useState<DiffState>({ status: 'loading' });

  const parentHash = commit.parentHashes[0] ?? null;

  useEffect(() => {
    if (parentHash === null) {
      setDiff({ status: 'initial-commit' });
      return;
    }
    setDiff({ status: 'loading' });
    let cancelled = false;
    void (async () => {
      try {
        const result = await computeDiff(parentHash, commit.hash);
        if (cancelled) return;
        setDiff({
          status: 'ready',
          summary: result.summary as DiffSummary,
          patches: result.patches as DiffPatch[],
        });
      } catch (err) {
        if (cancelled) return;
        setDiff({
          status: 'error',
          errorMessage: err instanceof Error ? err.message : String(err),
        });
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [parentHash, commit.hash, computeDiff]);

  if (diff.status === 'loading') {
    return (
      <div className="text-[10px] text-muted-foreground italic">
        {t('git.history.diff.loading')}
      </div>
    );
  }

  if (diff.status === 'initial-commit') {
    return (
      <div className="text-[10px] text-muted-foreground italic">
        {t('git.history.diff.initialCommit')}
      </div>
    );
  }

  if (diff.status === 'error') {
    return (
      <div className="text-[10px] text-destructive">
        {t('git.history.diff.error', { message: diff.errorMessage ?? '' })}
      </div>
    );
  }

  // status === 'ready'
  const s = diff.summary!;
  const patches = diff.patches ?? [];

  const hasSummary =
    s.framesChanged > 0 || s.nodesAdded > 0 || s.nodesRemoved > 0 || s.nodesModified > 0;

  return (
    <div className="flex flex-col gap-1.5">
      {/* Summary row */}
      {hasSummary ? (
        <div className="flex flex-wrap gap-x-3 gap-y-0.5">
          {s.framesChanged > 0 && (
            <span className="text-[10px] text-muted-foreground">
              {t('git.history.diff.framesChanged', { count: s.framesChanged })}
            </span>
          )}
          {s.nodesAdded > 0 && (
            <span className="text-[10px] text-primary">
              +{t('git.history.diff.nodesAdded', { count: s.nodesAdded })}
            </span>
          )}
          {s.nodesRemoved > 0 && (
            <span className="text-[10px] text-destructive">
              -{t('git.history.diff.nodesRemoved', { count: s.nodesRemoved })}
            </span>
          )}
          {s.nodesModified > 0 && (
            <span className="text-[10px] text-muted-foreground">
              ~{t('git.history.diff.nodesModified', { count: s.nodesModified })}
            </span>
          )}
        </div>
      ) : (
        <div className="text-[10px] text-muted-foreground italic">
          {t('git.history.diff.noChanges')}
        </div>
      )}

      {/* Compact patch list */}
      {patches.length > 0 && (
        <ul className="flex flex-col gap-0.5 max-h-24 overflow-y-auto">
          {patches.map((p, i) => (
            <li
              key={p.nodeId != null ? `${p.nodeId}-${i}` : i}
              className="text-[10px] text-muted-foreground font-mono truncate"
            >
              <span className="text-foreground">{p.op}</span>
              {p.nodeId != null && <span className="ml-1 opacity-60">{String(p.nodeId)}</span>}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
