// apps/web/src/components/panels/git-panel/git-panel-branch-picker.tsx
//
// Phase 5 Task 3 + Task 4：git 面板的分支选择器。
//
// What 这个文件 DOES：
//   - Declares 完整状态机（模式、branchName、inlineError、
// deleteTarget、canForce，打开）。
//   - Renders 触发 Button + Popover shell，有四个子模式：
//       * list — 分支行 + create/merge 输入按钮
//       * create — 具有本地验证的内联分支名称形式
//       * 删除确认 — 通过选择强制重试进行破坏性确认
//       * merge — 要合并到 HEAD 的非当前分支列表
//   - Dispatches switchBranch 用于非当前行并关闭弹出窗口
// 需要保存，以便现有面板保存警报接管。
//   - For 删除，内联显示引擎错误；只提供 Force
// 当存储返回 `branch-unmerged` 时重试 Delete。
//   - For 合并，依赖于 store 的冲突转换 — mergeBranch
// NOT 是否会引发冲突；它将 state.kind 翻转为“冲突”
// 在下一次渲染时触发该组件的顶级早期返回。
//   - Refreshes 状态 + 每当弹出窗口打开时分支（因此外部
// 终端更改会在用户下次查看时显示）。
//   - Early - 返回处于冲突状态的禁用触发器+工具提示（一个
// 分支而不是主渲染路径中的禁用=标志）。
//
// Conflict 状态是单个早期返回（禁用的触发器是 NOT a
// `disabled` 属性在主路径中切换）；保持列表模式
// 没有条件分支的代码路径永远不会命中。

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

  // State 机器在 list/create/merge/delete-confirm 模式之间共享。
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

  // Narrowed 别名，因此嵌套处理程序不需要 `repo!`。 TS 失去了嵌套函数表达式内的 `repo`
  // 空缩小，并且这里的单个别名让下面的每个处理程序都可以使用 `activeRepo.branches` /
  // `activeRepo.currentBranch` 来说话。
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
        // The 商店已设置 saveRequiredFor；关闭弹出窗口，以便面板的
        // <GitPanelSaveRequiredAlert> 可以接管。
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
    // git-store.createBranch 已经在内部调用了 refreshBranches，所以我们 NOT
    // 需要在这里进行第二次刷新。
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
      // 当存储返回特定的 `branch-unmerged` 代码时，Only 升级到强制删除重试 -
      // 其他错误（引擎崩溃、权限失败等）会以内联消息的形式出现，而不提供强制，因为在这些情况下强制无法提供帮助。
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
      // mergeBranch 会引发冲突 - 它将 state.kind 翻转为“冲突”并解决。 Our
      // 顶级早期返回会观察到这一点，并在下一次渲染时渲染禁用触发工具提示，因此这里的快乐路径只是“关闭弹出窗口并回家”。
      await mergeBranch(fromBranch);
      setOpen(false);
      setMode('list');
    } catch (err) {
      if (isGitError(err) && err.code === 'save-required') {
        // The 面板的 <GitPanelSaveRequiredAlert> 接管。
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
