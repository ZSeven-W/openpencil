// apps/web/src/components/panels/git-panel/git-panel-conflict-banner.tsx
//
// Destructive banner shown when state.kind === 'conflict'. Phase 5
// shipped abort-only — Phase 7 will land manual per-node / per-field
// resolution. Phase 6b extends this banner with a dedicated recovery
// strip for non-`.op` merge conflicts (e.g. README.md changed on both
// sides during a pull): the user can either mark those files resolved
// externally and hit [Continue], or abort the merge entirely.
//
// The banner sits inside <GitPanelConflict /> beneath the panel header.
// It does NOT render the history list — that's composed alongside it
// by GitPanelConflict.

import { AlertTriangle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { useGitStore } from '@/stores/git-store';

export function GitPanelConflictBanner() {
  const { t } = useTranslation();
  const state = useGitStore((s) => s.state);
  const abortMerge = useGitStore((s) => s.abortMerge);
  const applyMerge = useGitStore((s) => s.applyMerge);

  // The banner is only mounted by GitPanelConflict, so state.kind is
  // always 'conflict' here. The narrow is for TypeScript only.
  const unresolvedFiles = state.kind === 'conflict' ? state.unresolvedFiles : [];
  const hasNonOpConflict = unresolvedFiles.length > 0;

  return (
    <div
      role="alert"
      className="flex flex-col gap-2 border-b border-destructive/30 bg-destructive/10 px-4 py-3 text-destructive"
    >
      <div className="flex items-start gap-2">
        <AlertTriangle size={14} className="mt-0.5 shrink-0" aria-hidden />
        <div className="flex flex-col gap-1">
          <p className="text-xs font-medium">
            {hasNonOpConflict ? t('git.conflict.nonOp.title') : t('git.conflict.title')}
          </p>
          <p className="text-xs opacity-80">
            {hasNonOpConflict ? t('git.conflict.nonOp.description') : t('git.conflict.description')}
          </p>
        </div>
      </div>

      {hasNonOpConflict && (
        <div className="flex flex-col gap-1 rounded border border-destructive/30 bg-background/40 px-2 py-1.5">
          <div className="text-[11px] font-medium text-destructive">
            {t('git.conflict.nonOp.unresolvedHeading', { count: unresolvedFiles.length })}
          </div>
          <ul className="flex flex-col gap-0.5">
            {unresolvedFiles.map((path) => (
              <li key={path} className="text-[11px] text-foreground font-mono">
                {path}
              </li>
            ))}
          </ul>
        </div>
      )}

      <div className="flex justify-end gap-2">
        <Button type="button" variant="outline" size="sm" onClick={() => void abortMerge()}>
          {hasNonOpConflict ? t('git.conflict.nonOp.abort') : t('git.conflict.abort')}
        </Button>
        {hasNonOpConflict && (
          <Button type="button" variant="default" size="sm" onClick={() => void applyMerge()}>
            {t('git.conflict.nonOp.continue')}
          </Button>
        )}
      </div>
    </div>
  );
}
