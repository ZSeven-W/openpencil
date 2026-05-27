import { useEffect, useState } from 'react';
import { AlertTriangle, Loader2, ServerCog } from 'lucide-react';
import { useNavigate } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet';
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group';
import { WorkbenchShell } from '@/components/workbench/workbench-shell';
import { cn } from '@/lib/utils';
import { useCodegenJobStore } from '@/stores/codegen-job-store';
import { TaskDetail } from './task-detail';
import { NotificationRecords } from './task-notifications';
import { TaskTable } from './task-table';
import type { CloudCodegenJob, CodegenJobStatus, TaskNotification } from '@/types/cloud';
import {
  FILTERS,
  isCancellable,
  NOTIFICATION_PAGE_SIZE,
  TASK_LIST_LIMIT,
} from './task-center-utils';

interface TaskCenterProps {
  fileId?: string;
  jobId?: string;
  view?: 'tasks' | 'notifications';
}

export function TaskCenter({ fileId, jobId, view = 'tasks' }: TaskCenterProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const jobs = useCodegenJobStore((s) => s.jobs);
  const jobPage = useCodegenJobStore((s) => s.jobPage);
  const notifications = useCodegenJobStore((s) => s.notifications);
  const notificationPageMeta = useCodegenJobStore((s) => s.notificationPage);
  const loading = useCodegenJobStore((s) => s.loading);
  const error = useCodegenJobStore((s) => s.error);
  const refreshJobs = useCodegenJobStore((s) => s.refreshJobs);
  const refreshNotifications = useCodegenJobStore((s) => s.refreshNotifications);
  const workerOverview = useCodegenJobStore((s) => s.workerOverview);
  const refreshWorkerOverview = useCodegenJobStore((s) => s.refreshWorkerOverview);
  const cancelJob = useCodegenJobStore((s) => s.cancelJob);
  const deleteJob = useCodegenJobStore((s) => s.deleteJob);
  const batchCancelJobs = useCodegenJobStore((s) => s.batchCancelJobs);
  const retryJob = useCodegenJobStore((s) => s.retryJob);
  const updateJobPriority = useCodegenJobStore((s) => s.updateJobPriority);
  const markNotificationRead = useCodegenJobStore((s) => s.markNotificationRead);
  const deleteNotification = useCodegenJobStore((s) => s.deleteNotification);
  const [filter, setFilter] = useState<'all' | CodegenJobStatus>('all');
  const [selectedJobIds, setSelectedJobIds] = useState<Set<string>>(() => new Set());
  const [priorityDrafts, setPriorityDrafts] = useState<Record<string, string>>({});
  const [taskPage, setTaskPage] = useState(1);
  const [notificationPage, setNotificationPage] = useState(1);

  useEffect(() => {
    const status = filter === 'all' ? undefined : filter;
    void refreshJobs(
      fileId
        ? { fileId, status, limit: TASK_LIST_LIMIT, offset: (taskPage - 1) * TASK_LIST_LIMIT }
        : { status, limit: TASK_LIST_LIMIT, offset: (taskPage - 1) * TASK_LIST_LIMIT },
    );
    void refreshWorkerOverview({
      summary: true,
      workerLimit: 5,
      providerLimit: 1,
      failedLimit: 1,
    });
  }, [fileId, filter, refreshJobs, refreshWorkerOverview, taskPage]);

  useEffect(() => {
    void refreshNotifications({
      fileId,
      limit: NOTIFICATION_PAGE_SIZE,
      offset: (notificationPage - 1) * NOTIFICATION_PAGE_SIZE,
    });
  }, [fileId, notificationPage, refreshNotifications]);

  useEffect(() => setTaskPage(1), [filter, fileId]);
  useEffect(() => setNotificationPage(1), [fileId, view]);

  const visibleJobs = jobs;
  const visibleNotifications = notifications;
  const currentTaskPage = Math.floor(jobPage.offset / jobPage.limit) + 1;
  const taskTotal = jobPage.total;
  const pagedJobs = visibleJobs;
  const notificationPageCount = Math.max(
    1,
    Math.ceil(notificationPageMeta.total / NOTIFICATION_PAGE_SIZE),
  );
  const currentNotificationPage = Math.min(notificationPage, notificationPageCount);
  const pagedNotifications = visibleNotifications;
  const unreadCount = visibleNotifications.filter((item) => !item.readAt).length;
  const hasPendingJobs = jobs.some((job) => job.status === 'pending');
  const activeWorkerCount = workerOverview?.workers.filter((worker) => worker.active).length ?? 0;
  const showNoWorkerWarning = Boolean(workerOverview && hasPendingJobs && activeWorkerCount === 0);
  const selectedCancellableJobIds = visibleJobs
    .filter((job) => selectedJobIds.has(job.id) && isCancellable(job))
    .map((job) => job.id);
  const allPagedSelected =
    pagedJobs.length > 0 && pagedJobs.every((job) => selectedJobIds.has(job.id));
  const somePagedSelected = pagedJobs.some((job) => selectedJobIds.has(job.id));

  const openGeneration = (job: CloudCodegenJob) => {
    if (job.fileId) void navigate({ to: '/editor/$fileId', params: { fileId: job.fileId } });
  };

  const openJobDetail = (selectedJobId: string) => {
    const search = {
      ...(fileId ? { fileId } : {}),
      jobId: selectedJobId,
      ...(view === 'notifications' ? { view: 'notifications' as const } : {}),
    };
    void navigate({
      to: '/tasks',
      search,
    });
  };

  const closeJobDetail = () => {
    const search = {
      ...(fileId ? { fileId } : {}),
      ...(view === 'notifications' ? { view: 'notifications' as const } : {}),
    };
    void navigate({
      to: '/tasks',
      search,
    });
  };

  const refresh = () => {
    const status = filter === 'all' ? undefined : filter;
    void refreshJobs(
      fileId
        ? { fileId, status, limit: TASK_LIST_LIMIT, offset: (taskPage - 1) * TASK_LIST_LIMIT, force: true }
        : { status, limit: TASK_LIST_LIMIT, offset: (taskPage - 1) * TASK_LIST_LIMIT, force: true },
    );
    void refreshNotifications({
      fileId,
      limit: NOTIFICATION_PAGE_SIZE,
      offset: (notificationPage - 1) * NOTIFICATION_PAGE_SIZE,
      force: true,
    });
    void refreshWorkerOverview({
      summary: true,
      workerLimit: 5,
      providerLimit: 1,
      failedLimit: 1,
      force: true,
    });
  };

  const toggleSelected = (selectedJobId: string) => {
    setSelectedJobIds((current) => {
      const next = new Set(current);
      if (next.has(selectedJobId)) next.delete(selectedJobId);
      else next.add(selectedJobId);
      return next;
    });
  };

  const toggleSelectedPage = () => {
    setSelectedJobIds((current) => {
      const next = new Set(current);
      if (allPagedSelected) {
        for (const job of pagedJobs) next.delete(job.id);
      } else {
        for (const job of pagedJobs) next.add(job.id);
      }
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

  const openNotificationTarget = (item: TaskNotification) => {
    void markNotificationRead(item.id);
    if (view === 'notifications' && !item.jobId && item.fileId) {
      void navigate({ to: '/editor/$fileId', params: { fileId: item.fileId } });
    }
  };

  return (
    <WorkbenchShell
      active={view === 'notifications' ? 'notifications' : 'tasks'}
      title={view === 'notifications' ? t('tasks.notificationRecords') : t('tasks.title')}
      description={fileId ? t('tasks.fileSubtitle') : t('tasks.subtitle')}
      notificationCount={unreadCount}
      toolbar={
        <div className="mb-2 flex min-h-8 flex-wrap items-center justify-between gap-2 border-b border-border pb-2">
          {view === 'tasks' ? (
            <ToggleGroup
              type="single"
              value={filter}
              onValueChange={(value) => value && setFilter(value as typeof filter)}
              variant="outline"
              size="sm"
              className="flex-wrap"
            >
              {FILTERS.map((item) => {
                const Icon = item.icon;
                return (
                  <ToggleGroupItem key={item.id} value={item.id} className="h-7 text-xs">
                    <Icon className={cn(item.id === 'running' && 'animate-spin')} />
                    {t(`tasks.filter.${item.id}`)}
                  </ToggleGroupItem>
                );
              })}
            </ToggleGroup>
          ) : (
            <div className="min-h-7" />
          )}
          <div className="flex items-center gap-2">
            {!fileId && (
              <Button
                variant="outline"
                size="sm"
                className="h-7 px-2 text-xs shadow-none"
                onClick={() => void navigate({ to: '/tasks/workers' })}
              >
                <ServerCog data-icon="inline-start" />
                {t('tasks.workerManagement')}
              </Button>
            )}
            <Button
              variant="outline"
              size="sm"
              className="h-7 px-2 text-xs shadow-none"
              onClick={refresh}
              disabled={loading}
            >
              {loading && <Loader2 data-icon="inline-start" className="animate-spin" />}
              {t('tasks.refresh')}
            </Button>
          </div>
        </div>
      }
      contentClassName="w-full"
    >
      <main className="w-full">
        {view === 'notifications' ? (
          <NotificationRecords
            notifications={pagedNotifications}
            total={notificationPageMeta.total}
            page={currentNotificationPage}
            onPageChange={setNotificationPage}
            onOpen={openNotificationTarget}
            onDelete={(id) => void deleteNotification(id)}
            t={t}
          />
        ) : (
          <>
            {selectedJobIds.size > 0 && (
              <div className="mb-2 flex flex-wrap items-center justify-between gap-2 rounded border border-border bg-card px-2.5 py-1.5 text-xs">
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
              <Alert variant="destructive" className="mb-2">
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}
            {showNoWorkerWarning && (
              <Alert className="mb-2 rounded border-border bg-card">
                <AlertTriangle />
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div>
                    <AlertTitle>{t('tasks.workerUnavailable.title')}</AlertTitle>
                    <AlertDescription>
                      {t('tasks.workerUnavailable.description')}
                    </AlertDescription>
                  </div>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => void navigate({ to: '/tasks/workers' })}
                  >
                    <ServerCog data-icon="inline-start" />
                    {t('tasks.workerUnavailable.action')}
                  </Button>
                </div>
              </Alert>
            )}
            {visibleJobs.length === 0 ? (
              <div className="rounded border border-border bg-card p-8 text-center">
                <p className="text-sm font-medium">{t('tasks.empty.title')}</p>
                <p className="mt-1 text-sm text-muted-foreground">
                  {t('tasks.empty.description')}
                </p>
              </div>
            ) : (
              <TaskTable
                jobs={pagedJobs}
                total={taskTotal}
                page={currentTaskPage}
                selectedJobIds={selectedJobIds}
                allSelected={allPagedSelected}
                someSelected={somePagedSelected}
                priorityDrafts={priorityDrafts}
                onToggleSelected={toggleSelected}
                onToggleSelectedPage={toggleSelectedPage}
                onPageChange={setTaskPage}
                onCancel={(id) => void cancelJob(id)}
                onDelete={(id) => void deleteJob(id)}
                onRetry={(id) => void retryJob(id)}
                onOpenFile={openGeneration}
                onOpenDetail={openJobDetail}
                onPriorityChange={(id, value) =>
                  setPriorityDrafts((current) => ({ ...current, [id]: value }))
                }
                onSavePriority={(job) => void savePriority(job)}
                t={t}
              />
            )}
          </>
        )}
      </main>

      <Sheet open={Boolean(jobId)} onOpenChange={(open) => !open && closeJobDetail()}>
        <SheetContent className="w-full gap-0 overflow-y-auto p-0 sm:max-w-4xl" side="right">
          <SheetHeader className="sr-only">
            <SheetTitle>{t('tasks.detailTitle')}</SheetTitle>
            <SheetDescription>{t('tasks.detailDescription')}</SheetDescription>
          </SheetHeader>
          {jobId && (
            <TaskDetail jobId={jobId} embedded onClose={closeJobDetail} />
          )}
        </SheetContent>
      </Sheet>
    </WorkbenchShell>
  );
}
