// apps/web/src/components/panels/git-panel/git-panel-conflict-banner.tsx
//
// Red destructive banner shown when state.kind === 'conflict'. Phase 5
// ships abort-only — manual per-node / per-field resolution arrives in
// Phase 7. Until then the banner's only affordance is [Abort merge],
// which clears the in-flight merge and returns the store to 'ready'.
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
  const abortMerge = useGitStore((s) => s.abortMerge);

  return (
    <div
      role="alert"
      className="flex flex-col gap-2 border-b border-destructive/30 bg-destructive/10 px-4 py-3 text-destructive"
    >
      <div className="flex items-start gap-2">
        <AlertTriangle size={14} className="mt-0.5 shrink-0" aria-hidden />
        <div className="flex flex-col gap-1">
          <p className="text-xs font-medium">{t('git.conflict.title')}</p>
          <p className="text-xs opacity-80">{t('git.conflict.description')}</p>
        </div>
      </div>
      <div className="flex justify-end">
        <Button type="button" variant="destructive" size="sm" onClick={() => void abortMerge()}>
          {t('git.conflict.abort')}
        </Button>
      </div>
    </div>
  );
}
