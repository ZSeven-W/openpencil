// apps/web/src/components/panels/git-panel/git-panel.tsx
//
// Git panel body — renders inside a <PopoverContent> anchored to the
// top-bar GitButton. Phase 4a.1 removed the floating/draggable/minimized
// chrome in favor of a dropdown form. The popover itself owns visibility
// (open/close via the Popover's open + onOpenChange props wired in
// git-button.tsx), so GitPanel no longer checks panelOpen or
// panelMinimized — by the time this component renders, the popover is
// already open.
//
// The body switches on `state.kind` and delegates to the sibling
// components (empty-state, error card, etc.). Author identity is loaded
// once on first mount. Repo detection is triggered when the tracked
// file path changes while the panel is visible.

import { useEffect } from 'react';
import { Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useGitStore } from '@/stores/git-store';
import { useDocumentStore } from '@/stores/document-store';
import { GitPanelCloneForm } from './git-panel-clone-form';
import { GitPanelConflict } from './git-panel-conflict';
import { GitPanelEmptyState } from './git-panel-empty-state';
import { GitPanelErrorCard } from './git-panel-error-card';
import { GitPanelReady } from './git-panel-ready';
import { GitPanelTrackedPicker } from './git-panel-tracked-picker';
import { Button } from '@/components/ui/button';

/**
 * Body of the Git popover. Assumes the Popover ancestor is already open
 * (Radix unmounts PopoverContent when closed, so this component only
 * renders when visible).
 */
export function GitPanel() {
  const { t } = useTranslation();
  const state = useGitStore((s) => s.state);
  const loadAuthorIdentity = useGitStore((s) => s.loadAuthorIdentity);
  const detectRepo = useGitStore((s) => s.detectRepo);
  const lastAutoBindedPath = useGitStore((s) => s.lastAutoBindedPath);
  const acknowledgeAutoBind = useGitStore((s) => s.acknowledgeAutoBind);
  const acknowledgeAutoBindAndOpen = useGitStore((s) => s.acknowledgeAutoBindAndOpen);

  // Reactive document store reads — re-renders when the user saves or
  // opens a different file while the dropdown is open.
  const docFilePath = useDocumentStore((s) => s.filePath);

  // Load author identity once on first mount. loadAuthorIdentity itself
  // is idempotent and short-circuits on SSR via a typeof window guard.
  useEffect(() => {
    void loadAuthorIdentity();
  }, [loadAuthorIdentity]);

  // Trigger repo detection whenever the current file path changes while
  // the panel is open in no-file state. Covers the "user opened the
  // dropdown before saving, then saved" flow.
  useEffect(() => {
    if (state.kind !== 'no-file') return;
    if (docFilePath) {
      void detectRepo(docFilePath);
    }
  }, [state.kind, detectRepo, docFilePath]);

  return (
    <div className="flex-1 overflow-y-auto">
      {lastAutoBindedPath && (
        <AutoBindBanner
          path={lastAutoBindedPath}
          onOpen={() => void acknowledgeAutoBindAndOpen()}
          onDismiss={() => acknowledgeAutoBind()}
        />
      )}
      {(state.kind === 'no-file' || state.kind === 'no-repo') && <GitPanelEmptyState />}
      {state.kind === 'wizard-clone' && <GitPanelCloneForm />}
      {state.kind === 'initializing' && (
        <div className="flex flex-col items-center justify-center gap-2 p-6 text-muted-foreground">
          <Loader2 size={20} className="animate-spin" aria-hidden />
          <span className="text-xs">{t('git.initializing')}</span>
        </div>
      )}
      {state.kind === 'needs-tracked-file' && <GitPanelTrackedPicker />}
      {state.kind === 'ready' && <GitPanelReady />}
      {state.kind === 'conflict' && <GitPanelConflict />}
      {state.kind === 'error' && (
        <GitPanelErrorCard message={state.message} recoverable={state.recoverable} />
      )}
    </div>
  );
}

function AutoBindBanner({
  path,
  onOpen,
  onDismiss,
}: {
  path: string;
  onOpen: () => void;
  onDismiss: () => void;
}) {
  const { t } = useTranslation();
  const fileName = path.split(/[/\\]/).pop() || path;
  return (
    <div className="border-b border-border bg-muted/40 px-4 py-3 flex flex-col gap-2">
      <div className="text-xs text-foreground">
        {t('git.autoBind.confirmHeading', { fileName })}
      </div>
      <div className="flex items-center justify-end gap-2">
        <Button type="button" variant="ghost" size="sm" onClick={onDismiss}>
          {t('git.autoBind.dismissButton')}
        </Button>
        <Button type="button" variant="default" size="sm" onClick={onOpen}>
          {t('git.autoBind.openButton')}
        </Button>
      </div>
    </div>
  );
}
