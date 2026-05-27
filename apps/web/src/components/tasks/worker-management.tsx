import { useEffect, useId, useMemo, useState } from 'react';
import {
  Activity,
  Clock3,
  Gauge,
  History,
  Loader2,
  RotateCcw,
  Save,
  ServerCog,
  ShieldAlert,
  ShieldCheck,
  XCircle,
} from 'lucide-react';
import { useNavigate } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Field, FieldGroup, FieldLabel, FieldTitle } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import { Progress } from '@/components/ui/progress';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { WorkbenchShell, type WorkbenchNavKey } from '@/components/workbench/workbench-shell';
import { useCodegenJobStore } from '@/stores/codegen-job-store';
import type { CodegenProviderConfig, CodegenJobFailureType } from '@/types/cloud';
import {
  EmptyPanel,
  FAILURE_OPTIONS,
  formatDuration,
  formatPercent,
  formatTime,
  getReplayableJobs,
  isCircuitOpen,
  NumberField,
  PAGE_SIZE,
  pageInfo,
  QueueMetricCard,
  toInputDate,
  type PageKind,
  type WorkerView,
} from './worker-management/worker-management-utils';

export function WorkerManagement({ view = 'workers' }: { view?: WorkerView }) {
  const { t } = useTranslation();
  const navigate = useNavigate({ from: '/tasks/workers' });
  const providerFilterId = useId();
  const failureTypeId = useId();
  const deadLetteredFromId = useId();
  const deadLetteredToId = useId();
  const jobs = useCodegenJobStore((s) => s.jobs);
  const providerConfigs = useCodegenJobStore((s) => s.providerConfigs);
  const providerConfigAudits = useCodegenJobStore((s) => s.providerConfigAudits);
  const queueAccess = useCodegenJobStore((s) => s.queueAccess);
  const workerStats = useCodegenJobStore((s) => s.workerStats);
  const workerOverview = useCodegenJobStore((s) => s.workerOverview);
  const loading = useCodegenJobStore((s) => s.loading);
  const error = useCodegenJobStore((s) => s.error);
  const refreshJobs = useCodegenJobStore((s) => s.refreshJobs);
  const refreshWorkerOverview = useCodegenJobStore((s) => s.refreshWorkerOverview);
  const refreshWorkerStats = useCodegenJobStore((s) => s.refreshWorkerStats);
  const refreshQueueAccess = useCodegenJobStore((s) => s.refreshQueueAccess);
  const refreshProviderConfigs = useCodegenJobStore((s) => s.refreshProviderConfigs);
  const refreshProviderConfigAudits = useCodegenJobStore((s) => s.refreshProviderConfigAudits);
  const saveProviderConfig = useCodegenJobStore((s) => s.saveProviderConfig);
  const replayFailedJobs = useCodegenJobStore((s) => s.replayFailedJobs);
  const [providerFilter, setProviderFilter] = useState('');
  const [failureType, setFailureType] = useState<'all' | CodegenJobFailureType>('all');
  const [deadLetteredFrom, setDeadLetteredFrom] = useState('');
  const [deadLetteredTo, setDeadLetteredTo] = useState('');
  const [configReasons, setConfigReasons] = useState<Record<string, string>>({});
  const [offsets, setOffsets] = useState<Record<PageKind, number>>({
    workers: 0,
    providers: 0,
    failedJobs: 0,
  });
  const [configDrafts, setConfigDrafts] = useState<Record<string, CodegenProviderConfig>>({});
  const metrics = workerOverview?.metrics;
  const replayableJobs = useMemo(
    () => getReplayableJobs(jobs, workerOverview?.failedJobs ?? []),
    [jobs, workerOverview?.failedJobs],
  );
  const access = queueAccess ?? {
    role: 'viewer',
    bootstrapMode: false,
    canViewWorkers: true,
    canReplayFailed: false,
    canManageProviders: false,
  };

  const overviewFilters = useMemo(() => ({
    workerLimit: PAGE_SIZE,
    workerOffset: offsets.workers,
    providerLimit: PAGE_SIZE,
    providerOffset: offsets.providers,
    failedLimit: PAGE_SIZE,
    failedOffset: offsets.failedJobs,
    provider: providerFilter || undefined,
    failureType: failureType === 'all' ? undefined : failureType,
    deadLetteredFrom: toInputDate(deadLetteredFrom),
    deadLetteredTo: toInputDate(deadLetteredTo),
  }), [deadLetteredFrom, deadLetteredTo, failureType, offsets, providerFilter]);

  useEffect(() => {
    void refreshJobs({ deadLettered: true, limit: PAGE_SIZE });
    void refreshQueueAccess();
    void refreshProviderConfigs({ limit: PAGE_SIZE });
    void refreshProviderConfigAudits({ limit: PAGE_SIZE });
    void refreshWorkerStats();
  }, [
    refreshJobs,
    refreshProviderConfigAudits,
    refreshProviderConfigs,
    refreshQueueAccess,
    refreshWorkerStats,
  ]);

  useEffect(() => {
    void refreshWorkerOverview(overviewFilters);
  }, [overviewFilters, refreshWorkerOverview]);

  useEffect(() => {
    setConfigDrafts((current) => {
      const next = { ...current };
      for (const config of providerConfigs) {
        if (!next[config.provider]) next[config.provider] = config;
      }
      return next;
    });
  }, [providerConfigs]);

  const refresh = () => {
    void refreshJobs({ deadLettered: true, limit: PAGE_SIZE, force: true });
    void refreshQueueAccess({ force: true });
    void refreshProviderConfigs({ limit: PAGE_SIZE, force: true });
    void refreshProviderConfigAudits({ provider: providerFilter || undefined, limit: PAGE_SIZE, force: true });
    void refreshWorkerStats({ force: true });
    void refreshWorkerOverview({ ...overviewFilters, force: true });
  };

  const replayFiltered = async () => {
    await replayFailedJobs({
      provider: providerFilter || undefined,
      failureType: failureType === 'all' ? undefined : failureType,
      deadLetteredFrom: toInputDate(deadLetteredFrom),
      deadLetteredTo: toInputDate(deadLetteredTo),
      limit: PAGE_SIZE,
    });
    await refreshWorkerOverview(overviewFilters);
    await refreshProviderConfigAudits({ provider: providerFilter || undefined, limit: PAGE_SIZE });
  };

  const replayVisible = async () => {
    await replayFailedJobs({
      jobIds: replayableJobs.map((job) => job.id),
      limit: PAGE_SIZE,
    });
    await refreshWorkerOverview(overviewFilters);
    await refreshProviderConfigAudits({ provider: providerFilter || undefined, limit: PAGE_SIZE });
  };

  const updateOffset = (kind: PageKind, direction: -1 | 1) => {
    setOffsets((current) => ({
      ...current,
      [kind]: Math.max(0, current[kind] + direction * PAGE_SIZE),
    }));
  };

  const updateDraft = (
    provider: string,
    patch: Partial<Pick<
      CodegenProviderConfig,
      'enabled' | 'maxPerMinute' | 'circuitThreshold' | 'circuitOpenMs'
    >>,
  ) => {
    setConfigDrafts((current) => {
      const base = current[provider] ?? providerConfigs.find((item) => item.provider === provider);
      if (!base) return current;
      return { ...current, [provider]: { ...base, ...patch } };
    });
  };

  const saveConfig = async (provider: string) => {
    const draft = configDrafts[provider];
    if (!draft) return;
    await saveProviderConfig(provider, {
      enabled: draft.enabled,
      maxPerMinute: draft.maxPerMinute,
      circuitThreshold: draft.circuitThreshold,
      circuitOpenMs: draft.circuitOpenMs,
      reason: configReasons[provider]?.trim() || undefined,
    });
    await refreshProviderConfigAudits({ provider, limit: PAGE_SIZE });
  };

  const WorkerPageControls = ({ kind }: { kind: PageKind }) => {
    const page = pageInfo(workerOverview?.pages[kind]);
    const end = Math.min(page.total, page.offset + page.limit);
    return (
      <div className="flex items-center gap-2 text-xs text-muted-foreground">
        <span>{t('tasks.pageRange', { from: page.total === 0 ? 0 : page.offset + 1, to: end, total: page.total })}</span>
        <Button
          variant="outline"
          size="sm"
          onClick={() => updateOffset(kind, -1)}
          disabled={page.offset === 0}
        >
          {t('tasks.previousPage')}
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={() => updateOffset(kind, 1)}
          disabled={page.offset + page.limit >= page.total}
        >
          {t('tasks.nextPage')}
        </Button>
      </div>
    );
  };

  const activeNav: WorkbenchNavKey =
    view === 'providers' ? 'providers' : view === 'audit' ? 'audit' : 'workers';
  const providerReasonInputId = (provider: string) =>
    `provider-reason-${provider.replace(/[^a-zA-Z0-9_-]/g, '-')}`;

  return (
    <WorkbenchShell
      active={activeNav}
      title={
        view === 'providers'
          ? t('tasks.providerConfig')
          : view === 'audit'
            ? t('tasks.configAuditHistory')
            : t('tasks.workerManagement')
      }
      description={t('tasks.workerSubtitle')}
      toolbar={
        <div className="mb-2 flex min-h-8 flex-wrap items-center justify-end gap-2 border-b border-border pb-2">
          <div className="flex items-center gap-2">
            {view === 'workers' && (
              <Button
                variant="outline"
                size="sm"
                className="h-7 px-2 text-xs shadow-none"
                onClick={replayVisible}
                disabled={replayableJobs.length === 0 || !access.canReplayFailed}
              >
                <RotateCcw data-icon="inline-start" />
                {t('tasks.replayVisible')}
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
      <main className="flex w-full flex-col gap-2">
        {error && (
          <Alert variant="destructive">
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}

        <section className="grid gap-2 md:grid-cols-4">
          <QueueMetricCard icon={Activity} label={t('tasks.metric.total')} value={metrics?.total ?? 0} />
          <QueueMetricCard icon={Clock3} label={t('tasks.metric.running')} value={metrics?.running ?? 0} />
          <QueueMetricCard icon={XCircle} label={t('tasks.metric.deadLettered')} value={metrics?.deadLettered ?? 0} />
          <QueueMetricCard
            icon={Gauge}
            label={t('tasks.metric.failureRate')}
            value={formatPercent(metrics?.failureRate)}
            caption={t('tasks.metric.averageDuration', {
              duration: formatDuration(metrics?.averageDurationMs),
            })}
          />
        </section>

        <Card className="rounded py-0 shadow-none">
          <CardContent className="p-3">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <div>
              <h2 className="text-sm font-semibold">{t('tasks.queueAccess')}</h2>
              <p className="mt-1 text-xs text-muted-foreground">
                {t('tasks.queueRole', { role: access.role })}
                {access.bootstrapMode ? ` · ${t('tasks.bootstrapMode')}` : ''}
              </p>
            </div>
            <div className="grid gap-2 text-xs text-muted-foreground sm:grid-cols-3">
              <span>{t('tasks.canViewWorkers')}: {access.canViewWorkers ? t('common.yes') : t('common.no')}</span>
              <span>{t('tasks.canReplayFailed')}: {access.canReplayFailed ? t('common.yes') : t('common.no')}</span>
              <span>{t('tasks.canManageProviders')}: {access.canManageProviders ? t('common.yes') : t('common.no')}</span>
            </div>
          </div>
          </CardContent>
        </Card>

        <Card className="rounded py-0 shadow-none">
          <CardHeader className="min-h-8 border-b border-border px-3 py-2">
            <div className="flex items-center gap-2">
              <Gauge className="text-primary" />
              <CardTitle>{t('tasks.runtimeStats')}</CardTitle>
            </div>
          </CardHeader>
          <CardContent className="p-3">
            <div className="grid gap-2 md:grid-cols-3">
            <Card className="rounded bg-background py-2.5 shadow-none">
              <CardContent className="px-3">
              <p className="text-xs text-muted-foreground">{t('tasks.workerActive')}</p>
              <p className="mt-1 text-xl font-semibold">{workerStats?.workers.active ?? 0}</p>
              </CardContent>
            </Card>
            <Card className="rounded bg-background py-2.5 shadow-none">
              <CardContent className="px-3">
              <p className="text-xs text-muted-foreground">{t('tasks.workerStale')}</p>
              <p className="mt-1 text-xl font-semibold">{workerStats?.workers.stale ?? 0}</p>
              </CardContent>
            </Card>
            <Card className="rounded bg-background py-2.5 shadow-none">
              <CardContent className="px-3">
              <p className="text-xs text-muted-foreground">{t('tasks.statsBuckets')}</p>
              <p className="mt-1 text-xl font-semibold">{workerStats?.buckets.length ?? 0}</p>
              </CardContent>
            </Card>
          </div>
          <div className="mt-3 grid gap-2 lg:grid-cols-2">
            {(workerStats?.buckets ?? []).slice(-7).map((bucket) => (
              <div key={bucket.bucket} className="rounded border border-border bg-background px-2.5 py-2 text-xs">
                <div className="flex items-center justify-between">
                  <span className="font-medium">{bucket.bucket}</span>
                  <span className="text-muted-foreground">{formatPercent(bucket.failureRate)}</span>
                </div>
                <Progress
                  className="mt-2 h-1.5"
                  value={bucket.total > 0
                    ? Math.max(8, Math.round((bucket.succeeded / bucket.total) * 100))
                    : 0}
                  aria-label={t('tasks.runtimeChartBar', {
                    bucket: bucket.bucket,
                    total: bucket.total,
                    failed: bucket.failed,
                  })}
                />
                <p className="mt-1 text-muted-foreground">
                  {t('tasks.metric.total')}: {bucket.total} · {t('tasks.metric.succeeded')}: {bucket.succeeded} · {t('tasks.metric.failed')}: {bucket.failed}
                </p>
              </div>
            ))}
          </div>
          {(workerStats?.providers.length ?? 0) > 0 && (
            <div className="mt-4 flex flex-col gap-2">
              <h3 className="text-xs font-medium text-muted-foreground">{t('tasks.providerRuntimeStats')}</h3>
              {workerStats?.providers.map((provider) => (
                <div key={provider.provider} className="grid grid-cols-[96px_minmax(0,1fr)_64px] items-center gap-2 text-xs">
                  <span className="truncate font-medium">{provider.provider}</span>
                  <Progress
                    className="h-1.5"
                    value={provider.total > 0
                      ? Math.max(8, Math.round((provider.succeeded / provider.total) * 100))
                      : 0}
                    aria-label={t('tasks.providerRuntimeChartBar', {
                      provider: provider.provider,
                      total: provider.total,
                      failed: provider.failed,
                    })}
                  />
                  <span className="text-right text-muted-foreground">{formatPercent(provider.failureRate)}</span>
                </div>
              ))}
            </div>
          )}
          </CardContent>
        </Card>

        <Card className="rounded py-0 shadow-none">
          <CardContent className="p-3">
          <FieldGroup className="grid gap-2 md:grid-cols-5">
            <Field className="gap-1">
              <FieldLabel htmlFor={providerFilterId} className="text-xs text-muted-foreground">
                {t('tasks.providerFilter')}
              </FieldLabel>
              <Input
                id={providerFilterId}
                className="h-7 text-xs"
                value={providerFilter}
                onChange={(event) => {
                  setOffsets((current) => ({ ...current, failedJobs: 0, providers: 0 }));
                  setProviderFilter(event.target.value.trim());
                }}
                placeholder="openai"
              />
            </Field>
            <Field className="gap-1">
              <FieldLabel htmlFor={failureTypeId} className="text-xs text-muted-foreground">{t('tasks.failureType')}</FieldLabel>
              <Select
                value={failureType}
                onValueChange={(next) => {
                  setOffsets((current) => ({ ...current, failedJobs: 0 }));
                  setFailureType(next as 'all' | CodegenJobFailureType);
                }}
              >
                <SelectTrigger id={failureTypeId} size="sm" className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectGroup>
                    {FAILURE_OPTIONS.map((item) => (
                      <SelectItem key={item} value={item}>
                        {item === 'all' ? t('tasks.filter.all') : t(`tasks.failure.${item}`)}
                      </SelectItem>
                    ))}
                  </SelectGroup>
                </SelectContent>
              </Select>
            </Field>
            <Field className="gap-1">
              <FieldLabel htmlFor={deadLetteredFromId} className="text-xs text-muted-foreground">
                {t('tasks.deadLetteredFrom')}
              </FieldLabel>
              <Input
                id={deadLetteredFromId}
                type="datetime-local"
                className="h-7 text-xs"
                value={deadLetteredFrom}
                onChange={(event) => {
                  setOffsets((current) => ({ ...current, failedJobs: 0 }));
                  setDeadLetteredFrom(event.target.value);
                }}
              />
            </Field>
            <Field className="gap-1">
              <FieldLabel htmlFor={deadLetteredToId} className="text-xs text-muted-foreground">
                {t('tasks.deadLetteredTo')}
              </FieldLabel>
              <Input
                id={deadLetteredToId}
                type="datetime-local"
                className="h-7 text-xs"
                value={deadLetteredTo}
                onChange={(event) => {
                  setOffsets((current) => ({ ...current, failedJobs: 0 }));
                  setDeadLetteredTo(event.target.value);
                }}
              />
            </Field>
            <div className="flex items-end">
              <Button
                variant="outline"
                size="sm"
                onClick={replayFiltered}
                className="h-8 w-full text-xs shadow-none"
                disabled={!access.canReplayFailed}
              >
                <RotateCcw data-icon="inline-start" />
                {t('tasks.replayFiltered')}
              </Button>
            </div>
          </FieldGroup>
          </CardContent>
        </Card>

        {view !== 'audit' && (
        <section className="grid gap-2 lg:grid-cols-[minmax(0,1fr)_22rem]">
          <div className="flex flex-col gap-2">
            {view === 'workers' && (
            <Card className="rounded py-0 shadow-none">
              <CardHeader className="min-h-8 border-b border-border px-3 py-2">
                <div>
                  <CardTitle>{t('tasks.workers')}</CardTitle>
                  <CardDescription>{t('tasks.workerSubtitle')}</CardDescription>
                </div>
                <WorkerPageControls kind="workers" />
              </CardHeader>
              <CardContent className="flex flex-col gap-1.5 p-3">
                {(workerOverview?.workers.length ?? 0) === 0 ? (
                  <EmptyPanel
                    icon={ServerCog}
                    title={t('tasks.workersEmpty')}
                    description={t('tasks.workerSubtitle')}
                  />
                ) : (
                  workerOverview?.workers.map((worker) => (
                    <div key={worker.workerId} className="rounded border border-border bg-background px-2.5 py-2 text-xs">
                      <div className="flex items-center justify-between gap-2">
                        <span className="font-medium">{worker.workerId}</span>
                        <Badge variant={worker.active ? 'default' : 'secondary'}>
                          {worker.active ? t('tasks.workerActive') : t('tasks.workerIdle')}
                        </Badge>
                      </div>
                      <p className="mt-1 text-xs text-muted-foreground">
                        {t('tasks.lastHeartbeat')}: {formatTime(worker.lastHeartbeatAt)}
                      </p>
                    </div>
                  ))
                )}
              </CardContent>
            </Card>
            )}

            {view === 'workers' && (
            <Card className="rounded py-0 shadow-none">
              <CardHeader className="min-h-8 border-b border-border px-3 py-2">
                <CardTitle>{t('tasks.providerHealth')}</CardTitle>
                <WorkerPageControls kind="providers" />
              </CardHeader>
              <CardContent className="flex flex-col gap-1.5 p-3">
                {(workerOverview?.providers.length ?? 0) === 0 ? (
                  <EmptyPanel
                    icon={ShieldCheck}
                    title={t('tasks.providersEmpty')}
                    description={t('tasks.providerRuntimeStats')}
                  />
                ) : (
                  workerOverview?.providers.map((provider) => {
                    const open = isCircuitOpen(provider.circuitOpenUntil);
                    return (
                      <div key={provider.provider} className="rounded border border-border bg-background px-2.5 py-2 text-xs">
                        <div className="flex items-center justify-between gap-2">
                          <span className="font-medium">{provider.provider}</span>
                          <Badge variant={open ? 'destructive' : 'default'}>
                            {open ? <ShieldAlert /> : <ShieldCheck />}
                            {open ? t('tasks.circuitOpen') : t('tasks.circuitClosed')}
                          </Badge>
                        </div>
                        <div className="mt-2 grid gap-2 text-xs text-muted-foreground sm:grid-cols-3">
                          <span>{t('tasks.metric.total')}: {provider.total}</span>
                          <span>{t('tasks.metric.failed')}: {provider.failed}</span>
                          <span>{t('tasks.metric.failureRate')}: {formatPercent(provider.failureRate)}</span>
                        </div>
                        {provider.lastError && <p className="mt-2 text-xs text-destructive">{provider.lastError}</p>}
                      </div>
                    );
                  })
                )}
              </CardContent>
            </Card>
            )}

            {view === 'providers' && (
            <Card className="rounded py-0 shadow-none">
              <CardHeader className="min-h-8 border-b border-border px-3 py-2">
                <CardTitle>{t('tasks.providerConfig')}</CardTitle>
              </CardHeader>
              <CardContent className="flex flex-col gap-2 p-3">
                {providerConfigs.length === 0 ? (
                  <EmptyPanel
                    icon={ShieldCheck}
                    title={t('tasks.providerConfigEmpty')}
                    description={t('tasks.providerConfig')}
                  />
                ) : (
                  providerConfigs.map((config) => {
                    const draft = configDrafts[config.provider] ?? config;
                    const reasonInputId = providerReasonInputId(config.provider);
                    return (
                      <div key={config.provider} className="rounded border border-border bg-background p-2.5">
                        <div className="flex items-center justify-between gap-2">
                          <span className="text-sm font-medium">{config.provider}</span>
                          <Field orientation="horizontal" className="w-auto gap-2">
                            <Switch
                              size="sm"
                              checked={draft.enabled}
                              disabled={!access.canManageProviders}
                              onCheckedChange={(checked) =>
                                updateDraft(config.provider, { enabled: checked })
                              }
                            />
                            <FieldTitle className="text-xs text-muted-foreground">
                              {t('tasks.providerEnabled')}
                            </FieldTitle>
                          </Field>
                        </div>
                        <div className="mt-3 grid gap-2 sm:grid-cols-3">
                          <NumberField label={t('tasks.maxPerMinute')} value={draft.maxPerMinute} onChange={(value) => updateDraft(config.provider, { maxPerMinute: value })} />
                          <NumberField label={t('tasks.circuitThreshold')} value={draft.circuitThreshold} onChange={(value) => updateDraft(config.provider, { circuitThreshold: value })} />
                          <NumberField label={t('tasks.circuitOpenMs')} value={draft.circuitOpenMs} onChange={(value) => updateDraft(config.provider, { circuitOpenMs: value })} />
                        </div>
                        <div className="mt-3 flex flex-wrap items-end justify-between gap-2">
                          <Field className="min-w-48 flex-1 gap-1">
                            <FieldLabel htmlFor={reasonInputId} className="text-xs text-muted-foreground">
                              {t('tasks.changeReason')}
                            </FieldLabel>
                            <Input
                              id={reasonInputId}
                              className="h-8 text-xs"
                              value={configReasons[config.provider] ?? ''}
                              disabled={!access.canManageProviders}
                              onChange={(event) => setConfigReasons((current) => ({
                                ...current,
                                [config.provider]: event.target.value,
                              }))}
                            />
                          </Field>
                          <Button
                            variant="outline"
                            size="sm"
                            disabled={!access.canManageProviders}
                            onClick={() => void saveConfig(config.provider)}
                          >
                            <Save data-icon="inline-start" />
                            {t('tasks.saveProviderConfig')}
                          </Button>
                        </div>
                      </div>
                    );
                  })
                )}
              </CardContent>
            </Card>
            )}
          </div>

          <Card className="rounded py-0 shadow-none">
            <CardHeader className="min-h-8 border-b border-border px-3 py-2">
              <CardTitle>{t('tasks.failedReplay')}</CardTitle>
              <WorkerPageControls kind="failedJobs" />
            </CardHeader>
            <CardContent className="flex flex-col gap-1.5 p-3">
              {replayableJobs.length === 0 ? (
                <EmptyPanel
                  icon={RotateCcw}
                  title={t('tasks.failedReplayEmpty')}
                  description={t('tasks.failedReplay')}
                />
              ) : (
                replayableJobs.map((job) => (
                  <Button
                    key={job.id}
                    type="button"
                    variant="outline"
                    className="h-auto w-full justify-start rounded bg-background px-2.5 py-2 text-left text-xs shadow-none"
                    onClick={() => void navigate({ to: '/tasks', search: { jobId: job.id } })}
                  >
                    <span className="min-w-0 flex-1">
                      <span className="flex items-center justify-between gap-2">
                        <span className="font-medium">{job.provider ?? job.framework}</span>
                        <span className="text-muted-foreground">
                          {job.failureType
                            ? t(`tasks.failure.${job.failureType}`)
                            : t('tasks.status.failed')}
                        </span>
                      </span>
                      <span className="mt-1 block text-muted-foreground">
                        {formatTime(job.deadLetteredAt)}
                      </span>
                      {job.error && <span className="mt-1 block text-destructive">{job.error}</span>}
                    </span>
                  </Button>
                ))
              )}
            </CardContent>
          </Card>
        </section>
        )}

        {view === 'audit' && (
        <Card className="rounded py-0 shadow-none">
          <CardHeader className="min-h-8 border-b border-border px-3 py-2">
            <div className="flex items-center gap-2">
              <History className="text-primary" />
              <CardTitle>{t('tasks.configAuditHistory')}</CardTitle>
            </div>
          </CardHeader>
          <CardContent className="flex flex-col gap-1.5 p-3">
            {providerConfigAudits.length === 0 ? (
              <EmptyPanel
                icon={History}
                title={t('tasks.configAuditEmpty')}
                description={t('tasks.configAuditHistory')}
              />
            ) : (
              providerConfigAudits.map((audit) => (
                <div key={audit.id} className="rounded border border-border bg-background px-2.5 py-2 text-xs">
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <span className="font-medium">{audit.provider}</span>
                    <span className="text-muted-foreground">{formatTime(audit.createdAt)}</span>
                  </div>
                  <p className="mt-1 text-muted-foreground">
                    {audit.actorEmail ?? audit.actorId ?? t('versionHistory.systemActor')}
                  </p>
                  {audit.reason && <p className="mt-1 text-foreground">{audit.reason}</p>}
                  <p className="mt-1 font-mono text-[11px] text-muted-foreground">
                    {JSON.stringify(audit.afterConfig)}
                  </p>
                </div>
              ))
            )}
          </CardContent>
        </Card>
        )}
      </main>
    </WorkbenchShell>
  );
}
