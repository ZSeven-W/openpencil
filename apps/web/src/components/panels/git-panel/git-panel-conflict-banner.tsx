// apps/web/src/components/panels/git-panel/git-panel-conflict-banner.tsx
//
// Phase 7b: upgraded conflict banner. Replaces the Phase 5 abort-only shell
// and the Phase 6b non-op strip with a unified status header that:
//   - shows title + resolved/total progress count
//   - lists non-.op unresolved files when present
//   - dynamic primary button:
//       * 应用合并 ("Apply merge") when unresolved .op conflicts remain
//       * 继续 ("Continue") when .op conflicts are all resolved and only
//         terminal-resolved non-.op files are still pending
//   - always-visible 中止 merge ("Abort merge") button
//   - inline finalizeError from applyMerge() throwing merge-still-conflicted
//
// The banner sits inside <GitPanelConflict /> beneath the panel header.

import { AlertTriangle, AlertCircle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { useGitStore } from '@/stores/git-store';

export function GitPanelConflictBanner() {
  const { t } = useTranslation();
  const state = useGitStore((s) => s.state);
  const abortMerge = useGitStore((s) => s.abortMerge);
  const applyMerge = useGitStore((s) => s.applyMerge);

  // The banner is only mounted by GitPanelConflict, so state.kind is
  // always 'conflict' here. The narrows are for TypeScript.
  const unresolvedFiles = state.kind === 'conflict' ? state.unresolvedFiles : [];
  const hasNonOpConflict = unresolvedFiles.length > 0;
  const finalizeError = state.kind === 'conflict' ? state.finalizeError : null;
  // I2: panel reopened mid-merge — in-memory conflict state was lost.
  const reopenedMidMerge = state.kind === 'conflict' ? state.reopenedMidMerge : false;

  // Count resolved vs total .op conflicts for the progress display.
  let resolvedCount = 0;
  let totalCount = 0;
  if (state.kind === 'conflict') {
    const { nodeConflicts, docFieldConflicts } = state.conflicts;
    for (const c of nodeConflicts.values()) {
      totalCount++;
      if (c.resolution != null) resolvedCount++;
    }
    for (const c of docFieldConflicts.values()) {
      totalCount++;
      if (c.resolution != null) resolvedCount++;
    }
  }

  // Determine primary action label. Rule:
  //   - unresolved .op conflicts remain → "Apply merge" (or "Apply" when
  //     there are also non-.op files, to signal we're past the .op phase)
  //   - all .op resolved (or zero .op conflicts) + non-.op files pending → "Continue"
  const opUnresolved = totalCount - resolvedCount;
  const useApplyLabel = opUnresolved > 0 || (totalCount === 0 && !hasNonOpConflict);
  const primaryLabel = useApplyLabel
    ? t('git.conflict.banner.apply')
    : t('git.conflict.banner.continue');

  const showProgress = totalCount > 0;
  // I2: in the panel-reopen degraded mode, hide the primary button entirely —
  // only the abort button is actionable.
  const showPrimaryButton = !reopenedMidMerge && (useApplyLabel || hasNonOpConflict);

  return (
    <div
      role="alert"
      className="flex flex-col gap-2 border-b border-destructive/30 bg-destructive/10 px-4 py-3 text-destructive"
    >
      {/* Title + progress */}
      <div className="flex items-start gap-2">
        <AlertTriangle size={14} className="mt-0.5 shrink-0" aria-hidden />
        <div className="flex flex-col gap-0.5 flex-1">
          <div className="flex items-center justify-between gap-2">
            <p className="text-xs font-medium">
              {hasNonOpConflict ? t('git.conflict.nonOp.title') : t('git.conflict.title')}
            </p>
            {showProgress && (
              <span className="text-[10px] tabular-nums shrink-0">
                {t('git.conflict.banner.progress', {
                  resolved: resolvedCount,
                  total: totalCount,
                })}
              </span>
            )}
          </div>
          <p className="text-xs opacity-80">
            {hasNonOpConflict ? t('git.conflict.nonOp.description') : t('git.conflict.description')}
          </p>
        </div>
      </div>

      {/* I2: panel-reopen warning — shown instead of normal primary action */}
      {reopenedMidMerge && (
        <div className="flex items-start gap-1.5 rounded border border-destructive/30 bg-background/40 px-2 py-1.5">
          <AlertCircle size={11} className="mt-0.5 shrink-0" aria-hidden />
          <p className="text-[11px]">{t('git.conflict.banner.reopenMessage')}</p>
        </div>
      )}

      {/* Non-.op unresolved file list */}
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

      {/* Inline finalize error from merge-still-conflicted */}
      {finalizeError != null && (
        <div className="flex items-start gap-1.5 rounded border border-destructive/30 bg-background/40 px-2 py-1.5">
          <AlertCircle size={11} className="mt-0.5 shrink-0" aria-hidden />
          <p className="text-[11px]">
            {t('git.conflict.banner.finalizeError', { message: finalizeError })}
          </p>
        </div>
      )}

      {/* Action buttons */}
      <div className="flex justify-end gap-2">
        <Button type="button" variant="outline" size="sm" onClick={() => void abortMerge()}>
          {t('git.conflict.abort')}
        </Button>
        {showPrimaryButton && (
          <Button type="button" variant="default" size="sm" onClick={() => void applyMerge()}>
            {primaryLabel}
          </Button>
        )}
      </div>
    </div>
  );
}
