// apps/web/src/components/panels/git-panel/git-panel-error-card.tsx
//
// Generic error display for state.kind === 'error'. Shows the error message
// from the GitError, an optional retry button (when recoverable), and a
// dismiss button (calls closeRepo to reset to no-file).
//
// Phase 4a ships this for the init/open/clone error paths. Phase 4c will
// reuse it for autosave errors via the panel header indicator.

import { AlertCircle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { useGitStore } from '@/stores/git-store';

interface GitPanelErrorCardProps {
  message: string;
  recoverable: boolean;
  /** Optional retry handler. If omitted, the retry button calls closePanel. */
  onRetry?: () => void;
}

export function GitPanelErrorCard({ message, recoverable, onRetry }: GitPanelErrorCardProps) {
  const { t } = useTranslation();
  const closeRepo = useGitStore((s) => s.closeRepo);

  return (
    <div className="flex flex-col items-center justify-center gap-3 p-6 text-center">
      <AlertCircle size={28} className="text-destructive" aria-hidden />
      <div className="text-sm font-medium text-foreground">{t('git.error.title')}</div>
      <div className="text-xs text-muted-foreground max-w-[280px] break-words">{message}</div>
      <div className="flex items-center gap-2 pt-1">
        {recoverable && (
          <Button
            type="button"
            variant="default"
            size="sm"
            onClick={() => {
              if (onRetry) {
                onRetry();
              } else {
                void closeRepo();
              }
            }}
          >
            {t('git.error.retry')}
          </Button>
        )}
        <Button type="button" variant="ghost" size="sm" onClick={() => void closeRepo()}>
          {t('git.error.dismiss')}
        </Button>
      </div>
    </div>
  );
}
