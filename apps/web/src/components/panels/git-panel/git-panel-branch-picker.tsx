// apps/web/src/components/panels/git-panel/git-panel-branch-picker.tsx
//
// Phase 5 Task 3: wires branch creation + switching through the picker.
//
// What this file DOES:
//   - Declares the full state machine (mode, branchName, inlineError,
//     deleteTarget, open) so Task 4 can fill in the remaining behavior
//     without re-declaring anything.
//   - Renders the trigger Button + Popover shell, with two sub-modes:
//       * list — branch rows + create/merge entry buttons
//       * create — inline branch-name form with local validation
//   - Dispatches switchBranch for non-current rows and closes the popover
//     on save-required so the existing panel save alert takes over.
//   - Refreshes status + branches whenever the popover opens (so external
//     terminal changes show up the next time the user looks).
//   - Early-returns a disabled trigger + tooltip in conflict state (one
//     branch instead of a disabled=flag in the main render path).
//
// What this file DOES NOT DO (deliberately — Task 4 will add this):
//   - Delete-confirm and merge sub-modes. The mode union is already
//     declared so Task 4 only has to add the rendering branches.
//
// Conflict state is a single early return (the disabled trigger is NOT a
// `disabled` prop toggled in the main path); that keeps the list-mode
// code path free of conditional branches it would never hit.

import { ChevronDown, GitBranch } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { Separator } from '@/components/ui/separator';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { isGitError } from '@/services/git-error';
import { useGitStore } from '@/stores/git-store';
import { GitPanelBranchRow } from './git-panel-branch-row';

type BranchPickerMode = 'list' | 'create' | 'merge' | 'delete-confirm';

export function GitPanelBranchPicker() {
  const { t } = useTranslation();
  const state = useGitStore((s) => s.state);
  const refreshStatus = useGitStore((s) => s.refreshStatus);
  const refreshBranches = useGitStore((s) => s.refreshBranches);
  const createBranch = useGitStore((s) => s.createBranch);
  const switchBranch = useGitStore((s) => s.switchBranch);

  // State machine shared across list/create/merge/delete-confirm modes.
  // Declared here so Task 4 can reach for these without re-declaring.
  const [mode, setMode] = useState<BranchPickerMode>('list');
  const [branchName, setBranchName] = useState('');
  const [inlineError, setInlineError] = useState<string | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);
  const [open, setOpen] = useState(false);

  // Task 4 reads deleteTarget from this closure; keep the void so the
  // unused-variable lint stays quiet until the delete flow lands.
  void deleteTarget;

  const repo = state.kind === 'ready' || state.kind === 'conflict' ? state.repo : null;

  if (!repo) return null;

  if (state.kind === 'conflict') {
    return (
      <Tooltip>
        <TooltipTrigger asChild>
          <span tabIndex={0} className="inline-flex">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              disabled
              aria-label={repo.currentBranch}
              className="pointer-events-none flex max-w-[148px] items-center gap-1 text-muted-foreground"
            >
              <GitBranch size={12} strokeWidth={1.5} aria-hidden />
              <span className="truncate text-xs">{repo.currentBranch}</span>
            </Button>
          </span>
        </TooltipTrigger>
        <TooltipContent side="bottom">{t('git.branch.conflictDisabled')}</TooltipContent>
      </Tooltip>
    );
  }

  async function handleSelectBranch(name: string, isCurrent: boolean) {
    if (isCurrent) return;
    setInlineError(null);
    try {
      await switchBranch(name);
      setOpen(false);
      setMode('list');
    } catch (err) {
      if (isGitError(err) && err.code === 'save-required') {
        // The store has set saveRequiredFor; close the popover so the
        // panel's <GitPanelSaveRequiredAlert> can take over.
        setOpen(false);
        return;
      }
      setInlineError(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleCreateBranch() {
    const name = branchName.trim();
    if (!name) {
      setInlineError(t('git.branch.createEmpty'));
      return;
    }
    if (repo!.branches.some((b) => b.name === name)) {
      setInlineError(t('git.branch.createExists', { name }));
      return;
    }
    setInlineError(null);
    // git-store.createBranch already calls refreshBranches internally, so
    // we do NOT need a second refresh here.
    try {
      await createBranch({ name });
      setBranchName('');
      setMode('list');
    } catch (err) {
      setInlineError(err instanceof Error ? err.message : String(err));
    }
  }

  function beginDelete(_name: string) {
    // Task 4 will populate this to enter delete-confirm mode.
  }

  return (
    <Popover
      open={open}
      onOpenChange={(next) => {
        setOpen(next);
        if (next) {
          void refreshStatus();
          void refreshBranches();
          // Reset sub-mode state every time the popover re-opens so a
          // stale half-typed create form never leaks across sessions.
          setMode('list');
          setBranchName('');
          setInlineError(null);
          setDeleteTarget(null);
        }
      }}
    >
      <PopoverTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          aria-label={repo.currentBranch}
          data-testid="branch-picker-trigger"
          className="flex max-w-[148px] items-center gap-1"
        >
          <GitBranch size={12} strokeWidth={1.5} aria-hidden />
          <span className="truncate text-xs">{repo.currentBranch}</span>
          <ChevronDown size={12} strokeWidth={1.5} aria-hidden />
        </Button>
      </PopoverTrigger>
      <PopoverContent align="start" side="bottom" className="w-[280px] p-1">
        {mode === 'list' && (
          <div className="flex flex-col">
            <p className="px-2 py-1 text-[11px] font-medium text-muted-foreground">
              {t('git.branch.listHeading')}
            </p>
            {repo.branches.map((branch) => (
              <GitPanelBranchRow
                key={branch.name}
                branch={branch}
                onSelect={() => void handleSelectBranch(branch.name, branch.isCurrent)}
                onDelete={branch.isCurrent ? undefined : () => beginDelete(branch.name)}
              />
            ))}
            {inlineError && <p className="px-2 py-1 text-[11px] text-destructive">{inlineError}</p>}
            <Separator className="my-1" />
            <div className="flex items-center justify-end gap-2 px-2 py-1">
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => {
                  setInlineError(null);
                  setBranchName('');
                  setDeleteTarget(null);
                  setMode('create');
                }}
              >
                {t('git.branch.createAction')}
              </Button>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => {
                  setInlineError(null);
                  setDeleteTarget(null);
                  setMode('merge');
                }}
              >
                {t('git.branch.mergeAction')}
              </Button>
            </div>
          </div>
        )}
        {mode === 'create' && (
          <div className="flex flex-col gap-2 p-2">
            <input
              type="text"
              value={branchName}
              onChange={(e) => setBranchName(e.target.value)}
              placeholder={t('git.branch.createPlaceholder')}
              autoFocus
              className="w-full rounded-md border border-input bg-secondary px-2 py-1.5 text-xs text-foreground placeholder:text-muted-foreground focus:border-primary focus:outline-none"
            />
            {inlineError && <p className="text-[11px] text-destructive">{inlineError}</p>}
            <div className="flex justify-end gap-2">
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => {
                  setInlineError(null);
                  setBranchName('');
                  setMode('list');
                }}
              >
                {t('git.branch.cancel')}
              </Button>
              <Button type="button" size="sm" onClick={() => void handleCreateBranch()}>
                {t('git.branch.createSubmit')}
              </Button>
            </div>
          </div>
        )}
      </PopoverContent>
    </Popover>
  );
}
