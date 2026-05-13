import { useEffect, useMemo, useState } from 'react';
import {
  AlertTriangle,
  Bell,
  CheckCircle2,
  Clock3,
  FileCode2,
  Loader2,
  RotateCcw,
  ServerCog,
  XCircle,
} from 'lucide-react';
import { useNavigate } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { useCodegenJobStore } from '@/stores/codegen-job-store';
import type {
  CloudCodegenJob,
  CodegenJobStatus,
  TaskNotification,
} from '@/types/cloud';

const FILTERS: Array<{ id: 'all' | CodegenJobStatus; icon: typeof Clock3 }> = [
  { id: 'all', icon: Clock3 },
  { id: 'pending', icon: Clock3 },
  { id: 'running', icon: Loader2 },
  { id: 'succeeded', icon: CheckCircle2 },
  { id: 'failed', icon: XCircle },
  { id: 'canceled', icon: XCircle },
];

function getVisibleStatus(job: CloudCodegenJob, t: ReturnType<typeof useTranslation>['t']) {
  if (job.deadLetteredAt) return t('tasks.status.deadLettered');
  if (job.status === 'pending' && job.nextRunAt) {
    const nextRun = new Date(job.nextRunAt).getTime();
    if (Number.isFinite(nextRun) && nextRun > Date.now()) {
      return t('tasks.status.retryScheduled');
    }
  }
  return t(`tasks.status.${job.status}`);
}

function statusIcon(status: CodegenJobStatus) {
  if (status === 'succeeded') return <CheckCircle2 className="h-4 w-4 text-primary" />;
  if (status === 'failed') return <XCircle className="h-4 w-4 text-destructive" />;
  if (status === 'running') return <Loader2 className="h-4 w-4 animate-spin text-primary" />;
  return <Clock3 className="h-4 w-4 text-muted-foreground" />;
}

function formatTime(value: string): string {
  try {
    return new Intl.DateTimeFormat(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    }).format(new Date(value));
  } catch {
    return value;
  }
}

function renderNotificationCopy(
  notification: TaskNotification,
  t: ReturnType<typeof useTranslation>['t'],
) {
  if (notification.kind === 'codegen_job_succeeded') {
    return {
      title: t('tasks.notification.codegenSucceeded.title'),
      message: t('tasks.notification.codegenSucceeded.message', {
        framework: notification.message ?? '',
      }),
    };
  }
  if (notification.kind === 'codegen_job_failed') {
    return {
      title: t('tasks.notification.codegenFailed.title'),
      message: notification.message ?? t('tasks.notification.codegenFailed.message'),
    };
  }
  return {
    title: t('tasks.notification.codegenCanceled.title'),
    message: t('tasks.notification.codegenCanceled.message', {
      framework: notification.message ?? '',
    }),
  };
}

interface TaskCenterProps {
  fileId?: string;
}

function isCancellable(job: CloudCodegenJob): boolean {
  return job.status === 'pending' || job.status === 'running';
}

function describeJobKind(job: CloudCodegenJob, t: ReturnType<typeof useTranslation>['t']) {
  return t(`tasks.jobKind.${job.jobKind ?? 'full_generation'}`);
}

