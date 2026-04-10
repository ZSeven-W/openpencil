// apps/web/src/components/panels/git-panel/git-panel-header.tsx
//
// Header row for the Git panel ready/conflict states (Phase 4c).
// Renders a flex row with two groups:
//   Left:  branch trigger (disabled) + pull (disabled) + push (disabled)
//   Right: autosave-error dot + author-missing dot + overflow popover menu
//
// The component returns null unless state.kind is 'ready' or 'conflict'.
// TooltipProvider is mounted globally in editor-layout.tsx — no local wrapper needed.

import { ArrowDown, ArrowUp, GitBranch, MoreHorizontal } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { Separator } from '@/components/ui/separator';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { useGitStore } from '@/stores/git-store';

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

  const { currentBranch } = state.repo;

  return (
    <div className="flex items-center justify-between gap-1 border-b border-border px-2 py-1">
      {/* ── Left group: branch + pull + push ── */}
      <div className="flex items-center gap-0.5">
        {/* Branch trigger — disabled, coming in Phase 5 */}
        <Tooltip>
          <TooltipTrigger asChild>
            <span tabIndex={0} className="inline-flex">
              <Button
                type="button"
                variant="ghost"
                size="sm"
                disabled
                aria-label={currentBranch}
                className="pointer-events-none flex max-w-[120px] items-center gap-1 text-muted-foreground"
              >
                <GitBranch size={12} strokeWidth={1.5} aria-hidden />
                <span className="truncate text-xs">{currentBranch}</span>
              </Button>
            </span>
          </TooltipTrigger>
          <TooltipContent side="bottom">{t('git.header.branchComingSoon')}</TooltipContent>
        </Tooltip>

        {/* Pull — disabled, coming in Phase 6 */}
        <Tooltip>
          <TooltipTrigger asChild>
            <span tabIndex={0} className="inline-flex">
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                disabled
                aria-label={t('git.header.pullComingSoon')}
                className="pointer-events-none text-muted-foreground"
              >
                <ArrowDown size={12} strokeWidth={1.5} aria-hidden />
              </Button>
            </span>
          </TooltipTrigger>
          <TooltipContent side="bottom">{t('git.header.pullComingSoon')}</TooltipContent>
        </Tooltip>

        {/* Push — disabled, coming in Phase 6 */}
        <Tooltip>
          <TooltipTrigger asChild>
            <span tabIndex={0} className="inline-flex">
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                disabled
                aria-label={t('git.header.pushComingSoon')}
                className="pointer-events-none text-muted-foreground"
              >
                <ArrowUp size={12} strokeWidth={1.5} aria-hidden />
              </Button>
            </span>
          </TooltipTrigger>
          <TooltipContent side="bottom">{t('git.header.pushComingSoon')}</TooltipContent>
        </Tooltip>
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
