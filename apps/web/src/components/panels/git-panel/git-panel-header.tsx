// apps/web/src/components/panels/git-panel/git-panel-header.tsx
//
// Header row for the Git panel ready/conflict states (Phase 4c → 6b).
// Renders a flex row with two groups:
//   Left:  branch picker (Phase 5) + pull/push remote controls (Phase 6b)
//   Right: autosave-error dot + author-missing dot + overflow popover menu
//
// The component returns null unless state.kind is 'ready' or 'conflict'.
// Pull and push are delegated to <GitPanelRemoteControls /> so this file
// stays thin — the remote-action state machine, auth retry flow, and
// push-rejected recovery strip all live inside that component.

import { MoreHorizontal } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { Separator } from '@/components/ui/separator';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { useGitStore } from '@/stores/git-store';
import { GitPanelBranchPicker } from './git-panel-branch-picker';
import { GitPanelRemoteControls } from './git-panel-remote-controls';

export function GitPanelHeader() {
  const { t } = useTranslation();
  const [overflowOpen, setOverflowOpen] = useState(false);

  const state = useGitStore((s) => s.state);
  const autosaveError = useGitStore((s) => s.autosaveError);
  const clearAutosaveError = useGitStore((s) => s.clearAutosaveError);
  const enterTrackedFilePicker = useGitStore((s) => s.enterTrackedFilePicker);
  const clearAuthorIdentity = useGitStore((s) => s.clearAuthorIdentity);
  const closeRepo = useGitStore((s) => s.closeRepo);
  const authorIdentity = useGitStore((s) => s.authorIdentity);

  if (state.kind !== 'ready' && state.kind !== 'conflict') return null;

  return (
    <div className="flex items-center justify-between gap-1 border-b border-border px-2 py-1">
      {/* ── Left group: branch + remote controls ── */}
      <div className="flex items-center gap-0.5">
        <GitPanelBranchPicker />
        <GitPanelRemoteControls />
      </div>

      {/* ── Right group: status dots + overflow menu ── */}
      <div className="flex items-center gap-1">
        {/* Autosave error dot — rendered only when an error exists */}
        {autosaveError !== null && (
          <Tooltip>
            <TooltipTrigger asChild>
              <button
                type="button"
                onClick={() => clearAutosaveError()}
                className="flex h-6 w-6 items-center justify-center rounded-full"
                aria-label={t('git.header.autosaveError')}
              >
                <span className="block h-2 w-2 rounded-full bg-destructive" aria-hidden />
              </button>
            </TooltipTrigger>
            <TooltipContent side="bottom" className="max-w-[200px]">
              <p className="font-medium">{t('git.header.autosaveErrorTitle')}</p>
              <p className="text-xs opacity-80">{autosaveError}</p>
            </TooltipContent>
          </Tooltip>
        )}

        {/* Author-missing dot — rendered only when no author identity set */}
        {authorIdentity === null && (
          <Tooltip>
            <TooltipTrigger asChild>
              {/* Not clickable — tooltip-only hint */}
              <span
                className="flex h-5 w-5 cursor-default items-center justify-center rounded-full"
                role="status"
                aria-label={t('git.header.authorMissingWarning')}
              >
                {/* bg-yellow-500 is intentional — no shadcn token for "warning" */}
                <span className="block h-2 w-2 rounded-full bg-yellow-500" aria-hidden />
              </span>
            </TooltipTrigger>
            <TooltipContent side="bottom" className="max-w-[200px]">
              {t('git.header.authorMissingWarning')}
            </TooltipContent>
          </Tooltip>
        )}

        {/* Overflow menu */}
        <Popover open={overflowOpen} onOpenChange={setOverflowOpen}>
          <PopoverTrigger asChild>
            <Button
              type="button"
              variant="ghost"
              size="icon-sm"
              aria-label={t('git.header.overflowMoreActions')}
              className="text-muted-foreground"
            >
              <MoreHorizontal size={13} strokeWidth={1.5} aria-hidden />
            </Button>
          </PopoverTrigger>
          <PopoverContent align="end" className="w-56 p-1" role="menu">
            <button
              type="button"
              role="menuitem"
              onClick={() => {
                setOverflowOpen(false);
                enterTrackedFilePicker();
              }}
              className="flex w-full items-center rounded-sm px-2 py-1.5 text-xs text-foreground hover:bg-accent"
            >
              {t('git.header.overflowSwitchTracked')}
            </button>
            <button
              type="button"
              role="menuitem"
              onClick={() => {
                setOverflowOpen(false);
                void clearAuthorIdentity();
              }}
              className="flex w-full items-center rounded-sm px-2 py-1.5 text-xs text-foreground hover:bg-accent"
            >
              {t('git.header.overflowClearAuthor')}
            </button>
            <Separator className="my-1" />
            <button
              type="button"
              role="menuitem"
              onClick={() => {
                setOverflowOpen(false);
                void closeRepo();
              }}
              className="flex w-full items-center rounded-sm px-2 py-1.5 text-xs text-foreground hover:bg-accent"
            >
              {t('git.header.overflowCloseRepo')}
            </button>
          </PopoverContent>
        </Popover>
      </div>
    </div>
  );
}
