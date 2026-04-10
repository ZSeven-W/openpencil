// apps/web/src/components/panels/git-panel/git-panel-commit-input.tsx
//
// Commit input: textarea + "保存为里程碑" button + lazy author form
// trigger. Reads commitMessage from the store (so it persists across
// panel re-mounts within a session). On submit:
//   1. If commitMessage is empty, do nothing (button is also disabled).
//   2. If authorIdentity is null, show the inline author form and mark
//      the commit as pending-after-auth. The useEffect below re-fires
//      the submit once the form succeeds.
//   3. Call commitMilestone; the store's commitMilestone action handles
//      post-success log refresh + clearCommitMessage. If the save gate
//      trips, the store sets saveRequiredFor which the parent renders
//      via <GitPanelSaveRequiredAlert>.

import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { useGitStore } from '@/stores/git-store';
import { GitPanelAuthorForm } from './git-panel-author-form';

export function GitPanelCommitInput() {
  const { t } = useTranslation();
  const commitMessage = useGitStore((s) => s.commitMessage);
  const setCommitMessage = useGitStore((s) => s.setCommitMessage);
  const authorIdentity = useGitStore((s) => s.authorIdentity);
  const authorPromptVisible = useGitStore((s) => s.authorPromptVisible);
  const showAuthorPrompt = useGitStore((s) => s.showAuthorPrompt);
  const commitMilestone = useGitStore((s) => s.commitMilestone);

  const [pendingCommitAfterAuth, setPendingCommitAfterAuth] = useState(false);

  const handleSubmit = async () => {
    const trimmed = commitMessage.trim();
    if (!trimmed) return;

    // Lazy author form trigger — show once, remember we wanted to commit.
    if (authorIdentity === null) {
      setPendingCommitAfterAuth(true);
      showAuthorPrompt();
      return;
    }

    try {
      await commitMilestone(trimmed, authorIdentity);
      // commitMilestone clears commitMessage + refreshes log on success.
      // If the save gate tripped, the store set saveRequiredFor which the
      // parent renders via the save-required alert.
    } catch {
      // Swallow — the store has transitioned to error state OR set
      // saveRequiredFor. No extra work needed here.
    }
  };

  // Re-fire the commit after the author form succeeds.
  useEffect(() => {
    if (pendingCommitAfterAuth && authorIdentity !== null && !authorPromptVisible) {
      setPendingCommitAfterAuth(false);
      void handleSubmit();
    }
    // handleSubmit is intentionally omitted from deps. Because the textarea
    // is replaced by <GitPanelAuthorForm> while authorPromptVisible is true,
    // commitMessage cannot change during the auth flow. The handleSubmit
    // closure captured when pendingCommitAfterAuth was set therefore still
    // references the correct, up-to-date commitMessage, and re-invoking it
    // here is safe. Including handleSubmit in deps would re-run the effect
    // on every keystroke, which is not what we want.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pendingCommitAfterAuth, authorIdentity, authorPromptVisible]);

  return (
    <div className="border-b border-border">
      {authorPromptVisible ? (
        <GitPanelAuthorForm />
      ) : (
        <div className="flex flex-col gap-2 p-3">
          <textarea
            value={commitMessage}
            onChange={(e) => setCommitMessage(e.target.value)}
            placeholder={t('git.commit.placeholder')}
            rows={2}
            className="w-full resize-none rounded-md border border-input bg-secondary px-2 py-1.5 text-xs text-foreground placeholder:text-muted-foreground focus:border-primary focus:outline-none"
          />
          <div className="flex items-center justify-end">
            <Button
              type="button"
              variant="default"
              size="sm"
              disabled={commitMessage.trim().length === 0}
              onClick={() => void handleSubmit()}
            >
              {t('git.commit.submitButton')}
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
