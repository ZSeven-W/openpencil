// apps/web/src/components/panels/git-panel/git-panel-field-conflict-card.tsx
//
// Card 用于单个文档字段冲突。 Shows:
//   - Field 名称 + 路径
//   - Pretty-打印基础/我们的/他们的价值观
//   - 我们/他们的 Choose 按钮
//   - Manual JSON 编辑器切换（接受任何有效的 JSON 值）
//
// Resolution 状态由父级通过 onResolve 回调拥有。

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { GitPanelConflictJsonEditor } from './git-panel-conflict-json-editor';
import { prettyJson } from './conflict-formatters';
import type { GitConflictBag, GitConflictResolution } from '@/services/git-types';

/** Minimal 内联徽章 — shadcn Badge 在此项目中不可用。 */
function InlineBadge({
  children,
  variant = 'outline',
}: {
  children: React.ReactNode;
  variant?: 'outline' | 'primary';
}) {
  const cls =
    variant === 'primary'
      ? 'inline-flex items-center rounded px-1.5 py-0.5 text-[10px] font-medium bg-primary text-primary-foreground'
      : 'inline-flex items-center rounded border border-border px-1.5 py-0.5 text-[10px] font-medium';
  return <span className={cls}>{children}</span>;
}

type DocFieldConflictEntry = GitConflictBag['docFieldConflicts'][number] & {
  resolution?: GitConflictResolution;
};

export interface GitPanelFieldConflictCardProps {
  conflict: DocFieldConflictEntry;
  onResolve: (choice: GitConflictResolution) => void;
}

export function GitPanelFieldConflictCard({ conflict, onResolve }: GitPanelFieldConflictCardProps) {
  const { t } = useTranslation();
  const [showEditor, setShowEditor] = useState(false);

  const isResolved = conflict.resolution != null;

  function handleEditorSubmit(value: unknown) {
    onResolve({ kind: 'manual-field', value });
    setShowEditor(false);
  }

  return (
    <div
      className="flex flex-col gap-2 rounded border border-border bg-card p-3"
      data-testid={`field-conflict-card-${conflict.id}`}
    >
      {/* Header: type badge + field name + resolved badge */}
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-1.5 min-w-0">
          <InlineBadge variant="outline">{t('git.conflict.item.fieldConflict')}</InlineBadge>
          <span className="text-xs text-muted-foreground font-mono truncate">{conflict.field}</span>
        </div>
        {isResolved && (
          <InlineBadge variant="primary">{t('git.conflict.item.resolved')}</InlineBadge>
        )}
      </div>

      {/* Field path */}
      {conflict.path && conflict.path !== conflict.field && (
        <p className="text-[10px] text-muted-foreground font-mono truncate">{conflict.path}</p>
      )}

      {/* Three-way value comparison */}
      <div className="flex flex-col gap-1.5">
        {/* Base */}
        <div className="flex flex-col gap-0.5">
          <span className="text-[10px] font-medium text-muted-foreground">
            {t('git.conflict.card.base')}
          </span>
          <pre
            className="max-h-[60px] overflow-auto rounded border border-border bg-muted p-1.5 text-[10px] text-foreground"
            data-testid={`field-base-value-${conflict.id}`}
          >
            {prettyJson(conflict.base)}
          </pre>
        </div>

        <div className="grid grid-cols-2 gap-2">
          {/* Ours */}
          <div className="flex flex-col gap-1">
            <span className="text-[10px] font-medium text-foreground">
              {t('git.conflict.card.ours')}
            </span>
            <pre
              className="max-h-[80px] overflow-auto rounded border border-border bg-muted p-1.5 text-[10px] text-foreground"
              data-testid={`field-ours-value-${conflict.id}`}
            >
              {prettyJson(conflict.ours)}
            </pre>
            <Button
              type="button"
              variant={conflict.resolution?.kind === 'ours' ? 'default' : 'outline'}
              size="sm"
              className={
                conflict.resolution?.kind === 'ours' ? 'bg-primary text-primary-foreground' : ''
              }
              onClick={() => onResolve({ kind: 'ours' })}
              data-testid={`field-choose-ours-${conflict.id}`}
            >
              {t('git.conflict.card.keepMine')}
            </Button>
          </div>

          {/* Theirs */}
          <div className="flex flex-col gap-1">
            <span className="text-[10px] font-medium text-foreground">
              {t('git.conflict.card.theirs')}
            </span>
            <pre
              className="max-h-[80px] overflow-auto rounded border border-border bg-muted p-1.5 text-[10px] text-foreground"
              data-testid={`field-theirs-value-${conflict.id}`}
            >
              {prettyJson(conflict.theirs)}
            </pre>
            <Button
              type="button"
              variant={conflict.resolution?.kind === 'theirs' ? 'default' : 'outline'}
              size="sm"
              className={
                conflict.resolution?.kind === 'theirs' ? 'bg-primary text-primary-foreground' : ''
              }
              onClick={() => onResolve({ kind: 'theirs' })}
              data-testid={`field-choose-theirs-${conflict.id}`}
            >
              {t('git.conflict.card.keepTheirs')}
            </Button>
          </div>
        </div>
      </div>

      {/* Manual JSON editor toggle */}
      {!showEditor ? (
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="self-start text-xs"
          onClick={() => setShowEditor(true)}
          data-testid={`field-edit-manual-${conflict.id}`}
        >
          {t('git.conflict.editor.editManually')}
        </Button>
      ) : (
        <GitPanelConflictJsonEditor
          initialValue={prettyJson(conflict.ours)}
          mode="field"
          onSubmit={handleEditorSubmit}
          onCancel={() => setShowEditor(false)}
        />
      )}
    </div>
  );
}
