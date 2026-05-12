import { useEffect, useState } from 'react';
import { Activity, ChevronDown, Pencil, RotateCcw, Save, X } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  listCloudFileActivity,
  listCloudFileVersions,
  restoreCloudFileVersion,
  updateCloudFileVersionLabel,
} from '@/services/cloud/cloud-files';
import type { CloudActivityEvent, CloudActivityEventType, CloudFileVersion } from '@/types/cloud';
import { useCloudVersionPanelStore } from '@/stores/cloud-version-panel-store';
import { useDocumentStore } from '@/stores/document-store';

function formatVersionDate(value: string) {
  return new Date(value).toLocaleString();
}

function formatBytes(value: number) {
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${Math.round(value / 102.4) / 10} KB`;
  return `${Math.round(value / 1024 / 102.4) / 10} MB`;
}

const ACTIVITY_FILTERS: Array<{ value: CloudActivityEventType | 'all'; label: string }> = [
  { value: 'all', label: 'All activity' },
  { value: 'file_saved', label: 'Saves' },
  { value: 'file_restored', label: 'Restores' },
  { value: 'file_shared', label: 'Shares' },
  { value: 'codegen_created', label: 'Codegen' },
  { value: 'version_labeled', label: 'Labels' },
];

function formatActivityType(type: CloudActivityEventType) {
  return type.replaceAll('_', ' ');
}

function formatActivitySummary(item: CloudActivityEvent) {
  const revision =
    typeof item.metadata.revision === 'number' ? `rev ${item.metadata.revision}` : null;
  const label = typeof item.metadata.label === 'string' ? item.metadata.label : null;
  const framework = typeof item.metadata.framework === 'string' ? item.metadata.framework : null;
  const parts = [revision, label, framework].filter(Boolean);
  return parts.length > 0 ? parts.join(' · ') : 'No extra details';
}

export function CloudVersionHistoryPanel() {
  const open = useCloudVersionPanelStore((s) => s.open);
  const setOpen = useCloudVersionPanelStore((s) => s.setOpen);
  const cloudFileId = useDocumentStore((s) => s.cloudFileId);
  const cloudRevision = useDocumentStore((s) => s.cloudRevision);
  const cloudShareRole = useDocumentStore((s) => s.cloudShareRole);
  const [versions, setVersions] = useState<CloudFileVersion[]>([]);
  const [activity, setActivity] = useState<CloudActivityEvent[]>([]);
  const [activityFilter, setActivityFilter] = useState<CloudActivityEventType | 'all'>('all');
  const [activityCursor, setActivityCursor] = useState<string | null>(null);
  const [loadingMoreActivity, setLoadingMoreActivity] = useState(false);
  const [loading, setLoading] = useState(false);
  const [restoringId, setRestoringId] = useState<string | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [labelDraft, setLabelDraft] = useState('');
  const [savingLabelId, setSavingLabelId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open || !cloudFileId) return;
    const fileId = cloudFileId;
    let cancelled = false;
    setLoading(true);
    setError(null);
    Promise.all([listCloudFileVersions(fileId), loadActivityPage(fileId, null)])
      .then(([items, page]) => {
        if (!cancelled) {
          setVersions(items);
          setActivity(page.data);
          setActivityCursor(page.nextCursor);
        }
      })
      .catch((err) => {
        if (!cancelled) setError(err instanceof Error ? err.message : 'Failed to load versions');
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [activityFilter, cloudFileId, open]);

  const loadActivityPage = (fileId: string, cursor: string | null) =>
    listCloudFileActivity(fileId, {
      type: activityFilter,
      cursor,
      limit: 20,
    }).catch(() => ({ data: [], nextCursor: null, limit: 20 }));

  if (!open || !cloudFileId) return null;
  const canManageVersions = cloudShareRole !== 'viewer';

  const restoreVersion = async (version: CloudFileVersion) => {
    setRestoringId(version.id);
    setError(null);
    try {
      const file = await restoreCloudFileVersion({ fileId: cloudFileId, versionId: version.id });
      useDocumentStore.getState().loadCloudDocument(file);
      setVersions(await listCloudFileVersions(cloudFileId));
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to restore version');
    } finally {
      setRestoringId(null);
    }
  };

  const saveLabel = async (version: CloudFileVersion) => {
    setSavingLabelId(version.id);
    setError(null);
    try {
      const updated = await updateCloudFileVersionLabel({
        fileId: cloudFileId,
        versionId: version.id,
        label: labelDraft.trim() || null,
      });
      setVersions((items) => items.map((item) => (item.id === updated.id ? updated : item)));
      const page = await loadActivityPage(cloudFileId, null);
      setActivity(page.data);
      setActivityCursor(page.nextCursor);
      setEditingId(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to update version label');
    } finally {
      setSavingLabelId(null);
    }
  };

  const loadMoreActivity = async () => {
    if (!activityCursor) return;
    setLoadingMoreActivity(true);
    setError(null);
    try {
      const page = await loadActivityPage(cloudFileId, activityCursor);
      setActivity((items) => [...items, ...page.data]);
      setActivityCursor(page.nextCursor);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load more activity');
    } finally {
      setLoadingMoreActivity(false);
    }
  };

  return (
    <aside
      role="complementary"
      aria-label="Version history"
      className="absolute right-3 top-3 z-30 flex max-h-[calc(100%-1.5rem)] w-80 flex-col rounded-lg border border-border bg-card text-card-foreground shadow-lg"
    >
      <div className="flex items-start justify-between gap-3 border-b border-border p-3">
        <div>
          <h2 className="text-sm font-semibold">Version history</h2>
          <p className="mt-1 text-xs text-muted-foreground">
            Current rev {cloudRevision ?? 'unknown'}
          </p>
          {cloudShareRole === 'viewer' && (
            <p className="mt-1 text-xs text-muted-foreground">View-only shared file</p>
          )}
        </div>
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label="Close version history"
          onClick={() => setOpen(false)}
        >
          <X size={14} />
        </Button>
      </div>

      <div className="min-h-0 flex-1 overflow-auto p-3">
        {error && <p className="mb-3 text-xs text-destructive">{error}</p>}
        {loading ? (
          <p className="text-xs text-muted-foreground">Loading versions...</p>
        ) : versions.length === 0 ? (
          <p className="rounded-md border border-border bg-background p-3 text-xs text-muted-foreground">
            No saved versions yet.
          </p>
        ) : (
          <div className="space-y-2">
            {versions.map((version) => {
              const title = version.label ?? version.source;
              return (
                <div
                  key={version.id}
                  className="rounded-md border border-border bg-background p-3 text-xs"
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0">
                      <p className="font-medium">rev {version.revision}</p>
                      <p className="mt-1 truncate text-muted-foreground">{title}</p>
                      <p className="mt-1 text-muted-foreground">
                        {formatVersionDate(version.createdAt)}
                      </p>
                      <p className="mt-1 text-muted-foreground">
                        {formatBytes(version.sizeBytes)} · actor {version.actorId ?? 'unknown'}
                      </p>
                      {editingId === version.id && (
                        <div className="mt-2 flex gap-2">
                          <input
                            aria-label={`Label rev ${version.revision}`}
                            value={labelDraft}
                            onChange={(event) => setLabelDraft(event.currentTarget.value)}
                            className="min-w-0 flex-1 rounded-md border border-border bg-card px-2 py-1 text-xs outline-none"
                          />
                          <Button
                            variant="outline"
                            size="icon-sm"
                            aria-label={`Save label rev ${version.revision}`}
                            onClick={() => void saveLabel(version)}
                            disabled={savingLabelId !== null}
                          >
                            <Save size={12} />
                          </Button>
                        </div>
                      )}
                    </div>
                    {canManageVersions && (
                      <div className="flex shrink-0 flex-col gap-2">
                        <Button
                          variant="ghost"
                          size="icon-sm"
                          aria-label={`Edit label rev ${version.revision}`}
                          onClick={() => {
                            setEditingId(version.id);
                            setLabelDraft(version.label ?? '');
                          }}
                        >
                          <Pencil size={13} />
                        </Button>
                        <Button
                          variant="outline"
                          size="sm"
                          aria-label={`Restore rev ${version.revision}`}
                          onClick={() => void restoreVersion(version)}
                          disabled={restoringId !== null || version.revision === cloudRevision}
                        >
                          <RotateCcw size={13} />
                          {restoringId === version.id ? 'Restoring' : 'Restore'}
                        </Button>
                      </div>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        )}

        <div className="mt-4 border-t border-border pt-3">
          <div className="mb-2 flex items-center justify-between gap-2">
            <div className="flex items-center gap-2 text-xs font-medium">
              <Activity size={13} />
              Activity
            </div>
            <label className="flex items-center gap-1 text-xs text-muted-foreground">
              <span className="sr-only">Filter activity</span>
              <select
                aria-label="Filter activity"
                value={activityFilter}
                onChange={(event) => setActivityFilter(event.currentTarget.value as CloudActivityEventType | 'all')}
                className="rounded-md border border-border bg-background px-2 py-1 text-xs outline-none"
              >
                {ACTIVITY_FILTERS.map((filter) => (
                  <option key={filter.value} value={filter.value}>
                    {filter.label}
                  </option>
                ))}
              </select>
            </label>
          </div>
          {activity.length === 0 ? (
            <p className="rounded-md border border-border bg-background p-3 text-xs text-muted-foreground">
              No activity yet.
            </p>
          ) : (
            <div className="space-y-2">
              {activity.map((item) => (
                <div key={item.id} className="rounded-md border border-border bg-background p-2 text-xs">
                  <div className="flex items-start justify-between gap-2">
                    <p className="font-medium">{formatActivityType(item.type)}</p>
                    <p className="shrink-0 text-muted-foreground">
                      {item.actorId ? 'actor' : 'system'}
                    </p>
                  </div>
                  <p className="mt-1 text-muted-foreground">{formatActivitySummary(item)}</p>
                  <p className="mt-1 text-muted-foreground">{formatVersionDate(item.createdAt)}</p>
                </div>
              ))}
              {activityCursor && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => void loadMoreActivity()}
                  disabled={loadingMoreActivity}
                  className="w-full"
                >
                  <ChevronDown size={13} />
                  {loadingMoreActivity ? 'Loading activity...' : 'Load more activity'}
                </Button>
              )}
            </div>
          )}
        </div>
      </div>
    </aside>
  );
}
