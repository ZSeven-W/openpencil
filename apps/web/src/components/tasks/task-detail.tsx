import { useEffect, useMemo } from 'react';
import {
  ArrowLeft,
  CheckCircle2,
  Clock3,
  ExternalLink,
  FileCode2,
  Loader2,
  RotateCcw,
  XCircle,
} from 'lucide-react';
import { useNavigate } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { useCodegenJobStore } from '@/stores/codegen-job-store';
import type {
  CloudCodegenJob,
  CloudCodegenJobStep,
  CodegenJobAgentRole,
  CodegenJobStatus,
} from '@/types/cloud';

interface TaskDetailProps {
  jobId: string;
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

function statusIcon(status: CodegenJobStatus) {
  if (status === 'succeeded') return <CheckCircle2 className="h-4 w-4 text-primary" />;
  if (status === 'failed') return <XCircle className="h-4 w-4 text-destructive" />;
  if (status === 'running') return <Loader2 className="h-4 w-4 animate-spin text-primary" />;
  return <Clock3 className="h-4 w-4 text-muted-foreground" />;
}

function describeStepRole(role: CodegenJobAgentRole, t: ReturnType<typeof useTranslation>['t']) {
  return t(`tasks.agent.${role}`, { defaultValue: String(role) });
}

function describeEventType(type: string, t: ReturnType<typeof useTranslation>['t']): string {
  return t(`tasks.event.${type}`, { defaultValue: type });
}

function getOutputGenerationId(job: CloudCodegenJob): string | null {
  const outputGenerationId = job.output?.generationId;
  return typeof outputGenerationId === 'string' ? outputGenerationId : job.generationId;
}

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

function describeJobKind(job: CloudCodegenJob, t: ReturnType<typeof useTranslation>['t']) {
  return t(`tasks.jobKind.${job.jobKind ?? 'full_generation'}`);
}

const STEP_STATUS_RANK: Record<CodegenJobStatus, number> = {
  pending: 0,
  running: 1,
  canceled: 2,
  failed: 2,
  succeeded: 3,
};

const STEP_ROLE_ORDER: Record<CodegenJobAgentRole, number> = {
  planner: 0,
  page_codegen: 1,
  reviewer_repair: 2,
  bundler_persist: 3,
};

function timeValue(value?: string | null) {
  if (!value) return 0;
  const time = new Date(value).getTime();
  return Number.isFinite(time) ? time : 0;
}

function earlierTime(a?: string | null, b?: string | null) {
  if (!a) return b ?? null;
  if (!b) return a;
  return timeValue(a) <= timeValue(b) ? a : b;
}

function laterTime(a?: string | null, b?: string | null) {
  if (!a) return b ?? null;
  if (!b) return a;
  return timeValue(a) >= timeValue(b) ? a : b;
}

function mergeDuplicateStep(a: CloudCodegenJobStep, b: CloudCodegenJobStep) {
  const aRank = STEP_STATUS_RANK[a.status] ?? 0;
  const bRank = STEP_STATUS_RANK[b.status] ?? 0;
  const preferred =
    bRank > aRank || (bRank === aRank && timeValue(b.updatedAt) >= timeValue(a.updatedAt))
      ? b
      : a;
  return {
    ...preferred,
    progress: Math.max(Number(a.progress), Number(b.progress)),
    input: { ...(a.input ?? {}), ...(b.input ?? {}) },
    output: { ...(a.output ?? {}), ...(b.output ?? {}) },
    startedAt: earlierTime(a.startedAt, b.startedAt),
    completedAt: laterTime(a.completedAt, b.completedAt),
    createdAt: earlierTime(a.createdAt, b.createdAt) ?? preferred.createdAt,
    updatedAt: laterTime(a.updatedAt, b.updatedAt) ?? preferred.updatedAt,
    error: preferred.error ?? a.error ?? b.error,
  };
}

export function getVisibleTaskSteps(steps: CloudCodegenJobStep[] = []) {
  const grouped = new Map<string, CloudCodegenJobStep>();
  for (const step of steps) {
    const key = `${step.agentRole}:${step.attempt ?? 1}`;
    const existing = grouped.get(key);
    grouped.set(key, existing ? mergeDuplicateStep(existing, step) : step);
  }
  return Array.from(grouped.values()).sort((a, b) => {
    const roleDelta =
      (STEP_ROLE_ORDER[a.agentRole] ?? 99) - (STEP_ROLE_ORDER[b.agentRole] ?? 99);
    if (roleDelta !== 0) return roleDelta;
    const attemptDelta = Number(a.attempt ?? 1) - Number(b.attempt ?? 1);
    if (attemptDelta !== 0) return attemptDelta;
    return timeValue(a.createdAt) - timeValue(b.createdAt);
  });
}

export function TaskDetail({ jobId }: TaskDetailProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const jobs = useCodegenJobStore((s) => s.jobs);
  const loading = useCodegenJobStore((s) => s.loading);
  const error = useCodegenJobStore((s) => s.error);
  const loadJob = useCodegenJobStore((s) => s.loadJob);
  const cancelJob = useCodegenJobStore((s) => s.cancelJob);
  const retryJob = useCodegenJobStore((s) => s.retryJob);
  const job = useMemo(() => jobs.find((item) => item.id === jobId), [jobId, jobs]);
  const generationId = job ? getOutputGenerationId(job) : null;
  const visibleSteps = useMemo(() => getVisibleTaskSteps(job?.steps ?? []), [job?.steps]);

  useEffect(() => {
    void loadJob(jobId);
  }, [jobId, loadJob]);

  const openFile = () => {
    if (!job?.fileId) return;
    void navigate({
      to: '/editor/$fileId',
      params: { fileId: job.fileId },
      search: generationId ? { generationId } : undefined,
    } as never);
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
              {job ? statusIcon(job.status) : <Loader2 className="h-4 w-4 animate-spin" />}
              <h1 className="truncate text-lg font-semibold">
                {job ? t('tasks.codegenTitle', { framework: job.framework }) : t('tasks.detailTitle')}
              </h1>
            </div>
            {job && (
              <p className="mt-1 text-sm text-muted-foreground">
                {getVisibleStatus(job, t)} · {formatTime(job.createdAt)}
              </p>
            )}
          </div>
          {job && (
            <div className="flex items-center gap-2">
              {(job.status === 'pending' || job.status === 'running') && (
                <Button variant="outline" size="sm" onClick={() => void cancelJob(job.id)}>
                  {t('tasks.cancel')}
                </Button>
              )}
              {job.status === 'failed' && (
                <Button variant="outline" size="sm" onClick={() => void retryJob(job.id)}>
                  <RotateCcw className="h-4 w-4" />
                  {t('tasks.retry')}
                </Button>
              )}
              <Button variant="outline" size="sm" onClick={openFile}>
                <FileCode2 className="h-4 w-4" />
                {generationId ? t('tasks.openGeneration') : t('tasks.openFile')}
              </Button>
            </div>
          )}
        </div>
      </header>

      <main className="mx-auto grid max-w-6xl gap-4 px-5 py-5 lg:grid-cols-[minmax(0,1fr)_360px]">
        <section className="min-w-0 space-y-4">
          {error && (
            <div className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">
              {error}
            </div>
          )}
          {loading && !job && (
            <div className="rounded-md border border-border bg-card p-10 text-center text-sm text-muted-foreground">
              {t('tasks.loadingDetail')}
            </div>
          )}
          {!loading && !job && (
            <div className="rounded-md border border-border bg-card p-10 text-center text-sm text-muted-foreground">
              {t('tasks.detailNotFound')}
            </div>
          )}
          {job && (
            <>
              <div className="rounded-md border border-border bg-card p-4">
                <h2 className="text-sm font-semibold">{t('tasks.inputTarget')}</h2>
                <dl className="mt-3 grid gap-3 text-xs sm:grid-cols-2">
                  <div>
                    <dt className="text-muted-foreground">{t('tasks.jobKindLabel')}</dt>
                    <dd className="mt-1 font-medium">{describeJobKind(job, t)}</dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">{t('tasks.fileId')}</dt>
                    <dd className="mt-1 break-all font-medium">{job.fileId}</dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">{t('tasks.framework')}</dt>
                    <dd className="mt-1 font-medium">{job.framework}</dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">{t('tasks.pageId')}</dt>
                    <dd className="mt-1 break-all font-medium">{job.pageId}</dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">{t('tasks.targetKind')}</dt>
                    <dd className="mt-1 font-medium">{job.targetKind}</dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">{t('tasks.documentRevision')}</dt>
                    <dd className="mt-1 font-medium">{job.documentRevision}</dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">{t('tasks.nodeCount')}</dt>
                    <dd className="mt-1 font-medium">{job.nodeIds.length}</dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">{t('tasks.providerModel')}</dt>
                    <dd className="mt-1 font-medium">
                      {[job.provider, job.model].filter(Boolean).join(' / ') || '-'}
                    </dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">{t('tasks.attemptsLabel')}</dt>
                    <dd className="mt-1 font-medium">
                      {t('tasks.attempts', {
                        attempts: job.attempts,
                        maxAttempts: job.maxAttempts,
                      })}
                    </dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">{t('tasks.lockHeartbeat')}</dt>
                    <dd className="mt-1 font-medium">{formatTime(job.lastHeartbeatAt)}</dd>
                  </div>
                  {job.nextRunAt && (
                    <div>
                      <dt className="text-muted-foreground">{t('tasks.nextRunAt')}</dt>
                      <dd className="mt-1 font-medium">{formatTime(job.nextRunAt)}</dd>
                    </div>
                  )}
                  {job.deadLetteredAt && (
                    <div>
                      <dt className="text-muted-foreground">{t('tasks.deadLetteredAt')}</dt>
                      <dd className="mt-1 font-medium">{formatTime(job.deadLetteredAt)}</dd>
                    </div>
                  )}
                  {job.failureType && (
                    <div>
                      <dt className="text-muted-foreground">{t('tasks.failureType')}</dt>
                      <dd className="mt-1 font-medium">{t(`tasks.failure.${job.failureType}`)}</dd>
                    </div>
                  )}
                  <div>
                    <dt className="text-muted-foreground">{t('tasks.targetHash')}</dt>
                    <dd className="mt-1 break-all font-medium">{job.targetHash}</dd>
                  </div>
                  {job.jobKind === 'patch_generation' &&
                    typeof job.inputSnapshot.baseGenerationId === 'string' && (
                      <div>
                        <dt className="text-muted-foreground">{t('tasks.baseGenerationId')}</dt>
                        <dd className="mt-1 break-all font-medium">
                          {job.inputSnapshot.baseGenerationId}
                        </dd>
                      </div>
                    )}
                  {job.jobKind === 'patch_generation' &&
                    typeof job.inputSnapshot.patchInstruction === 'string' && (
                      <div className="sm:col-span-2">
                        <dt className="text-muted-foreground">{t('tasks.patchInstruction')}</dt>
                        <dd className="mt-1 whitespace-pre-wrap font-medium">
                          {job.inputSnapshot.patchInstruction}
                        </dd>
                      </div>
                    )}
                </dl>
              </div>

              <div className="rounded-md border border-border bg-card p-4">
                <h2 className="text-sm font-semibold">{t('tasks.outputHistory')}</h2>
                <div className="mt-3 rounded-md border border-border bg-background px-3 py-2 text-xs">
                  {generationId ? (
                    <div className="flex items-center justify-between gap-2">
                      <div className="min-w-0">
                        <p className="text-muted-foreground">{t('tasks.generationId')}</p>
                        <p className="mt-1 break-all font-medium">{generationId}</p>
                      </div>
                      <Button variant="ghost" size="sm" onClick={openFile}>
                        <ExternalLink className="h-4 w-4" />
                        {t('tasks.openGeneration')}
                      </Button>
                    </div>
                  ) : (
                    <p className="text-muted-foreground">{t('tasks.outputEmpty')}</p>
                  )}
                </div>
              </div>

              <div className="rounded-md border border-border bg-card p-4">
                <h2 className="text-sm font-semibold">{t('tasks.steps')}</h2>
                <div className="mt-3 space-y-2">
                  {visibleSteps.length === 0 ? (
                    <p className="text-xs text-muted-foreground">{t('tasks.stepsEmpty')}</p>
                  ) : (
                    visibleSteps.map((step) => (
                      <div
                        key={step.id}
                        className="rounded-md border border-border bg-background px-3 py-2 text-xs"
                      >
                        <div className="flex items-center justify-between gap-2">
                          <span className="font-medium">{describeStepRole(step.agentRole, t)}</span>
                          <span className="text-muted-foreground">
                            {t(`tasks.status.${step.status}`)}
                          </span>
                        </div>
                        <div className="mt-2 h-1 overflow-hidden rounded-full bg-muted">
                          <div
                            className="h-full rounded-full bg-primary"
                            style={{ width: `${step.progress}%` }}
                          />
                        </div>
                        <p className="mt-2 text-muted-foreground">
                          {formatTime(step.startedAt)} - {formatTime(step.completedAt)}
                        </p>
                        {step.error && <p className="mt-1 text-destructive">{step.error}</p>}
                      </div>
                    ))
                  )}
                </div>
              </div>
            </>
          )}
        </section>

        <aside className="rounded-md border border-border bg-card p-4">
          <h2 className="text-sm font-semibold">{t('tasks.events')}</h2>
          <div className="mt-3 space-y-2">
            {!job || (job.events ?? []).length === 0 ? (
              <p className="text-xs text-muted-foreground">{t('tasks.eventsEmpty')}</p>
            ) : (
              job.events?.map((event) => (
                <div
                  key={event.id}
                  className="rounded-md border border-border bg-background px-3 py-2 text-xs"
                >
                  <div className="flex items-center justify-between gap-2">
                    <span className="font-medium">{describeEventType(event.type, t)}</span>
                    <span className="text-muted-foreground">{formatTime(event.createdAt)}</span>
                  </div>
                  {event.message && (
                    <p className="mt-1 text-muted-foreground">{event.message}</p>
                  )}
                </div>
              ))
            )}
          </div>
        </aside>
      </main>
    </div>
  );
}
