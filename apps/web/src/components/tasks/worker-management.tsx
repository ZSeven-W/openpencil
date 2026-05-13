import { useEffect, useMemo, useState } from 'react';
import {
  Activity,
  ArrowLeft,
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
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { useCodegenJobStore } from '@/stores/codegen-job-store';
import type {
  CloudCodegenJob,
  CodegenJobFailureType,
  CodegenProviderConfig,
  CodegenQueuePage,
} from '@/types/cloud';

type PageKind = 'workers' | 'providers' | 'failedJobs';

const PAGE_SIZE = 10;
const FAILURE_OPTIONS: Array<'all' | CodegenJobFailureType> = [
  'all',
  'rate_limit',
  'timeout',
  'provider_error',
  'execution_error',
  'canceled',
];

function formatPercent(value?: number): string {
  return `${Math.round(Number(value ?? 0) * 100)}%`;
}

function formatDuration(ms?: number): string {
  const value = Number(ms ?? 0);
  if (!Number.isFinite(value) || value <= 0) return '0 ms';
  if (value < 1000) return `${Math.round(value)} ms`;
  return `${(value / 1000).toFixed(1)} s`;
}

function formatTime(value?: string | null): string {
  if (!value) return '-';
  try {
    return new Intl.DateTimeFormat(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    }).format(new Date(value));
  } catch {
    return value;
  }
}

function isCircuitOpen(value?: string | null): boolean {
  const timestamp = value ? new Date(value).getTime() : 0;
  return Number.isFinite(timestamp) && timestamp > Date.now();
}

function pageInfo(page?: CodegenQueuePage) {
  return {
    total: page?.total ?? 0,
    limit: page?.limit ?? PAGE_SIZE,
    offset: page?.offset ?? 0,
  };
}

function toInputDate(value?: string) {
  if (!value) return undefined;
  return new Date(value).toISOString();
}

function getReplayableJobs(jobs: CloudCodegenJob[], overviewJobs: CloudCodegenJob[]) {
  const merged = new Map<string, CloudCodegenJob>();
  for (const job of [...overviewJobs, ...jobs]) {
    if (job.status === 'failed' || job.deadLetteredAt) merged.set(job.id, job);
  }
  return Array.from(merged.values());
}

function NumberField({
  label,
  value,
  onChange,
}: {
  label: string;
  value: number;
  onChange: (value: number) => void;
}) {
  return (
    <label className="space-y-1 text-xs text-muted-foreground">
      <span>{label}</span>
      <input
        type="number"
        className="h-8 w-full rounded-md border border-input bg-background px-2 text-xs text-foreground"
        value={value}
        onChange={(event) => onChange(Number(event.target.value))}
      />
    </label>
  );
}

