// apps/web/src/components/panels/git-panel/git-panel-conflict-list.tsx
//
// Conflict 工作区安装在 GitPanelConflict 中的横幅下方。全部 Renders
// 节点冲突和文档字段冲突按文档树顺序交错，
// 使用批量操作按钮可以在未解决的问题上选择我们的所有或他们的所有
// 项目。
//
// Bulk 操作通过在未解决的问题上循环 resolveConflict() 来保持渲染器端
// items — 不需要新的 IPC 调用。 The 横幅已经拥有主
// apply/continue 按钮；该列表仅拥有批量操作快捷方式。

import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { useGitStore } from '@/stores/git-store';
import { useDocumentStore } from '@/stores/document-store';
import { orderConflicts } from './conflict-formatters';
import { GitPanelConflictItem } from './git-panel-conflict-item';
import type {
  ConflictItemData,
  FieldConflictItemData,
  NodeConflictItemData,
} from './git-panel-conflict-item';

export function GitPanelConflictList() {
  const { t } = useTranslation();
  const state = useGitStore((s) => s.state);
  const resolveConflict = useGitStore((s) => s.resolveConflict);
  const document = useDocumentStore((s) => s.document);

  // Only 在冲突状态下渲染。
  if (state.kind !== 'conflict') return null;

  const { nodeConflicts, docFieldConflicts } = state.conflicts;

  // Build 按文档树位置排序的平面列表。 orderConflicts 以深度优先方式遍历当前文档，因此 ours/theirs
  // 预览的显示顺序与图层面板相同。 Orphan 冲突（节点已删除）添加在末尾；文档字段冲突如下，按路径字母顺序排序。 useMemo
  // 被故意省略——冲突列表很小，树遍历是 O(n) 在一个适度的集合上，所以记忆增加了复杂性，但没有任何好处。
  const ordered = orderConflicts(document, nodeConflicts, docFieldConflicts);

  const items: ConflictItemData[] = ordered.map((c) => {
    if ('nodeId' in c) {
      return {
        kind: 'node',
        id: c.id,
        pageId: c.pageId,
        nodeId: c.nodeId,
        reason: c.reason,
        base: c.base,
        ours: c.ours,
        theirs: c.theirs,
        resolution: c.resolution,
      } satisfies NodeConflictItemData;
    }
    return {
      kind: 'field',
      id: c.id,
      field: c.field,
      path: c.path,
      base: c.base,
      ours: c.ours,
      theirs: c.theirs,
      resolution: c.resolution,
    } satisfies FieldConflictItemData;
  });

  const totalCount = items.length;
  const resolvedCount = items.filter((i) => i.resolution != null).length;
  const unresolvedItems = items.filter((i) => i.resolution == null);
  const allResolved = totalCount > 0 && resolvedCount === totalCount;

  // Bulk-action 处理程序：迭代未解决的项目并分派 resolveConflict。
  function handleSelectAllOurs() {
    for (const item of unresolvedItems) {
      void resolveConflict(item.id, { kind: 'ours' });
    }
  }

  function handleSelectAllTheirs() {
    for (const item of unresolvedItems) {
      void resolveConflict(item.id, { kind: 'theirs' });
    }
  }

  if (totalCount === 0) return null;

  return (
    <div className="flex flex-col gap-0" data-testid="conflict-list">
      {/* 具有批量操作的 Section 标头 */}
      <div className="flex items-center justify-between gap-2 px-4 py-2">
        <div className="flex items-center gap-2">
          <span className="text-xs font-medium text-foreground">
            {t('git.conflict.list.heading')}
          </span>
          {allResolved ? (
            <span className="text-[10px] text-primary" data-testid="conflict-list-all-resolved">
              {t('git.conflict.list.allResolved')}
            </span>
          ) : (
            <span
              className="text-[10px] text-muted-foreground"
              data-testid="conflict-list-progress"
            >
              {t('git.conflict.list.progress', { resolved: resolvedCount, total: totalCount })}
            </span>
          )}
        </div>

        {/* Bulk 操作 — 仅在存在未解决的项目时显示 */}
        {unresolvedItems.length > 0 && (
          <div className="flex items-center gap-1" data-testid="bulk-actions">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-6 px-2 text-[11px]"
              onClick={handleSelectAllOurs}
              data-testid="bulk-ours"
            >
              {t('git.conflict.list.allOurs')}
            </Button>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-6 px-2 text-[11px]"
              onClick={handleSelectAllTheirs}
              data-testid="bulk-theirs"
            >
              {t('git.conflict.list.allTheirs')}
            </Button>
          </div>
        )}
      </div>

      <Separator />

      {/* Conflict 项目列表 — 具有最大高度的普通 div；不需要 shadcn ScrollArea */}
      <div className="max-h-[400px] overflow-y-auto">
        <div className="flex flex-col gap-3 p-4" data-testid="conflict-items">
          {items.map((item) => (
            <GitPanelConflictItem key={item.id} item={item} />
          ))}
        </div>
      </div>
    </div>
  );
}
