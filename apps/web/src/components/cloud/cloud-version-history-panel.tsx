import { useEffect, useState } from 'react';
import { Activity, ChevronDown, Pencil, RotateCcw, Save, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
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

const ACTIVITY_FILTERS: Array<{ value: CloudActivityEventType | 'all'; labelKey: string }> = [
  { value: 'all', labelKey: 'versionHistory.activityFilter.all' },
  { value: 'file_saved', labelKey: 'versionHistory.activityFilter.saves' },
  { value: 'file_restored', labelKey: 'versionHistory.activityFilter.restores' },
  { value: 'file_shared', labelKey: 'versionHistory.activityFilter.shares' },
  { value: 'codegen_created', labelKey: 'versionHistory.activityFilter.codegen' },
  { value: 'codegen_job_created', labelKey: 'versionHistory.activityFilter.codegenJobs' },
  { value: 'version_labeled', labelKey: 'versionHistory.activityFilter.labels' },
];

function getActivityTypeKey(type: CloudActivityEventType) {
  return `versionHistory.activityType.${type}`;
}

function formatActivitySummary(
  item: CloudActivityEvent,
  t: ReturnType<typeof useTranslation>['t'],
) {
  const metadata = item.metadata ?? {};
  const revision =
    typeof metadata.revision === 'number'
      ? t('versionHistory.revisionValue', { revision: metadata.revision })
      : null;
  const label = typeof metadata.label === 'string' ? metadata.label : null;
  const framework = typeof metadata.framework === 'string' ? metadata.framework : null;
  const parts = [revision, label, framework].filter(Boolean);
  return parts.length > 0 ? parts.join(' · ') : t('versionHistory.noExtraDetails');
}

export function CloudVersionHistoryPanel() {
  const { t } = useTranslation();
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
        if (!cancelled) {
          setError(err instanceof Error ? err.message : t('versionHistory.errorLoad'));
        }
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
      setError(err instanceof Error ? err.message : t('versionHistory.errorRestore'));
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
      setError(err instanceof Error ? err.message : t('versionHistory.errorLabel'));
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
      setError(err instanceof Error ? err.message : t('versionHistory.errorLoadMoreActivity'));
    } finally {
      setLoadingMoreActivity(false);
    }
  };

  return (
    <aside
      role="complementary"
      aria-label={t('versionHistory.title')}
      className="absolute right-3 top-3 z-30 flex max-h-[calc(100%-1.5rem)] w-80 flex-col rounded-lg border border-border bg-card text-card-foreground shadow-lg"
    >
      <div className="flex items-start justify-between gap-3 border-b border-border p-3">
        <div>
          <h2 className="text-sm font-semibold">{t('versionHistory.title')}</h2>
          <p className="mt-1 text-xs text-muted-foreground">
            {t('versionHistory.currentRevision', {
              revision: cloudRevision ?? t('versionHistory.unknownRevision'),
            })}
          </p>
          {cloudShareRole === 'viewer' && (
            <p className="mt-1 text-xs text-muted-foreground">
              {t('versionHistory.viewOnlySharedFile')}
            </p>
          )}
        </div>
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={t('versionHistory.close')}
          onClick={() => setOpen(false)}
        >
          <X size={14} />
        </Button>
      </div>

      <div className="min-h-0 flex-1 overflow-auto p-3">
        {error && <p className="mb-3 text-xs text-destructive">{error}</p>}
        {loading ? (
          <p className="text-xs text-muted-foreground">{t('versionHistory.loading')}</p>
        ) : versions.length === 0 ? (
          <p className="rounded-md border border-border bg-background p-3 text-xs text-muted-foreground">
            {t('versionHistory.empty')}
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
                      <p className="font-medium">
                        {t('versionHistory.revisionValue', { revision: version.revision })}
                      </p>
                      <p className="mt-1 truncate text-muted-foreground">{title}</p>
                      <p className="mt-1 text-muted-foreground">
                        {formatVersionDate(version.createdAt)}
                      </p>
                      <p className="mt-1 text-muted-foreground">
                        {t('versionHistory.versionMeta', {
                          size: formatBytes(version.sizeBytes),
                          actor: version.actorId ?? t('versionHistory.unknownActor'),
                        })}
                      </p>
                      {editingId === version.id && (
                        <div className="mt-2 flex gap-2">
                          <input
                            aria-label={t('versionHistory.labelRevision', {
                              revision: version.revision,
                            })}
                            value={labelDraft}
                            onChange={(event) => setLabelDraft(event.currentTarget.value)}
                            className="min-w-0 flex-1 rounded-md border border-border bg-card px-2 py-1 text-xs outline-none"
                          />
                          <Button
                            variant="outline"
                            size="icon-sm"
                            aria-label={t('versionHistory.saveLabelRevision', {
                              revision: version.revision,
                            })}
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
                          aria-label={t('versionHistory.editLabelRevision', {
                            revision: version.revision,
                          })}
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
                          aria-label={t('versionHistory.restoreRevision', {
                            revision: version.revision,
                          })}
                          onClick={() => void restoreVersion(version)}
                          disabled={restoringId !== null || version.revision === cloudRevision}
                        >
                          <RotateCcw size={13} />
                          {restoringId === version.id
                            ? t('versionHistory.restoring')
                            : t('versionHistory.restore')}
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
              {t('versionHistory.activity')}
            </div>
            <label className="flex items-center gap-1 text-xs text-muted-foreground">
              <span className="sr-only">{t('versionHistory.filterActivity')}</span>
              <select
                aria-label={t('versionHistory.filterActivity')}
                value={activityFilter}
                onChange={(event) => setActivityFilter(event.currentTarget.value as CloudActivityEventType | 'all')}
                className="rounded-md border border-border bg-background px-2 py-1 text-xs outline-none"
              >
                {ACTIVITY_FILTERS.map((filter) => (
                  <option key={filter.value} value={filter.value}>
                    {t(filter.labelKey)}
                  </option>
                ))}
              </select>
            </label>
          </div>
          {activity.length === 0 ? (
            <p className="rounded-md border border-border bg-background p-3 text-xs text-muted-foreground">
              {t('versionHistory.noActivity')}
            </p>
          ) : (
            <div className="space-y-2">
              {activity.map((item) => (
                <div key={item.id} className="rounded-md border border-border bg-background p-2 text-xs">
                  <div className="flex items-start justify-between gap-2">
                    <p className="font-medium">{t(getActivityTypeKey(item.type))}</p>
                    <p className="shrink-0 text-muted-foreground">
                      {item.actorId ? t('versionHistory.actor') : t('versionHistory.systemActor')}
                    </p>
                  </div>
                  <p className="mt-1 text-muted-foreground">{formatActivitySummary(item, t)}</p>
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
                  {loadingMoreActivity
                    ? t('versionHistory.loadingActivity')
                    : t('versionHistory.loadMoreActivity')}
                </Button>
              )}
            </div>
          )}
        </div>
      </div>
    </aside>
  );
}