export function TaskCenter({ fileId }: TaskCenterProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const jobs = useCodegenJobStore((s) => s.jobs);
  const notifications = useCodegenJobStore((s) => s.notifications);
  const loading = useCodegenJobStore((s) => s.loading);
  const error = useCodegenJobStore((s) => s.error);
  const refreshJobs = useCodegenJobStore((s) => s.refreshJobs);
  const refreshNotifications = useCodegenJobStore((s) => s.refreshNotifications);
  const workerOverview = useCodegenJobStore((s) => s.workerOverview);
  const refreshWorkerOverview = useCodegenJobStore((s) => s.refreshWorkerOverview);
  const cancelJob = useCodegenJobStore((s) => s.cancelJob);
  const batchCancelJobs = useCodegenJobStore((s) => s.batchCancelJobs);
  const retryJob = useCodegenJobStore((s) => s.retryJob);
  const updateJobPriority = useCodegenJobStore((s) => s.updateJobPriority);
  const markNotificationRead = useCodegenJobStore((s) => s.markNotificationRead);
  const [filter, setFilter] = useState<'all' | CodegenJobStatus>('all');
  const [selectedJobIds, setSelectedJobIds] = useState<Set<string>>(() => new Set());
  const [priorityDrafts, setPriorityDrafts] = useState<Record<string, string>>({});

  useEffect(() => {
    void refreshJobs(fileId ? { fileId } : undefined);
    void refreshNotifications();
    void refreshWorkerOverview({ workerLimit: 5, providerLimit: 1, failedLimit: 1 });
  }, [fileId, refreshJobs, refreshNotifications, refreshWorkerOverview]);

  const filteredJobs = useMemo(
    () => (filter === 'all' ? jobs : jobs.filter((job) => job.status === filter)),
    [filter, jobs],
  );
  const visibleNotifications = useMemo(
    () => (fileId ? notifications.filter((item) => item.fileId === fileId) : notifications),
    [fileId, notifications],
  );
  const unreadCount = visibleNotifications.filter((item) => !item.readAt).length;
  const hasPendingJobs = jobs.some((job) => job.status === 'pending');
  const activeWorkerCount = workerOverview?.workers.filter((worker) => worker.active).length ?? 0;
  const showNoWorkerWarning = Boolean(workerOverview && hasPendingJobs && activeWorkerCount === 0);
  const selectedCancellableJobIds = filteredJobs
    .filter((job) => selectedJobIds.has(job.id) && isCancellable(job))
    .map((job) => job.id);

  const openGeneration = (job: CloudCodegenJob) => {
    if (job.fileId) void navigate({ to: '/editor/$fileId', params: { fileId: job.fileId } });
  };

  const openJobDetail = (jobId: string) => {
    void navigate({ to: '/tasks/$jobId', params: { jobId } });
  };

  const refresh = () => {
    void refreshJobs(fileId ? { fileId } : undefined);
    void refreshNotifications();
    void refreshWorkerOverview({ workerLimit: 5, providerLimit: 1, failedLimit: 1 });
  };

  const toggleSelected = (jobId: string) => {
    setSelectedJobIds((current) => {
      const next = new Set(current);
      if (next.has(jobId)) next.delete(jobId);
      else next.add(jobId);
      return next;
    });
  };

  const cancelSelected = async () => {
    await batchCancelJobs(selectedCancellableJobIds);
    setSelectedJobIds((current) => {
      const next = new Set(current);
      for (const id of selectedCancellableJobIds) next.delete(id);
      return next;
    });
  };

  const savePriority = async (job: CloudCodegenJob) => {
    const value = Number(priorityDrafts[job.id] ?? job.priority);
    if (!Number.isInteger(value)) return;
    await updateJobPriority(job.id, value);
  };

  return (
    <div className="min-h-screen bg-background text-foreground">
      <header className="border-b border-border bg-card px-5 py-4">
        <div className="mx-auto flex max-w-6xl items-center justify-between gap-3">
          <div>
            <h1 className="text-lg font-semibold">{t('tasks.title')}</h1>
            <p className="text-sm text-muted-foreground">
              {fileId ? t('tasks.fileSubtitle') : t('tasks.subtitle')}
            </p>
          </div>
          <div className="flex items-center gap-2">
            {!fileId && (
              <Button
                variant="outline"
                size="sm"
                onClick={() => void navigate({ to: '/tasks/workers' })}
              >
                <ServerCog className="h-4 w-4" />
                {t('tasks.workerManagement')}
              </Button>
            )}
            <Button variant="outline" size="sm" onClick={refresh} disabled={loading}>
              {loading && <Loader2 className="h-4 w-4 animate-spin" />}
              {t('tasks.refresh')}
            </Button>
          </div>
        </div>
      </header>

      <main className="mx-auto grid max-w-6xl gap-4 px-5 py-5 lg:grid-cols-[220px_minmax(0,1fr)_280px]">
        <aside className="space-y-1">
          {FILTERS.map((item) => {
            const Icon = item.icon;
            return (
              <button
                key={item.id}
                type="button"
                className={cn(
                  'flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-sm transition-colors hover:bg-accent',
                  filter === item.id && 'bg-accent text-accent-foreground',
                )}
                onClick={() => setFilter(item.id)}
              >
                <Icon className={cn('h-4 w-4', item.id === 'running' && 'animate-spin')} />
                {t(`tasks.filter.${item.id}`)}
              </button>
            );
          })}
        </aside>

        <section className="min-w-0">
          {selectedJobIds.size > 0 && (
            <div className="mb-3 flex flex-wrap items-center justify-between gap-2 rounded-md border border-border bg-card px-3 py-2 text-sm">
              <span className="text-muted-foreground">
                {t('tasks.selectedCount', { count: selectedJobIds.size })}
              </span>
              <div className="flex items-center gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={cancelSelected}
                  disabled={selectedCancellableJobIds.length === 0}
                >
                  {t('tasks.batchCancel')}
                </Button>
                <Button variant="ghost" size="sm" onClick={() => setSelectedJobIds(new Set())}>
                  {t('tasks.clearSelection')}
                </Button>
              </div>
            </div>
          )}
          {error && (
            <div className="mb-3 rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">
              {error}
            </div>
          )}
          {showNoWorkerWarning && (
            <div className="mb-3 flex flex-wrap items-start justify-between gap-3 rounded-md border border-border bg-muted/50 px-3 py-3 text-sm text-foreground">
              <div className="flex min-w-0 gap-2">
                <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
                <div>
                  <p className="font-medium">{t('tasks.workerUnavailable.title')}</p>
                  <p className="mt-1 text-xs text-muted-foreground">
                    {t('tasks.workerUnavailable.description')}
                  </p>
                </div>
              </div>
              <Button
                variant="outline"
                size="sm"
                onClick={() => void navigate({ to: '/tasks/workers' })}
              >
                <ServerCog className="h-4 w-4" />
                {t('tasks.workerUnavailable.action')}
              </Button>
            </div>
          )}
          {filteredJobs.length === 0 ? (
            <div className="rounded-md border border-border bg-card p-10 text-center">
              <p className="text-sm font-medium">{t('tasks.empty.title')}</p>
              <p className="mt-1 text-sm text-muted-foreground">{t('tasks.empty.description')}</p>
            </div>
          ) : (
            <div className="space-y-2">
              {filteredJobs.map((job) => (
                <article
                  key={job.id}
                  className="rounded-md border border-border bg-card px-4 py-3"
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="flex min-w-0 items-start gap-3">
                      <input
                        type="checkbox"
                        className="mt-0.5 h-4 w-4 rounded border-border"
                        checked={selectedJobIds.has(job.id)}
                        onChange={() => toggleSelected(job.id)}
                        aria-label={t('tasks.selectJob', { id: job.id })}
                      />
                      <div className="min-w-0">
                        <div className="flex items-center gap-2">
                          {statusIcon(job.status)}
                          <h2 className="truncate text-sm font-semibold">
                            {t('tasks.codegenTitle', { framework: job.framework })}
                          </h2>
                          <span className="rounded border border-border bg-background px-1.5 py-0.5 text-[10px] text-muted-foreground">
                            {describeJobKind(job, t)}
                          </span>
                        </div>
                        <p className="mt-1 text-xs text-muted-foreground">
                          {formatTime(job.createdAt)} · {getVisibleStatus(job, t)}
                          {job.attempts > 0 &&
                            ` · ${t('tasks.attempts', {
                              attempts: job.attempts,
                              maxAttempts: job.maxAttempts,
                            })}`}
                        </p>
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      {job.status === 'failed' && (
                        <Button variant="outline" size="sm" onClick={() => void retryJob(job.id)}>
                          <RotateCcw className="h-4 w-4" />
                          {t('tasks.retry')}
                        </Button>
                      )}
                      {(job.status === 'pending' || job.status === 'running') && (
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => void cancelJob(job.id)}
                        >
                          {t('tasks.cancel')}
                        </Button>
                      )}
                      {job.status === 'pending' && (
                        <div className="flex items-center gap-1">
                          <label className="sr-only" htmlFor={`priority-${job.id}`}>
                            {t('tasks.priority')}
                          </label>
                          <input
                            id={`priority-${job.id}`}
                            type="number"
                            className="h-8 w-20 rounded-md border border-input bg-background px-2 text-xs"
                            value={priorityDrafts[job.id] ?? String(job.priority)}
                            onChange={(event) =>
                              setPriorityDrafts((current) => ({
                                ...current,
                                [job.id]: event.target.value,
                              }))
                            }
                          />
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => void savePriority(job)}
                          >
                            {t('tasks.savePriority')}
                          </Button>
                        </div>
                      )}
                      <Button variant="ghost" size="sm" onClick={() => openGeneration(job)}>
                        <FileCode2 className="h-4 w-4" />
                        {t('tasks.openFile')}
                      </Button>
                      <Button variant="ghost" size="sm" onClick={() => openJobDetail(job.id)}>
                        {t('tasks.details')}
                      </Button>
                    </div>
                  </div>
                  <div className="mt-3 h-1.5 overflow-hidden rounded-full bg-muted">
                    <div
                      className="h-full rounded-full bg-primary transition-all"
                      style={{ width: `${job.progress}%` }}
                    />
                  </div>
                  {job.error && <p className="mt-2 text-xs text-destructive">{job.error}</p>}
                </article>
              ))}
            </div>
          )}
        </section>

        <aside className="rounded-md border border-border bg-card p-4">
          <div className="mb-3 flex items-center justify-between">
            <div className="flex items-center gap-2 text-sm font-semibold">
              <Bell className="h-4 w-4" />
              {t('tasks.notifications')}
            </div>
            {unreadCount > 0 && (
              <span className="rounded-full bg-primary px-2 py-0.5 text-[10px] text-primary-foreground">
                {unreadCount}
              </span>
            )}
          </div>
          <div className="space-y-2">
            {visibleNotifications.length === 0 ? (
              <p className="text-xs text-muted-foreground">{t('tasks.notificationsEmpty')}</p>
            ) : (
              visibleNotifications.map((item) => {
                const copy = renderNotificationCopy(item, t);
                return (
                  <button
                    key={item.id}
                    type="button"
                    className={cn(
                      'w-full rounded-md border border-border px-3 py-2 text-left text-xs',
                      item.readAt ? 'bg-background text-muted-foreground' : 'bg-muted/40',
                    )}
                    onClick={() => {
                      void markNotificationRead(item.id);
                      if (item.jobId) {
                        openJobDetail(item.jobId);
                      } else if (item.fileId) {
                        void navigate({
                          to: '/editor/$fileId',
                          params: { fileId: item.fileId },
                        });
                      }
                    }}
                  >
                    <p className="font-medium text-foreground">{copy.title}</p>
                    {copy.message && <p className="mt-1 text-muted-foreground">{copy.message}</p>}
                  </button>
                );
              })
            )}
          </div>
        </aside>
      </main>
    </div>
  );
}
