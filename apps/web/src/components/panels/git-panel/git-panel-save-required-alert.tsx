// apps/web/src/components/panels/git-panel/git-panel-save-required-alert.tsx
//
// Inline alert shown above the commit input when a mutating action
// tripped the withCleanWorkingTree gate. Body quotes the pending action's
// label (e.g. "commit milestone", "restore", "pull"). Two buttons:
//   [保存] → retrySaveRequired (saves the document then re-runs the queued action)
//   [取消] → cancelSaveRequired (clears the flag without retrying)

import { AlertCircle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { useGitStore } from '@/stores/git-store';

export function GitPanelSaveRequiredAlert() {
  const { t } = useTranslation();
  const state = useGitStore((s) => s.state);
  const retrySaveRequired = useGitStore((s) => s.retrySaveRequired);
  const cancelSaveRequired = useGitStore((s) => s.cancelSaveRequired);

  // Only render when we're in ready/conflict with a pending action.
  if (state.kind !== 'ready' && state.kind !== 'conflict') return null;
  if (!state.saveRequiredFor) return null;

  const label = state.saveRequiredFor.label;

  return (
    <div className="border-b border-border/60 bg-destructive/10 px-4 py-3 flex flex-col gap-2">
      <div className="flex items-start gap-2">
        <AlertCircle size={14} className="text-destructive mt-0.5 shrink-0" aria-hidden />
        <div className="flex flex-col gap-0.5">
          <div className="text-xs font-medium text-foreground">
            {t('git.commit.saveRequiredTitle')}
          </div>
          <div className="text-[11px] text-muted-foreground">
            {t('git.commit.saveRequiredBody', { label })}
          </div>
        </div>
      </div>
      <div className="flex items-center justify-end gap-2">
        <Button type="button" variant="ghost" size="sm" onClick={cancelSaveRequired}>
          {t('git.commit.saveRequiredCancel')}
        </Button>
        <Button type="button" variant="default" size="sm" onClick={() => void retrySaveRequired()}>
          {t('git.commit.saveRequiredSave')}
        </Button>
      </div>
    </div>
  );
}
