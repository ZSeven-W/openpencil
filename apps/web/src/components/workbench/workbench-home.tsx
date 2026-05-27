import { useEffect, useMemo, useState } from 'react';
import { useNavigate } from '@tanstack/react-router';
import {
  Bell,
  CheckCircle2,
  Clock3,
  Cloud,
  FileJson,
  Loader2,
  Plus,
  ServerCog,
  XCircle,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from '@/components/ui/empty';
import { Skeleton } from '@/components/ui/skeleton';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { WorkbenchShell } from '@/components/workbench/workbench-shell';
import { useCloudFileStore } from '@/stores/cloud-file-store';
import { useCodegenJobStore } from '@/stores/codegen-job-store';
import { createEmptyDocument } from '@/stores/document-store';
import type { CloudCodegenJob } from '@/types/cloud';
import type { PenDocument } from '@/types/pen';
import { formatDate } from '@/components/cloud/cloud-file-library-utils';
import {
  formatProviderModel,
  formatTime,
  getTaskFileDisplay,
  getTaskPageDisplay,
} from '@/components/tasks/task-center-utils';

const HOME_FILE_LIMIT = 6;
const HOME_TASK_LIMIT = 5;
const HOME_NOTIFICATION_LIMIT = 4;

function taskStatusIcon(job: CloudCodegenJob) {
  if (job.status === 'succeeded') return <CheckCircle2 />;
  if (job.status === 'failed') return <XCircle />;
  if (job.status === 'running') return <Loader2 className="animate-spin" />;
  return <Clock3 />;
}

export function WorkbenchHome() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const files = useCloudFileStore((s) => s.files);
  const filePage = useCloudFileStore((s) => s.filePage);
  const loadingFiles = useCloudFileStore((s) => s.loading);
  const initializeLibrary = useCloudFileStore((s) => s.initializeLibrary);
  const createFile = useCloudFileStore((s) => s.createFile);
  const jobs = useCodegenJobStore((s) => s.jobs);
  const jobPage = useCodegenJobStore((s) => s.jobPage);
  const notifications = useCodegenJobStore((s) => s.notifications);
  const workerOverview = useCodegenJobStore((s) => s.workerOverview);
  const refreshJobs = useCodegenJobStore((s) => s.refreshJobs);
  const refreshNotifications = useCodegenJobStore((s) => s.refreshNotifications);
  const refreshWorkerOverview = useCodegenJobStore((s) => s.refreshWorkerOverview);
  const [creating, setCreating] = useState(false);

  useEffect(() => {
    void initializeLibrary();
    void refreshJobs({ limit: HOME_TASK_LIMIT });
    void refreshNotifications({ limit: HOME_NOTIFICATION_LIMIT });
    void refreshWorkerOverview({
      summary: true,
      workerLimit: 5,
      providerLimit: 3,
      failedLimit: 3,
    });
  }, [initializeLibrary, refreshJobs, refreshNotifications, refreshWorkerOverview]);

  const unreadCount = useMemo(
    () => notifications.filter((notification) => !notification.readAt).length,
    [notifications],
  );
  const activeWorkers = workerOverview?.workers.filter((worker) => worker.active).length ?? 0;
  const queueMetrics = workerOverview?.metrics;

  const createDesign = async () => {
    setCreating(true);
    try {
      const id = await createFile(t('common.untitled'), createEmptyDocument() as PenDocument);
      if (id) void navigate({ to: '/editor/$fileId', params: { fileId: id } });
    } finally {
      setCreating(false);
    }
  };

  return (
    <WorkbenchShell
      active="home"
      title={t('workbench.home.title')}
      description={t('workbench.home.description')}
      notificationCount={unreadCount}
      onCreateDesign={() => void createDesign()}
      createDisabled={creating}
    >
      <div className="flex min-w-0 flex-col gap-2">
        <section className="grid min-w-0 gap-2 md:grid-cols-2 xl:grid-cols-4">
          <MetricCard
            icon={Cloud}
            label={t('workbench.home.metric.files')}
            value={filePage.total}
            detail={t('workbench.home.metric.filesDetail')}
          />
          <MetricCard
            icon={Clock3}
            label={t('workbench.home.metric.tasks')}
            value={jobPage.total}
            detail={t('workbench.home.metric.tasksDetail')}
          />
          <MetricCard
            icon={Bell}
            label={t('workbench.home.metric.unread')}
            value={unreadCount}
            detail={t('workbench.home.metric.unreadDetail')}
          />
          <MetricCard
            icon={ServerCog}
            label={t('workbench.home.metric.workers')}
            value={activeWorkers}
            detail={t('workbench.home.metric.workersDetail', {
              total: workerOverview?.workers.length ?? 0,
            })}
          />
        </section>

        <section className="grid min-w-0 gap-2 xl:grid-cols-[minmax(0,1.45fr)_minmax(17rem,0.9fr)]">
          <Card className="min-w-0 rounded py-0 shadow-none">
            <CardHeader className="min-h-8 border-b border-border px-3 py-2">
              <div>
                <CardTitle className="text-sm">{t('workbench.home.recentFiles')}</CardTitle>
                <CardDescription className="text-xs">
                  {t('workbench.home.recentFilesDescription')}
                </CardDescription>
              </div>
              <CardAction>
                <Button variant="outline" size="sm" onClick={() => void navigate({ to: '/cloud' })}>
                  {t('workbench.viewAll')}
                </Button>
              </CardAction>
            </CardHeader>
            <CardContent className="p-0">
              {loadingFiles && files.length === 0 ? (
                <div className="flex flex-col gap-3 p-4">
                  <Skeleton className="h-8 w-full" />
                  <Skeleton className="h-8 w-full" />
                  <Skeleton className="h-8 w-2/3" />
                </div>
              ) : files.length === 0 ? (
                <Empty className="border-0">
                  <EmptyHeader>
                    <EmptyMedia variant="icon">
                      <FileJson />
                    </EmptyMedia>
                    <EmptyTitle>{t('cloudLibrary.empty.default.title')}</EmptyTitle>
                    <EmptyDescription>{t('cloudLibrary.empty.default.description')}</EmptyDescription>
                  </EmptyHeader>
                </Empty>
              ) : (
                <Table>
                  <TableHeader>
                    <TableRow className="hover:bg-transparent">
                      <TableHead>{t('common.name')}</TableHead>
                      <TableHead className="w-[120px]">{t('cloudLibrary.details.revision')}</TableHead>
                      <TableHead className="w-[160px]">{t('cloudLibrary.details.updated')}</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {files.slice(0, HOME_FILE_LIMIT).map((file) => (
                      <TableRow
                        key={file.id}
                        className="cursor-pointer"
                        onClick={() =>
                          void navigate({ to: '/editor/$fileId', params: { fileId: file.id } })
                        }
                      >
                        <TableCell>
                          <div className="flex min-w-0 items-center gap-2">
                            <FileJson />
                            <span className="truncate text-sm font-medium">{file.name}</span>
                          </div>
                        </TableCell>
                        <TableCell className="text-xs text-muted-foreground">
                          rev {file.revision}
                        </TableCell>
                        <TableCell className="text-xs text-muted-foreground">
                          {formatDate(file.updatedAt)}
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              )}
            </CardContent>
          </Card>

          <Card className="min-w-0 rounded py-0 shadow-none">
            <CardHeader className="min-h-8 border-b border-border px-3 py-2">
              <div>
                <CardTitle className="text-sm">{t('workbench.home.queueHealth')}</CardTitle>
                <CardDescription className="text-xs">
                  {t('workbench.home.queueHealthDescription')}
                </CardDescription>
              </div>
              <CardAction>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => void navigate({ to: '/tasks/workers' })}
                >
                  {t('workbench.open')}
                </Button>
              </CardAction>
            </CardHeader>
            <CardContent className="flex flex-col gap-2 p-3">
              <div className="grid grid-cols-3 gap-1.5">
                <MiniStat label={t('tasks.metric.running')} value={queueMetrics?.running ?? 0} />
                <MiniStat label={t('tasks.metric.pending')} value={queueMetrics?.pending ?? 0} />
                <MiniStat label={t('tasks.metric.failed')} value={queueMetrics?.failed ?? 0} />
              </div>
              <div className="rounded border border-border bg-background px-2.5 py-2">
                <div className="flex items-center justify-between gap-2 text-xs">
                  <span className="text-muted-foreground">{t('tasks.metric.failureRate')}</span>
                  <Badge variant="secondary">
                    {Math.round(Number(queueMetrics?.failureRate ?? 0) * 100)}%
                  </Badge>
                </div>
                <p className="mt-2 text-xs text-muted-foreground">
                  {t('tasks.metric.averageDuration', {
                    duration: queueMetrics?.averageDurationMs
                      ? `${(queueMetrics.averageDurationMs / 1000).toFixed(1)} s`
                      : '0 ms',
                  })}
                </p>
              </div>
            </CardContent>
          </Card>
        </section>

        <section className="grid min-w-0 gap-2 xl:grid-cols-[minmax(0,1fr)_minmax(17rem,21rem)]">
          <Card className="min-w-0 rounded py-0 shadow-none">
            <CardHeader className="min-h-8 border-b border-border px-3 py-2">
              <div>
                <CardTitle className="text-sm">{t('workbench.home.recentTasks')}</CardTitle>
                <CardDescription className="text-xs">
                  {t('workbench.home.recentTasksDescription')}
                </CardDescription>
              </div>
              <CardAction>
                <Button variant="outline" size="sm" onClick={() => void navigate({ to: '/tasks' })}>
                  {t('workbench.viewAll')}
                </Button>
              </CardAction>
            </CardHeader>
            <CardContent className="p-0">
              {jobs.length === 0 ? (
                <Empty className="border-0">
                  <EmptyHeader>
                    <EmptyMedia variant="icon">
                      <Clock3 />
                    </EmptyMedia>
                    <EmptyTitle>{t('tasks.empty.title')}</EmptyTitle>
                    <EmptyDescription>{t('tasks.empty.description')}</EmptyDescription>
                  </EmptyHeader>
                </Empty>
              ) : (
                <Table>
                  <TableHeader>
                    <TableRow className="hover:bg-transparent">
                      <TableHead>{t('tasks.table.task')}</TableHead>
                      <TableHead className="w-[180px]">{t('tasks.table.file')}</TableHead>
                      <TableHead className="w-[160px]">{t('tasks.table.model')}</TableHead>
                      <TableHead className="w-[150px]">{t('tasks.table.created')}</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {jobs.slice(0, HOME_TASK_LIMIT).map((job) => {
                      const file = getTaskFileDisplay(job);
                      const page = getTaskPageDisplay(job);
                      return (
                        <TableRow
                          key={job.id}
                          className="cursor-pointer"
                          onClick={() => void navigate({ to: '/tasks', search: { jobId: job.id } })}
                        >
                          <TableCell>
                            <div className="min-w-0">
                              <div className="flex min-w-0 items-center gap-2">
                                {taskStatusIcon(job)}
                                <span className="truncate text-sm font-medium">
                                  {t('tasks.codegenTitle', { framework: job.framework })}
                                </span>
                              </div>
                              <p className="mt-1 truncate text-xs text-muted-foreground">
                                {page.primary}
                              </p>
                            </div>
                          </TableCell>
                          <TableCell className="text-xs">
                            <span className="block truncate">{file.primary}</span>
                            {file.secondary && (
                              <span className="block truncate text-muted-foreground">
                                {file.secondary}
                              </span>
                            )}
                          </TableCell>
                          <TableCell className="truncate text-xs text-muted-foreground">
                            {formatProviderModel(job)}
                          </TableCell>
                          <TableCell className="text-xs text-muted-foreground">
                            {formatTime(job.createdAt)}
                          </TableCell>
                        </TableRow>
                      );
                    })}
                  </TableBody>
                </Table>
              )}
            </CardContent>
          </Card>

          <Card className="min-w-0 rounded py-0 shadow-none">
            <CardHeader className="min-h-8 border-b border-border px-3 py-2">
              <div>
                <CardTitle className="text-sm">{t('tasks.notifications')}</CardTitle>
                <CardDescription className="text-xs">
                  {t('workbench.home.notificationsDescription')}
                </CardDescription>
              </div>
              <CardAction>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => void navigate({ to: '/tasks', search: { view: 'notifications' } })}
                >
                  {t('workbench.viewAll')}
                </Button>
              </CardAction>
            </CardHeader>
            <CardContent className="flex flex-col gap-1.5 p-3">
              {notifications.length === 0 ? (
                <p className="text-sm text-muted-foreground">{t('tasks.notificationsEmpty')}</p>
              ) : (
                notifications.slice(0, HOME_NOTIFICATION_LIMIT).map((notification) => (
                  <Button
                    key={notification.id}
                    type="button"
                    variant="outline"
                    className="h-auto w-full justify-start rounded bg-background/60 px-2.5 py-2 text-left shadow-none"
                    onClick={() =>
                      void navigate({ to: '/tasks', search: { view: 'notifications' } })
                    }
                  >
                    <span className="min-w-0 flex-1">
                      <span className="flex items-center justify-between gap-2">
                        <span className="truncate text-sm font-medium">{notification.title}</span>
                        {!notification.readAt && (
                          <Badge variant="secondary">{t('workbench.unread')}</Badge>
                        )}
                      </span>
                      {notification.message && (
                        <span className="mt-1 line-clamp-2 text-xs text-muted-foreground">
                          {notification.message}
                        </span>
                      )}
                    </span>
                  </Button>
                ))
              )}
            </CardContent>
          </Card>
        </section>

        <div className="flex justify-end">
          <Button size="sm" onClick={() => void createDesign()} disabled={creating}>
            <Plus data-icon="inline-start" />
            {creating ? t('cloudLibrary.creating') : t('cloudLibrary.newDesign')}
          </Button>
        </div>
      </div>
    </WorkbenchShell>
  );
}

function MetricCard({
  icon: Icon,
  label,
  value,
  detail,
}: {
  icon: typeof Cloud;
  label: string;
  value: number;
  detail: string;
}) {
  return (
    <Card className="min-w-0 rounded py-2.5 shadow-none">
      <CardContent className="px-3">
        <div className="flex items-center justify-between gap-2">
          <div className="min-w-0">
            <p className="text-xs text-muted-foreground">{label}</p>
            <p className="mt-1 text-xl font-semibold tabular-nums">{value}</p>
          </div>
          <span className="flex size-7 items-center justify-center rounded border border-border bg-background/60 text-muted-foreground">
            <Icon />
          </span>
        </div>
        <p className="mt-1.5 truncate text-xs text-muted-foreground">{detail}</p>
      </CardContent>
    </Card>
  );
}

function MiniStat({ label, value }: { label: string; value: number }) {
  return (
    <div className="min-w-0 rounded border border-border bg-background/60 px-2.5 py-2">
      <p className="text-xs text-muted-foreground">{label}</p>
      <p className="mt-1 text-lg font-semibold tabular-nums">{value}</p>
    </div>
  );
}
