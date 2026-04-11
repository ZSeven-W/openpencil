// apps/web/src/components/panels/git-panel/git-panel-conflict-list.tsx
//
// Conflict workspace mounted below the banner in GitPanelConflict. Renders all
// node conflicts and doc-field conflicts interleaved in document tree order,
// with bulk-action buttons to choose all-ours or all-theirs across unresolved
// items.
//
// Bulk actions stay renderer-side by looping resolveConflict() over unresolved
// items — no new IPC call is required. The banner already owns the primary
// apply/continue button; the list owns only the bulk action shortcuts.

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

  // Only render in conflict state.
  if (state.kind !== 'conflict') return null;

  const { nodeConflicts, docFieldConflicts } = state.conflicts;

  // Build a flat list ordered by document tree position. orderConflicts walks
  // the current document depth-first so ours/theirs previews appear in the same
  // sequence as the layer panel. Orphan conflicts (node deleted in theirs) are
  // appended at the end; doc-field conflicts follow, sorted alphabetically by path.
  // useMemo is intentionally omitted — the conflict list is small and the tree
  // walk is O(n) over a modest set, so memoisation adds complexity without benefit.
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

  // Bulk-action handlers: iterate unresolved items and dispatch resolveConflict.
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
      {/* Section header with bulk actions */}
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

        {/* Bulk actions — only shown when there are unresolved items */}
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

      {/* Conflict item list — plain div with max-height; no shadcn ScrollArea needed */}
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
