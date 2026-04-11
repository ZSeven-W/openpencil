// apps/web/src/components/panels/git-panel/git-panel-branch-row.tsx
//
// Presenter row for a single branch in the Phase 5 branch picker popover.
//
// Important layout invariant: the select button and the delete button are
// SIBLINGS inside the outer div — NOT nested. Radix/A11y rules forbid
// interactive controls inside other interactive controls, and the Phase 5
// plan's self-review checklist calls this out specifically.

import { Check, Trash2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { GitBranchInfo } from '@/services/git-types';

export function GitPanelBranchRow({
  branch,
  onSelect,
  onDelete,
}: {
  branch: GitBranchInfo;
  onSelect: () => void;
  onDelete?: () => void;
}) {
  const { t } = useTranslation();

  return (
    <div className="flex items-start gap-1 rounded-sm px-1 py-1 hover:bg-accent">
      <button
        type="button"
        onClick={onSelect}
        className="min-w-0 flex-1 rounded-sm px-1 py-0.5 text-left"
        aria-label={branch.name}
      >
        <span className="block truncate text-xs font-medium">{branch.name}</span>
        <span className="block truncate text-[11px] text-muted-foreground">
          {branch.lastCommit?.message ?? t('git.branch.noCommits')}
        </span>
        {(branch.ahead > 0 || branch.behind > 0) && (
          <span className="mt-1 block text-[10px] text-muted-foreground">
            ↑{branch.ahead} ↓{branch.behind}
          </span>
        )}
      </button>
      <span className="ml-1 flex items-center gap-1 self-center">
        {branch.isCurrent && <Check size={12} strokeWidth={1.5} aria-hidden />}
        {!branch.isCurrent && onDelete && (
          <button
            type="button"
            onClick={onDelete}
            aria-label={t('git.branch.deleteLabel', { name: branch.name })}
            className="rounded-sm p-1 text-muted-foreground hover:bg-accent-foreground/10"
          >
            <Trash2 size={12} strokeWidth={1.5} aria-hidden />
          </button>
        )}
      </span>
    </div>
  );
}
