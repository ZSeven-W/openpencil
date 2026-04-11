// apps/web/src/components/panels/git-panel/git-panel-header.tsx
//
// Header row for the Git panel ready/conflict states (Phase 4c → 6c).
// Renders a flex row with two groups:
//   Left:  branch picker (Phase 5) + pull/push remote controls (Phase 6b)
//   Right: autosave-error dot + author-missing dot + overflow popover menu
//
// Phase 6c expands the overflow popover into a LOCAL state machine mirroring
// the Phase 5 branch picker pattern:
//   { view: 'menu' | 'remote-settings' | 'ssh-keys' }
// The menu view shows the existing three entries plus two new entries that
// swap the popover content into the subviews defined in git-panel-remote-
// settings.tsx / git-panel-ssh-keys.tsx. Subview state is NOT persisted to
// the store — it lives entirely in this file and resets on popover close.
//
// The component returns null unless state.kind is 'ready' or 'conflict'.

import {
  ChevronRight,
  FileSearch,
  Key,
  LogOut,
  MoreHorizontal,
  Settings2,
  UserX,
} from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { Separator } from '@/components/ui/separator';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { useGitStore } from '@/stores/git-store';
import { GitPanelBranchPicker } from './git-panel-branch-picker';
import { GitPanelRemoteControls } from './git-panel-remote-controls';
import { GitPanelRemoteSettings } from './git-panel-remote-settings';
import { GitPanelSshKeys } from './git-panel-ssh-keys';

type OverflowView = 'menu' | 'remote-settings' | 'ssh-keys';

export function GitPanelHeader() {
  const { t } = useTranslation();
  const [overflowOpen, setOverflowOpen] = useState(false);
  const [overflowView, setOverflowView] = useState<OverflowView>('menu');

  const state = useGitStore((s) => s.state);
  const autosaveError = useGitStore((s) => s.autosaveError);
  const clearAutosaveError = useGitStore((s) => s.clearAutosaveError);
  const enterTrackedFilePicker = useGitStore((s) => s.enterTrackedFilePicker);
  const clearAuthorIdentity = useGitStore((s) => s.clearAuthorIdentity);
  const closeRepo = useGitStore((s) => s.closeRepo);
  const authorIdentity = useGitStore((s) => s.authorIdentity);

  if (state.kind !== 'ready' && state.kind !== 'conflict') return null;

  const popoverWidth =
    overflowView === 'menu' ? 'w-56' : overflowView === 'remote-settings' ? 'w-[300px]' : 'w-80';

  return (
    <div className="flex items-center justify-between gap-1 border-b border-border/60 bg-card/40 px-2.5 py-1.5 backdrop-blur-sm">
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
        <Popover
          open={overflowOpen}
          onOpenChange={(next) => {
            setOverflowOpen(next);
            if (next) {
              // Always open on the menu view — a previous session's subview
              // should never leak back in when the user reopens the popover.
              setOverflowView('menu');
            }
          }}
        >
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
          <PopoverContent
            align="end"
            className={`${popoverWidth} rounded-lg border-border/70 p-1 shadow-lg`}
            role="menu"
          >
            {overflowView === 'menu' && (
              <>
                <button
                  type="button"
                  role="menuitem"
                  onClick={() => {
                    setOverflowOpen(false);
                    enterTrackedFilePicker();
                  }}
                  className="flex w-full items-center gap-2.5 rounded-md px-2 py-1.5 text-[12px] text-foreground transition-colors hover:bg-accent/60"
                >
                  <FileSearch
                    size={13}
                    strokeWidth={1.75}
                    className="text-muted-foreground"
                    aria-hidden
                  />
                  {t('git.header.overflowSwitchTracked')}
                </button>
                <button
                  type="button"
                  role="menuitem"
                  onClick={() => {
                    setOverflowOpen(false);
                    void clearAuthorIdentity();
                  }}
                  className="flex w-full items-center gap-2.5 rounded-md px-2 py-1.5 text-[12px] text-foreground transition-colors hover:bg-accent/60"
                >
                  <UserX
                    size={13}
                    strokeWidth={1.75}
                    className="text-muted-foreground"
                    aria-hidden
                  />
                  {t('git.header.overflowClearAuthor')}
                </button>
                <Separator className="my-1 bg-border/50" />
                <button
                  type="button"
                  role="menuitem"
                  data-testid="overflow-open-remote-settings"
                  onClick={() => setOverflowView('remote-settings')}
                  className="flex w-full items-center gap-2.5 rounded-md px-2 py-1.5 text-[12px] text-foreground transition-colors hover:bg-accent/60"
                >
                  <Settings2
                    size={13}
                    strokeWidth={1.75}
                    className="text-muted-foreground"
                    aria-hidden
                  />
                  <span className="flex-1 text-left">{t('git.header.overflowRemoteSettings')}</span>
                  <ChevronRight
                    size={12}
                    strokeWidth={1.5}
                    className="text-muted-foreground/70"
                    aria-hidden
                  />
                </button>
                <button
                  type="button"
                  role="menuitem"
                  data-testid="overflow-open-ssh-keys"
                  onClick={() => setOverflowView('ssh-keys')}
                  className="flex w-full items-center gap-2.5 rounded-md px-2 py-1.5 text-[12px] text-foreground transition-colors hover:bg-accent/60"
                >
                  <Key size={13} strokeWidth={1.75} className="text-muted-foreground" aria-hidden />
                  <span className="flex-1 text-left">{t('git.header.overflowSshKeys')}</span>
                  <ChevronRight
                    size={12}
                    strokeWidth={1.5}
                    className="text-muted-foreground/70"
                    aria-hidden
                  />
                </button>
                <Separator className="my-1 bg-border/50" />
                <button
                  type="button"
                  role="menuitem"
                  onClick={() => {
                    setOverflowOpen(false);
                    void closeRepo();
                  }}
                  className="flex w-full items-center gap-2.5 rounded-md px-2 py-1.5 text-[12px] text-foreground transition-colors hover:bg-accent/60"
                >
                  <LogOut
                    size={13}
                    strokeWidth={1.75}
                    className="text-muted-foreground"
                    aria-hidden
                  />
                  {t('git.header.overflowCloseRepo')}
                </button>
              </>
            )}
            {overflowView === 'remote-settings' && (
              <GitPanelRemoteSettings onBack={() => setOverflowView('menu')} />
            )}
            {overflowView === 'ssh-keys' && (
              <GitPanelSshKeys onBack={() => setOverflowView('menu')} />
            )}
          </PopoverContent>
        </Popover>
      </div>
    </div>
  );
}
