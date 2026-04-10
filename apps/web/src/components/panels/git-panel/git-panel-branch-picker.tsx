// apps/web/src/components/panels/git-panel/git-panel-branch-picker.tsx
//
// Phase 5 Task 2: shell-only branch picker.
//
// What this file DOES:
//   - Declares the full state machine (mode, branchName, inlineError,
//     deleteTarget, open) so Tasks 3/4 can fill in the behavior without
//     re-declaring anything.
//   - Renders the trigger Button + Popover shell, with the list-mode body:
//     heading, branch rows, and the create/merge entry buttons.
//   - Refreshes status + branches whenever the popover opens (so external
//     terminal changes show up the next time the user looks).
//   - Early-returns a disabled trigger + tooltip in conflict state (one
//     branch instead of a disabled=flag in the main render path).
//
// What this file DOES NOT DO (deliberately — Tasks 3/4 will add these):
//   - Create, switch, delete, or merge branches. handleSelectBranch and
//     beginDelete are stub no-ops with a comment below explaining why.
//   - Render the create / merge / delete-confirm sub-modes. The mode
//     union is already declared here so those tasks only have to add the
//     rendering branches.
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
import { useGitStore } from '@/stores/git-store';
import { GitPanelBranchRow } from './git-panel-branch-row';

type BranchPickerMode = 'list' | 'create' | 'merge' | 'delete-confirm';

export function GitPanelBranchPicker() {
  const { t } = useTranslation();
  const state = useGitStore((s) => s.state);
  const refreshStatus = useGitStore((s) => s.refreshStatus);
  const refreshBranches = useGitStore((s) => s.refreshBranches);

  // State machine shared across list/create/merge/delete-confirm modes.
  // Declared here so Tasks 3/4 can reach for these without re-declaring.
  const [mode, setMode] = useState<BranchPickerMode>('list');
  const [branchName, setBranchName] = useState('');
  const [inlineError, setInlineError] = useState<string | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);
  const [open, setOpen] = useState(false);

  // Silence the unused-variable warnings for the shell commit. Tasks 3/4
  // read these from closures; Task 2 only needs them declared so the
  // state machine is already wired when those tasks arrive.
  void mode;
  void branchName;
  void inlineError;
  void deleteTarget;
  void setBranchName;

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

  // Shell stubs — Task 3 wires switch/create, Task 4 wires delete/merge.
  // Keeping these as declared functions (not inline arrows) keeps the
  // render path identical to the final shape, so the diff for Tasks 3/4
  // only touches the function bodies, not the call sites.
  async function handleSelectBranch(_name: string, _isCurrent: boolean) {
    // Task 3 will populate this with the switch-branch flow.
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
          className="flex max-w-[148px] items-center gap-1"
        >
          <GitBranch size={12} strokeWidth={1.5} aria-hidden />
          <span className="truncate text-xs">{repo.currentBranch}</span>
          <ChevronDown size={12} strokeWidth={1.5} aria-hidden />
        </Button>
      </PopoverTrigger>
      <PopoverContent align="start" side="bottom" className="w-[280px] p-1">
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
      </PopoverContent>
    </Popover>
  );
}