export function WorkerManagement() {
  const { t } = useTranslation();
  const navigate = useNavigate();
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
    void refreshJobs({ deadLettered: true });
    void refreshQueueAccess();
    void refreshProviderConfigs();
    void refreshProviderConfigAudits();
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
    void refreshJobs({ deadLettered: true });
    void refreshQueueAccess();
    void refreshProviderConfigs();
    void refreshProviderConfigAudits({ provider: providerFilter || undefined });
    void refreshWorkerStats();
    void refreshWorkerOverview(overviewFilters);
  };

  const replayFiltered = async () => {
    await replayFailedJobs({
      provider: providerFilter || undefined,
      failureType: failureType === 'all' ? undefined : failureType,
      deadLetteredFrom: toInputDate(deadLetteredFrom),
      deadLetteredTo: toInputDate(deadLetteredTo),
      limit: 50,
    });
    await refreshWorkerOverview(overviewFilters);
    await refreshProviderConfigAudits({ provider: providerFilter || undefined });
  };

  const replayVisible = async () => {
    await replayFailedJobs({
      jobIds: replayableJobs.map((job) => job.id),
      limit: 50,
    });
    await refreshWorkerOverview(overviewFilters);
    await refreshProviderConfigAudits({ provider: providerFilter || undefined });
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
    await refreshProviderConfigAudits({ provider });
  };

  const Pagination = ({ kind }: { kind: PageKind }) => {
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

  return (
    <div className="min-h-screen bg-background text-foreground">
      <header className="border-b border-border bg-card px-5 py-4">
        <div className="mx-auto flex max-w-6xl items-center justify-between gap-3">
          <div className="min-w-0">
            <Button variant="ghost" size="sm" onClick={() => void navigate({ to: '/tasks' })}>
              <ArrowLeft className="h-4 w-4" />
              {t('tasks.backToTasks')}
            </Button>
            <div className="mt-3 flex items-center gap-2">
              <ServerCog className="h-5 w-5 text-primary" />
              <h1 className="truncate text-lg font-semibold">{t('tasks.workerManagement')}</h1>
            </div>
            <p className="mt-1 text-sm text-muted-foreground">{t('tasks.workerSubtitle')}</p>
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={replayVisible}
              disabled={replayableJobs.length === 0 || !access.canReplayFailed}
            >
              <RotateCcw className="h-4 w-4" />
              {t('tasks.replayVisible')}
            </Button>
            <Button variant="outline" size="sm" onClick={refresh} disabled={loading}>
              {loading && <Loader2 className="h-4 w-4 animate-spin" />}
              {t('tasks.refresh')}
            </Button>
          </div>
        </div>
      </header>

      <main className="mx-auto max-w-6xl space-y-4 px-5 py-5">
        {error && (
          <div className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">
            {error}
          </div>
        )}

        <section className="grid gap-3 md:grid-cols-4">
          {[
            { icon: Activity, label: t('tasks.metric.total'), value: metrics?.total ?? 0 },
            { icon: Clock3, label: t('tasks.metric.running'), value: metrics?.running ?? 0 },
            { icon: XCircle, label: t('tasks.metric.deadLettered'), value: metrics?.deadLettered ?? 0 },
            {
              icon: Gauge,
              label: t('tasks.metric.failureRate'),
              value: formatPercent(metrics?.failureRate),
              caption: t('tasks.metric.averageDuration', {
                duration: formatDuration(metrics?.averageDurationMs),
              }),
            },
          ].map((item) => {
            const Icon = item.icon;
            return (
              <div key={item.label} className="rounded-md border border-border bg-card p-4">
                <div className="flex items-center gap-2 text-xs text-muted-foreground">
                  <Icon className="h-4 w-4" />
                  {item.label}
                </div>
                <p className="mt-2 text-2xl font-semibold">{item.value}</p>
                {'caption' in item && item.caption && (
                  <p className="mt-1 text-xs text-muted-foreground">{item.caption}</p>
                )}
              </div>
            );
          })}
        </section>

        <section className="rounded-md border border-border bg-card p-4">
          <div className="flex flex-wrap items-center justify-between gap-3">
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
        </section>

        <section className="rounded-md border border-border bg-card p-4">
          <div className="mb-3 flex items-center gap-2">
            <Gauge className="h-4 w-4 text-primary" />
            <h2 className="text-sm font-semibold">{t('tasks.runtimeStats')}</h2>
          </div>
          <div className="grid gap-3 md:grid-cols-3">
            <div className="rounded-md border border-border bg-background p-3">
              <p className="text-xs text-muted-foreground">{t('tasks.workerActive')}</p>
              <p className="mt-1 text-xl font-semibold">{workerStats?.workers.active ?? 0}</p>
            </div>
            <div className="rounded-md border border-border bg-background p-3">
              <p className="text-xs text-muted-foreground">{t('tasks.workerStale')}</p>
              <p className="mt-1 text-xl font-semibold">{workerStats?.workers.stale ?? 0}</p>
            </div>
            <div className="rounded-md border border-border bg-background p-3">
              <p className="text-xs text-muted-foreground">{t('tasks.statsBuckets')}</p>
              <p className="mt-1 text-xl font-semibold">{workerStats?.buckets.length ?? 0}</p>
            </div>
          </div>
          <div className="mt-3 grid gap-2 lg:grid-cols-2">
            {(workerStats?.buckets ?? []).slice(-7).map((bucket) => (
              <div key={bucket.bucket} className="rounded-md border border-border bg-background px-3 py-2 text-xs">
                <div className="flex items-center justify-between">
                  <span className="font-medium">{bucket.bucket}</span>
                  <span className="text-muted-foreground">{formatPercent(bucket.failureRate)}</span>
                </div>
                <div
                  className="mt-2 h-1.5 overflow-hidden rounded-full bg-muted"
                  aria-label={t('tasks.runtimeChartBar', {
                    bucket: bucket.bucket,
                    total: bucket.total,
                    failed: bucket.failed,
                  })}
                >
                  <div
                    className="h-full rounded-full bg-primary"
                    style={{
                      width: `${bucket.total > 0
                        ? Math.max(8, Math.round((bucket.succeeded / bucket.total) * 100))
                        : 0}%`,
                    }}
                  />
                </div>
                <p className="mt-1 text-muted-foreground">
                  {t('tasks.metric.total')}: {bucket.total} · {t('tasks.metric.succeeded')}: {bucket.succeeded} · {t('tasks.metric.failed')}: {bucket.failed}
                </p>
              </div>
            ))}
          </div>
          {(workerStats?.providers.length ?? 0) > 0 && (
            <div className="mt-4 space-y-2">
              <h3 className="text-xs font-medium text-muted-foreground">{t('tasks.providerRuntimeStats')}</h3>
              {workerStats?.providers.map((provider) => (
                <div key={provider.provider} className="grid grid-cols-[96px_minmax(0,1fr)_64px] items-center gap-2 text-xs">
                  <span className="truncate font-medium">{provider.provider}</span>
                  <div
                    className="h-1.5 overflow-hidden rounded-full bg-muted"
                    aria-label={t('tasks.providerRuntimeChartBar', {
                      provider: provider.provider,
                      total: provider.total,
                      failed: provider.failed,
                    })}
                  >
                    <div
                      className="h-full rounded-full bg-primary"
                      style={{
                        width: `${provider.total > 0
                          ? Math.max(8, Math.round((provider.succeeded / provider.total) * 100))
                          : 0}%`,
                      }}
                    />
                  </div>
                  <span className="text-right text-muted-foreground">{formatPercent(provider.failureRate)}</span>
                </div>
              ))}
            </div>
          )}
        </section>

        <section className="rounded-md border border-border bg-card p-4">
          <div className="grid gap-3 md:grid-cols-5">
            <label className="space-y-1 text-xs text-muted-foreground">
              <span>{t('tasks.providerFilter')}</span>
              <input
                className="h-8 w-full rounded-md border border-input bg-background px-2 text-xs text-foreground"
                value={providerFilter}
                onChange={(event) => {
                  setOffsets((current) => ({ ...current, failedJobs: 0, providers: 0 }));
                  setProviderFilter(event.target.value.trim());
                }}
                placeholder="openai"
              />
            </label>
            <label className="space-y-1 text-xs text-muted-foreground">
              <span>{t('tasks.failureType')}</span>
              <select
                className="h-8 w-full rounded-md border border-input bg-background px-2 text-xs text-foreground"
                value={failureType}
                onChange={(event) => {
                  setOffsets((current) => ({ ...current, failedJobs: 0 }));
                  setFailureType(event.target.value as 'all' | CodegenJobFailureType);
                }}
              >
                {FAILURE_OPTIONS.map((item) => (
                  <option key={item} value={item}>
                    {item === 'all' ? t('tasks.filter.all') : t(`tasks.failure.${item}`)}
                  </option>
                ))}
              </select>
            </label>
            <label className="space-y-1 text-xs text-muted-foreground">
              <span>{t('tasks.deadLetteredFrom')}</span>
              <input
                type="datetime-local"
                className="h-8 w-full rounded-md border border-input bg-background px-2 text-xs text-foreground"
                value={deadLetteredFrom}
                onChange={(event) => {
                  setOffsets((current) => ({ ...current, failedJobs: 0 }));
                  setDeadLetteredFrom(event.target.value);
                }}
              />
            </label>
            <label className="space-y-1 text-xs text-muted-foreground">
              <span>{t('tasks.deadLetteredTo')}</span>
              <input
                type="datetime-local"
                className="h-8 w-full rounded-md border border-input bg-background px-2 text-xs text-foreground"
                value={deadLetteredTo}
                onChange={(event) => {
                  setOffsets((current) => ({ ...current, failedJobs: 0 }));
                  setDeadLetteredTo(event.target.value);
                }}
              />
            </label>
            <div className="flex items-end">
              <Button
                variant="outline"
                size="sm"
                onClick={replayFiltered}
                className="w-full"
                disabled={!access.canReplayFailed}
              >
                <RotateCcw className="h-4 w-4" />
                {t('tasks.replayFiltered')}
              </Button>
            </div>
          </div>
        </section>

        <section className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_380px]">
          <div className="space-y-4">
            <div className="rounded-md border border-border bg-card p-4">
              <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
                <h2 className="text-sm font-semibold">{t('tasks.workers')}</h2>
                <Pagination kind="workers" />
              </div>
              <div className="space-y-2">
                {(workerOverview?.workers.length ?? 0) === 0 ? (
                  <p className="rounded-md border border-border bg-background px-3 py-6 text-center text-sm text-muted-foreground">
                    {t('tasks.workersEmpty')}
                  </p>
                ) : (
                  workerOverview?.workers.map((worker) => (
                    <div key={worker.workerId} className="rounded-md border border-border bg-background px-3 py-2 text-sm">
                      <div className="flex items-center justify-between gap-2">
                        <span className="font-medium">{worker.workerId}</span>
                        <span className={cn('rounded-full px-2 py-0.5 text-[10px]', worker.active ? 'bg-primary/10 text-primary' : 'bg-muted text-muted-foreground')}>
                          {worker.active ? t('tasks.workerActive') : t('tasks.workerIdle')}
                        </span>
                      </div>
                      <p className="mt-1 text-xs text-muted-foreground">
                        {t('tasks.lastHeartbeat')}: {formatTime(worker.lastHeartbeatAt)}
                      </p>
                    </div>
                  ))
                )}
              </div>
            </div>

            <div className="rounded-md border border-border bg-card p-4">
              <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
                <h2 className="text-sm font-semibold">{t('tasks.providerHealth')}</h2>
                <Pagination kind="providers" />
              </div>
              <div className="space-y-2">
                {(workerOverview?.providers.length ?? 0) === 0 ? (
                  <p className="rounded-md border border-border bg-background px-3 py-6 text-center text-sm text-muted-foreground">
                    {t('tasks.providersEmpty')}
                  </p>
                ) : (
                  workerOverview?.providers.map((provider) => {
                    const open = isCircuitOpen(provider.circuitOpenUntil);
                    return (
                      <div key={provider.provider} className="rounded-md border border-border bg-background px-3 py-2 text-sm">
                        <div className="flex items-center justify-between gap-2">
                          <span className="font-medium">{provider.provider}</span>
                          <span className={cn('flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px]', open ? 'bg-destructive/10 text-destructive' : 'bg-primary/10 text-primary')}>
                            {open ? <ShieldAlert className="h-3 w-3" /> : <ShieldCheck className="h-3 w-3" />}
                            {open ? t('tasks.circuitOpen') : t('tasks.circuitClosed')}
                          </span>
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
              </div>
            </div>

            <div className="rounded-md border border-border bg-card p-4">
              <h2 className="text-sm font-semibold">{t('tasks.providerConfig')}</h2>
              <div className="mt-3 space-y-3">
                {providerConfigs.length === 0 ? (
                  <p className="rounded-md border border-border bg-background px-3 py-6 text-center text-sm text-muted-foreground">
                    {t('tasks.providerConfigEmpty')}
                  </p>
                ) : (
                  providerConfigs.map((config) => {
                    const draft = configDrafts[config.provider] ?? config;
                    return (
                      <div key={config.provider} className="rounded-md border border-border bg-background p-3">
                        <div className="flex items-center justify-between gap-2">
                          <span className="text-sm font-medium">{config.provider}</span>
                          <label className="flex items-center gap-2 text-xs text-muted-foreground">
                            <input
                              type="checkbox"
                              checked={draft.enabled}
                              disabled={!access.canManageProviders}
                              onChange={(event) => updateDraft(config.provider, { enabled: event.target.checked })}
                            />
                            {t('tasks.providerEnabled')}
                          </label>
                        </div>
                        <div className="mt-3 grid gap-2 sm:grid-cols-3">
                          <NumberField label={t('tasks.maxPerMinute')} value={draft.maxPerMinute} onChange={(value) => updateDraft(config.provider, { maxPerMinute: value })} />
                          <NumberField label={t('tasks.circuitThreshold')} value={draft.circuitThreshold} onChange={(value) => updateDraft(config.provider, { circuitThreshold: value })} />
                          <NumberField label={t('tasks.circuitOpenMs')} value={draft.circuitOpenMs} onChange={(value) => updateDraft(config.provider, { circuitOpenMs: value })} />
                        </div>
                        <div className="mt-3 flex flex-wrap items-end justify-between gap-2">
                          <label className="min-w-48 flex-1 space-y-1 text-xs text-muted-foreground">
                            <span>{t('tasks.changeReason')}</span>
                            <input
                              className="h-8 w-full rounded-md border border-input bg-background px-2 text-xs text-foreground"
                              value={configReasons[config.provider] ?? ''}
                              disabled={!access.canManageProviders}
                              onChange={(event) => setConfigReasons((current) => ({
                                ...current,
                                [config.provider]: event.target.value,
                              }))}
                            />
                          </label>
                          <Button
                            variant="outline"
                            size="sm"
                            disabled={!access.canManageProviders}
                            onClick={() => void saveConfig(config.provider)}
                          >
                            <Save className="h-4 w-4" />
                            {t('tasks.saveProviderConfig')}
                          </Button>
                        </div>
                      </div>
                    );
                  })
                )}
              </div>
            </div>
          </div>

          <aside className="rounded-md border border-border bg-card p-4">
            <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
              <h2 className="text-sm font-semibold">{t('tasks.failedReplay')}</h2>
              <Pagination kind="failedJobs" />
            </div>
            <div className="space-y-2">
              {replayableJobs.length === 0 ? (
                <p className="rounded-md border border-border bg-background px-3 py-6 text-center text-sm text-muted-foreground">
                  {t('tasks.failedReplayEmpty')}
                </p>
              ) : (
                replayableJobs.map((job) => (
                  <button
                    key={job.id}
                    type="button"
                    className="w-full rounded-md border border-border bg-background px-3 py-2 text-left text-xs hover:bg-accent"
                    onClick={() => void navigate({ to: '/tasks/$jobId', params: { jobId: job.id } })}
                  >
                    <div className="flex items-center justify-between gap-2">
                      <span className="font-medium">{job.provider ?? job.framework}</span>
                      <span className="text-muted-foreground">
                        {job.failureType ? t(`tasks.failure.${job.failureType}`) : t('tasks.status.failed')}
                      </span>
                    </div>
                    <p className="mt-1 text-muted-foreground">{formatTime(job.deadLetteredAt)}</p>
                    {job.error && <p className="mt-1 text-destructive">{job.error}</p>}
                  </button>
                ))
              )}
            </div>
          </aside>
        </section>

        <section className="rounded-md border border-border bg-card p-4">
          <div className="mb-3 flex items-center gap-2">
            <History className="h-4 w-4 text-primary" />
            <h2 className="text-sm font-semibold">{t('tasks.configAuditHistory')}</h2>
          </div>
          <div className="space-y-2">
            {providerConfigAudits.length === 0 ? (
              <p className="rounded-md border border-border bg-background px-3 py-6 text-center text-sm text-muted-foreground">
                {t('tasks.configAuditEmpty')}
              </p>
            ) : (
              providerConfigAudits.map((audit) => (
                <div key={audit.id} className="rounded-md border border-border bg-background px-3 py-2 text-xs">
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
          </div>
        </section>
      </main>
    </div>
  );
}
