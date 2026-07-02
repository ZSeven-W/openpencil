// apps/web/src/components/panels/git-panel/git-panel-node-conflict-card.tsx
//
// Card for a single node conflict. Shows:
//   - Reason label
//   - Ours / theirs thumbnails side by side (or placeholder when rendering unavailable)
//   - Per-side choose buttons
//   - Manual JSON editor toggle
//
// Resolution state is owned by the parent via the onResolve callback —
// this card is stateless with respect to the store.

import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { GitPanelConflictJsonEditor } from './git-panel-conflict-json-editor';
import { formatConflictReason, prettyJson } from './conflict-formatters';
import type { GitConflictBag, GitConflictResolution } from '@/services/git-types';
import { useDocumentStore } from '@/stores/document-store';

/** Minimal inline badge — shadcn Badge not available in this project. */
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

type NodeConflictEntry = GitConflictBag['nodeConflicts'][number] & {
  resolution?: GitConflictResolution;
};

export interface GitPanelNodeConflictCardProps {
  conflict: NodeConflictEntry;
  onResolve: (choice: GitConflictResolution) => void;
}

export function GitPanelNodeConflictCard({ conflict, onResolve }: GitPanelNodeConflictCardProps) {
  const { t } = useTranslation();
  const [showEditor, setShowEditor] = useState(false);
  const penDocument = useDocumentStore((s) => s.document);

  // Thumbnail data URLs — null means "not rendered yet" or "unavailable".
  const [oursThumbnail, setOursThumbnail] = useState<string | null>(null);
  const [theirsThumbnail, setTheirsThumbnail] = useState<string | null>(null);

  const isResolved = conflict.resolution != null;

  // Attempt to render thumbnails via the pen-renderer helper.
  // We import lazily so SSR / test environments that lack CanvasKit don't crash.
  useEffect(() => {
    let cancelled = false;

    async function renderThumbnails() {
      try {
        const { renderNodeThumbnail } = await import('@zseven-w/pen-renderer');
        // Use the real document so $variable references and ref-type nodes resolve
        // correctly. A stub document with empty children silently broke resolution.
        const ctx = { document: penDocument, pageId: conflict.pageId, size: 120 };

        if (conflict.ours && typeof conflict.ours === 'object') {
          const url = await renderNodeThumbnail(
            conflict.ours as import('@zseven-w/pen-types').PenNode,
            ctx,
          );
          if (!cancelled) setOursThumbnail(url);
        }
        if (conflict.theirs && typeof conflict.theirs === 'object') {
          const url = await renderNodeThumbnail(
            conflict.theirs as import('@zseven-w/pen-types').PenNode,
            ctx,
          );
          if (!cancelled) setTheirsThumbnail(url);
        }
      } catch {
        // Thumbnails are best-effort; rendering failures are silent.
      }
    }

    void renderThumbnails();
    return () => {
      cancelled = true;
    };
  }, [conflict.ours, conflict.theirs, conflict.pageId, penDocument]);

  function handleEditorSubmit(value: unknown) {
    onResolve({ kind: 'manual-node', node: value });
    setShowEditor(false);
  }

  const reasonLabel = formatConflictReason(conflict.reason);

  return (
    <div
      className="flex flex-col gap-2 rounded border border-border bg-card p-3"
      data-testid={`node-conflict-card-${conflict.id}`}
    >
      {/* Header: type badge + nodeId + resolved badge */}
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-1.5 min-w-0">
          <InlineBadge variant="outline">{t('git.conflict.item.nodeConflict')}</InlineBadge>
          <span className="text-xs text-muted-foreground font-mono truncate">
            {conflict.nodeId}
          </span>
        </div>
        {isResolved && (
          <InlineBadge variant="primary">{t('git.conflict.item.resolved')}</InlineBadge>
        )}
      </div>

      {/* Reason */}
      <p className="text-xs text-muted-foreground">{reasonLabel}</p>

      {/* Side-by-side thumbnail comparison */}
      <div className="grid grid-cols-2 gap-2">
        {/* Ours */}
        <div className="flex flex-col gap-1">
          <span className="text-[10px] font-medium text-foreground">
            {t('git.conflict.card.ours')}
          </span>
          <div
            className="flex h-[120px] items-center justify-center rounded border border-border bg-muted overflow-hidden"
            data-testid={`node-ours-thumbnail-${conflict.id}`}
          >
            {oursThumbnail ? (
              <img
                src={oursThumbnail}
                alt={t('git.conflict.card.oursThumbnailAlt')}
                className="max-h-full max-w-full object-contain"
              />
            ) : (
              <pre className="max-h-full w-full overflow-auto p-1 text-[9px] text-muted-foreground">
                {prettyJson(conflict.ours).slice(0, 200)}
              </pre>
            )}
          </div>
          <Button
            type="button"
            variant={conflict.resolution?.kind === 'ours' ? 'default' : 'outline'}
            size="sm"
            className={
              conflict.resolution?.kind === 'ours' ? 'bg-primary text-primary-foreground' : ''
            }
            onClick={() => onResolve({ kind: 'ours' })}
            data-testid={`node-choose-ours-${conflict.id}`}
          >
            {t('git.conflict.card.keepMine')}
          </Button>
        </div>

        {/* Theirs */}
        <div className="flex flex-col gap-1">
          <span className="text-[10px] font-medium text-foreground">
            {t('git.conflict.card.theirs')}
          </span>
          <div
            className="flex h-[120px] items-center justify-center rounded border border-border bg-muted overflow-hidden"
            data-testid={`node-theirs-thumbnail-${conflict.id}`}
          >
            {theirsThumbnail ? (
              <img
                src={theirsThumbnail}
                alt={t('git.conflict.card.theirsThumbnailAlt')}
                className="max-h-full max-w-full object-contain"
              />
            ) : (
              <pre className="max-h-full w-full overflow-auto p-1 text-[9px] text-muted-foreground">
                {prettyJson(conflict.theirs).slice(0, 200)}
              </pre>
            )}
          </div>
          <Button
            type="button"
            variant={conflict.resolution?.kind === 'theirs' ? 'default' : 'outline'}
            size="sm"
            className={
              conflict.resolution?.kind === 'theirs' ? 'bg-primary text-primary-foreground' : ''
            }
            onClick={() => onResolve({ kind: 'theirs' })}
            data-testid={`node-choose-theirs-${conflict.id}`}
          >
            {t('git.conflict.card.keepTheirs')}
          </Button>
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
          data-testid={`node-edit-manual-${conflict.id}`}
        >
          {t('git.conflict.editor.editManually')}
        </Button>
      ) : (
        <GitPanelConflictJsonEditor
          initialValue={prettyJson(conflict.ours)}
          mode="node"
          nodeId={conflict.nodeId}
          onSubmit={handleEditorSubmit}
          onCancel={() => setShowEditor(false)}
        />
      )}
    </div>
  );
}
