// apps/web/src/components/panels/git-panel/git-panel-branch-picker.tsx
//
// Phase 5 Task 3 + Task 4: branch picker for the git panel.
//
// What this file DOES:
//   - Declares the full state machine (mode, branchName, inlineError,
//     deleteTarget, canForce, open).
//   - Renders the trigger Button + Popover shell, with four sub-modes:
//       * list           — branch rows + create/merge entry buttons
//       * create         — inline branch-name form with local validation
//       * delete-confirm — destructive confirm with opt-in force retry
//       * merge          — list of non-current branches to merge into HEAD
//   - Dispatches switchBranch for non-current rows and closes the popover
//     on save-required so the existing panel save alert takes over.
//   - For delete, surfaces engine errors inline; only offers a Force
//     Delete retry when the store returns `branch-unmerged`.
//   - For merge, relies on the store's conflict transition — mergeBranch
//     does NOT throw on conflict; it flips state.kind to 'conflict' which
//     fires this component's top-level early return on the next render.
//   - Refreshes status + branches whenever the popover opens (so external
//     terminal changes show up the next time the user looks).
//   - Early-returns a disabled trigger + tooltip in conflict state (one
//     branch instead of a disabled=flag in the main render path).
//
// Conflict state is a single early return (the disabled trigger is NOT a
// `disabled` prop toggled in the main path); that keeps the list-mode
// code path free of conditional branches it would never hit.

import { ChevronDown, ChevronRight, GitBranch } from 'lucide-react';
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
  const deleteBranch = useGitStore((s) => s.deleteBranch);
  const mergeBranch = useGitStore((s) => s.mergeBranch);

  // State machine shared across list/create/merge/delete-confirm modes.
  const [mode, setMode] = useState<BranchPickerMode>('list');
  const [branchName, setBranchName] = useState('');
  const [inlineError, setInlineError] = useState<string | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);
  const [canForce, setCanForce] = useState(false);
  const [open, setOpen] = useState(false);

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

  // Narrowed alias so nested handlers do not need `repo!`. TS loses the
  // `repo` null-narrowing inside nested function expressions, and the
  // single alias here lets every handler below speak in terms of
  // `activeRepo.branches` / `activeRepo.currentBranch`.
  const activeRepo = repo;

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
    if (activeRepo.branches.some((b) => b.name === name)) {
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

  function beginDelete(name: string) {
    setDeleteTarget(name);
    setInlineError(null);
    setCanForce(false);
    setMode('delete-confirm');
  }

  async function handleDelete(force = false) {
    if (!deleteTarget) return;
    try {
      await deleteBranch(deleteTarget, force ? { force: true } : undefined);
      setDeleteTarget(null);
      setCanForce(false);
      setMode('list');
    } catch (err) {
      // Only upgrade to a force-delete retry when the store returns the
      // specific `branch-unmerged` code — other errors (engine-crash,
      // permission failures, etc.) surface as inline messages without
      // offering force, because force cannot help in those cases.
      if (isGitError(err) && err.code === 'branch-unmerged' && !force) {
        setInlineError(t('git.branch.deleteWarning', { name: deleteTarget }));
        setCanForce(true);
        return;
      }
      setCanForce(false);
      setInlineError(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleMerge(fromBranch: string) {
    setInlineError(null);
    try {
      // mergeBranch does NOT throw on conflict — it flips state.kind to
      // 'conflict' and resolves. Our top-level early return observes that
      // and renders the disabled-trigger tooltip on the next render, so
      // the happy path here is simply "close the popover and go home".
      await mergeBranch(fromBranch);
      setOpen(false);
      setMode('list');
    } catch (err) {
      if (isGitError(err) && err.code === 'save-required') {
        // The panel's <GitPanelSaveRequiredAlert> takes over.
        setOpen(false);
        return;
      }
      setInlineError(err instanceof Error ? err.message : String(err));
    }
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
          setCanForce(false);
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
            {activeRepo.branches.map((branch) => (
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
        {mode === 'delete-confirm' && deleteTarget && (
          <div className="flex flex-col gap-2 p-2">
            <p className="text-xs text-foreground">
              {t('git.branch.deletePrompt', { name: deleteTarget })}
            </p>
            {inlineError && <p className="text-[11px] text-destructive">{inlineError}</p>}
            <div className="flex justify-end gap-2">
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => {
                  setDeleteTarget(null);
                  setCanForce(false);
                  setInlineError(null);
                  setMode('list');
                }}
              >
                {t('git.branch.cancel')}
              </Button>
              <Button
                type="button"
                variant="destructive"
                size="sm"
                onClick={() => void handleDelete(false)}
              >
                {t('git.branch.deleteConfirm')}
              </Button>
              {canForce && (
                <Button
                  type="button"
                  variant="destructive"
                  size="sm"
                  onClick={() => void handleDelete(true)}
                >
                  {t('git.branch.deleteForce')}
                </Button>
              )}
            </div>
          </div>
        )}
        {mode === 'merge' && (
          <div className="flex flex-col gap-1 p-1">
            <p className="px-2 py-1 text-[11px] font-medium text-muted-foreground">
              {t('git.branch.mergeHeading', { name: activeRepo.currentBranch })}
            </p>
            {inlineError && <p className="px-2 text-[11px] text-destructive">{inlineError}</p>}
            {activeRepo.branches
              .filter((branch) => !branch.isCurrent)
              .map((branch) => (
                <button
                  key={branch.name}
                  type="button"
                  onClick={() => void handleMerge(branch.name)}
                  aria-label={branch.name}
                  className="flex w-full items-center justify-between rounded-sm px-2 py-1.5 text-left hover:bg-accent"
                >
                  <span className="text-xs">{branch.name}</span>
                  <ChevronRight size={12} strokeWidth={1.5} aria-hidden />
                </button>
              ))}
            <div className="flex justify-end px-2 py-1">
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => {
                  setInlineError(null);
                  setMode('list');
                }}
              >
                {t('git.branch.cancel')}
              </Button>
            </div>
          </div>
        )}
      </PopoverContent>
    </Popover>
  );
}
