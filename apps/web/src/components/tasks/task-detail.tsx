import { useEffect, useMemo } from 'react';
import {
  CheckCircle2,
  Clock3,
  ExternalLink,
  FileCode2,
  Loader2,
  RotateCcw,
  Trash2,
  X,
  XCircle,
} from 'lucide-react';
import { useNavigate } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from '@/components/ui/empty';
import { Skeleton } from '@/components/ui/skeleton';
import { ConfirmActionButton } from '@/components/workbench/confirm-action-button';
import { WorkbenchShell } from '@/components/workbench/workbench-shell';
import { useCodegenJobStore } from '@/stores/codegen-job-store';
import type {
  CloudCodegenJob,
  CloudCodegenJobStep,
  CodegenJobAgentRole,
  CodegenJobStatus,
} from '@/types/cloud';
import {
  clampProgress,
  describeJobKind,
  describePipelineMode,
  durationBetween,
  formatDuration,
  formatProviderModel,
  getTaskFileDisplay,
  getTaskPageDisplay,
  shortId,
} from './task-center-utils';
import {
  QualityReportCard,
  StageArtifactsPanel,
  TimingBreakdownPanel,
} from './task-detail-stage-panels';

interface TaskDetailProps {
  jobId: string;
  embedded?: boolean;
  onClose?: () => void;
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
  quality_check: 3,
  repair: 4,
  final_validation: 5,
  bundler_persist: 6,
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

function normalizeSucceededJobStep(job: CloudCodegenJob, step: CloudCodegenJobStep) {
  if (job.status !== 'succeeded') return step;
  if (step.status !== 'pending' && step.status !== 'running') return step;
  return {
    ...step,
    status: 'succeeded' as const,
    progress: Math.max(Number(step.progress) || 0, 100),
    completedAt: step.completedAt ?? job.completedAt ?? job.updatedAt,
    updatedAt: laterTime(step.updatedAt, job.completedAt ?? job.updatedAt) ?? step.updatedAt,
  };
}

export function TaskDetail({ jobId, embedded = false, onClose }: TaskDetailProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const jobs = useCodegenJobStore((s) => s.jobs);
  const loading = useCodegenJobStore((s) => s.loading);
  const error = useCodegenJobStore((s) => s.error);
  const loadJob = useCodegenJobStore((s) => s.loadJob);
  const cancelJob = useCodegenJobStore((s) => s.cancelJob);
  const deleteJob = useCodegenJobStore((s) => s.deleteJob);
  const retryJob = useCodegenJobStore((s) => s.retryJob);
  const resumeJob = useCodegenJobStore((s) => s.resumeJob);
  const rerunJobStep = useCodegenJobStore((s) => s.rerunJobStep);
  const job = useMemo(() => jobs.find((item) => item.id === jobId), [jobId, jobs]);
  const generationId = job ? getOutputGenerationId(job) : null;
  const visibleSteps = useMemo(() => {
    const steps = getVisibleTaskSteps(job?.steps ?? []);
    return job ? steps.map((step) => normalizeSucceededJobStep(job, step)) : steps;
  }, [job]);
  const jobDuration = job ? durationBetween(job.startedAt ?? job.createdAt, job.completedAt) : 0;

  useEffect(() => {
    void loadJob(jobId);
  }, [jobId, loadJob]);

  const openFile = () => {
    if (!job?.fileId) return;
    if (generationId) {
      void navigate({
        to: '/editor/$fileId',
        params: { fileId: job.fileId },
        search: { generationId },
      });
      return;
    }
    void navigate({
      to: '/editor/$fileId',
      params: { fileId: job.fileId },
    });
  };

  const handleDelete = async () => {
    if (!job) return;
    await deleteJob(job.id);
    if (embedded) {
      onClose?.();
      return;
    }
    void navigate({ to: '/tasks' });
  };

  const content = (
    <div className={embedded ? 'bg-background text-foreground' : 'text-foreground'}>
      <header className="border-b border-border bg-card px-5 py-4">
        <div
          className={
            embedded
              ? 'flex items-center justify-between gap-3'
              : 'flex items-center justify-between gap-3'
          }
        >
          <div className="min-w-0">
            {embedded ? (
              <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                {t('tasks.detailTitle')}
              </p>
            ) : (
              <Button variant="ghost" size="sm" onClick={() => void navigate({ to: '/tasks' })}>
                {t('tasks.backToTasks')}
              </Button>
            )}
            <div className="mt-3 flex items-center gap-2">
              {job ? statusIcon(job.status) : <Loader2 className="h-4 w-4 animate-spin" />}
              <h1 className="truncate text-lg font-semibold">
                {job
                  ? t('tasks.codegenTitle', { framework: job.framework })
                  : t('tasks.detailTitle')}
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
                <>
                  <Button variant="outline" size="sm" onClick={() => void resumeJob(job.id)}>
                    <RotateCcw className="h-4 w-4" />
                    {t('tasks.resumeFromFailedStage')}
                  </Button>
                  <Button variant="ghost" size="sm" onClick={() => void retryJob(job.id)}>
                    {t('tasks.retry')}
                  </Button>
                </>
              )}
              <Button variant="outline" size="sm" onClick={openFile}>
                <FileCode2 className="h-4 w-4" />
                {generationId ? t('tasks.openGeneration') : t('tasks.openFile')}
              </Button>
              <ConfirmActionButton
                variant="ghost"
                size="icon-sm"
                ariaLabel={t('tasks.deleteTask')}
                title={t('tasks.deleteTask')}
                description={t('tasks.confirm.deleteTask')}
                confirmLabel={t('common.delete')}
                cancelLabel={t('common.cancel')}
                className="text-muted-foreground hover:text-destructive"
                onConfirm={handleDelete}
              >
                <Trash2 />
              </ConfirmActionButton>
            </div>
          )}
          {embedded && (
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label={t('common.close')}
              onClick={onClose}
            >
              <X className="h-4 w-4" />
            </Button>
          )}
        </div>
      </header>

      <main
        className={
          embedded
            ? 'grid gap-4 px-5 py-5 xl:grid-cols-[minmax(0,1fr)_300px]'
            : 'grid gap-4 py-5 lg:grid-cols-[minmax(0,1fr)_360px]'
        }
      >
        <section className="flex min-w-0 flex-col gap-4">
          {error && (
            <Alert variant="destructive">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}
          {loading && !job && (
            <div className="rounded-md border border-border bg-card p-4">
              <Skeleton className="h-5 w-40" />
              <Skeleton className="mt-4 h-24 w-full" />
              <Skeleton className="mt-3 h-24 w-full" />
            </div>
          )}
          {!loading && !job && (
            <Empty className="border border-border bg-card">
              <EmptyHeader>
                <EmptyMedia variant="icon">
                  <Clock3 />
                </EmptyMedia>
                <EmptyTitle>{t('tasks.detailNotFound')}</EmptyTitle>
                <EmptyDescription>{t('tasks.loadingDetail')}</EmptyDescription>
              </EmptyHeader>
            </Empty>
          )}
          {job && (
            <>
              <ExecutionSummary job={job} durationMs={jobDuration} t={t} />

              <StepProgressList
                jobId={job.id}
                steps={visibleSteps}
                onRerunStep={rerunJobStep}
                t={t}
              />

              <TimingBreakdownPanel job={job} t={t} />

              <StageArtifactsPanel job={job} t={t} />

              <div className="rounded-md border border-border bg-card p-4">
                <h2 className="text-sm font-semibold">{t('tasks.inputTarget')}</h2>
                <dl className="mt-3 grid gap-3 text-xs sm:grid-cols-2">
                  <div>
                    <dt className="text-muted-foreground">{t('tasks.jobKindLabel')}</dt>
                    <dd className="mt-1 font-medium">{describeJobKind(job, t)}</dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">{t('tasks.pipelineModeLabel')}</dt>
                    <dd className="mt-1 font-medium">{describePipelineMode(job, t)}</dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">{t('tasks.fileLibrary')}</dt>
                    <dd className="mt-1 break-all font-medium">{getTaskFileDisplay(job).primary}</dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">{t('tasks.framework')}</dt>
                    <dd className="mt-1 font-medium">{job.framework}</dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">{t('tasks.page')}</dt>
                    <dd className="mt-1 break-all font-medium">{getTaskPageDisplay(job).primary}</dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">{t('tasks.fileId')}</dt>
                    <dd className="mt-1 break-all font-medium">{job.fileId}</dd>
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
                    <dd className="mt-1 font-medium">{formatProviderModel(job)}</dd>
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
            </>
          )}
        </section>

        <aside className="rounded-md border border-border bg-card p-4">
          <h2 className="text-sm font-semibold">{t('tasks.events')}</h2>
          <div className="mt-3 flex flex-col gap-2">
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

  if (embedded) return content;

  return (
    <WorkbenchShell
      active="tasks"
      title={t('tasks.detailTitle')}
      description={job ? getVisibleStatus(job, t) : t('tasks.detailDescription')}
    >
      {content}
    </WorkbenchShell>
  );
}

function ExecutionSummary({
  job,
  durationMs,
  t,
}: {
  job: CloudCodegenJob;
  durationMs: number;
  t: ReturnType<typeof useTranslation>['t'];
}) {
  const file = getTaskFileDisplay(job);
  const page = getTaskPageDisplay(job);
  const progress = clampProgress(job.progress);
  const metrics = [
    { label: t('tasks.fileLibrary'), value: file.primary, detail: file.secondary },
    { label: t('tasks.page'), value: page.primary, detail: page.secondary },
    { label: t('tasks.providerModel'), value: formatProviderModel(job) },
    { label: t('tasks.pipelineModeLabel'), value: describePipelineMode(job, t) },
    { label: t('tasks.executionDuration'), value: formatDuration(durationMs) },
    { label: t('tasks.startedAt'), value: formatTime(job.startedAt) },
    { label: t('tasks.completedAt'), value: formatTime(job.completedAt) },
  ];

  return (
    <div className="rounded-md border border-border bg-card p-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-sm font-semibold">{t('tasks.executionSummary')}</h2>
          <p className="mt-1 text-xs text-muted-foreground">
            {t('tasks.executionSubtitle', {
              createdAt: formatTime(job.createdAt),
              updatedAt: formatTime(job.updatedAt),
            })}
          </p>
        </div>
        <span className="rounded border border-border bg-background px-2 py-1 text-xs text-muted-foreground">
          {t('tasks.progressPercent', { progress })}
        </span>
      </div>
      <div className="mt-4 h-2 overflow-hidden rounded-full bg-muted">
        <div className="h-full rounded-full bg-primary" style={{ width: `${progress}%` }} />
      </div>
      <dl className="mt-4 grid gap-3 text-xs sm:grid-cols-2 xl:grid-cols-3">
        {metrics.map((item) => (
          <div key={item.label} className="min-w-0 rounded-md border border-border bg-background p-3">
            <dt className="text-muted-foreground">{item.label}</dt>
            <dd className="mt-1 truncate font-medium">{item.value}</dd>
            {item.detail && <dd className="mt-1 truncate text-muted-foreground">{item.detail}</dd>}
          </div>
        ))}
      </dl>
    </div>
  );
}

function StepProgressList({
  jobId,
  steps,
  onRerunStep,
  t,
}: {
  jobId: string;
  steps: CloudCodegenJobStep[];
  onRerunStep: ReturnType<typeof useCodegenJobStore.getState>['rerunJobStep'];
  t: ReturnType<typeof useTranslation>['t'];
}) {
  return (
    <div className="rounded-md border border-border bg-card p-4">
      <h2 className="text-sm font-semibold">{t('tasks.steps')}</h2>
      <div className="mt-3 flex flex-col gap-2">
        {steps.length === 0 ? (
          <p className="text-xs text-muted-foreground">{t('tasks.stepsEmpty')}</p>
        ) : (
          steps.map((step) => (
            <StepProgressCard
              key={step.id}
              jobId={jobId}
              step={step}
              onRerunStep={onRerunStep}
              t={t}
            />
          ))
        )}
      </div>
    </div>
  );
}

function StepProgressCard({
  jobId,
  step,
  onRerunStep,
  t,
}: {
  jobId: string;
  step: CloudCodegenJobStep;
  onRerunStep: ReturnType<typeof useCodegenJobStore.getState>['rerunJobStep'];
  t: ReturnType<typeof useTranslation>['t'];
}) {
  const progress = clampProgress(step.progress);
  const duration = durationBetween(step.startedAt ?? step.createdAt, step.completedAt);
  const inputSummary = compactJson(step.input);
  const outputSummary = compactJson(step.output);
  const qualityReport = getStepQualityReport(step);
  const chunkId = getStepChunkId(step);
  const canRerun =
    step.status === 'failed' &&
    (step.agentRole === 'page_codegen' ||
      step.agentRole === 'quality_check' ||
      step.agentRole === 'repair');
  const rerunStage =
    step.agentRole === 'page_codegen'
      ? 'chunk'
      : step.agentRole === 'quality_check'
        ? 'quality_check'
        : 'repair';

  return (
    <div className="rounded-md border border-border bg-background px-3 py-3 text-xs">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="min-w-0">
          <p className="font-medium">{describeStepRole(step.agentRole, t)}</p>
          <p className="mt-0.5 text-muted-foreground">
            {t('tasks.stepAttempt', { attempt: step.attempt })} · {shortId(step.id)}
          </p>
        </div>
        <div className="flex items-center gap-2 text-muted-foreground">
          <span>{t(`tasks.status.${step.status}`)}</span>
          <span>{formatDuration(duration)}</span>
          {canRerun && (
            <Button
              variant="ghost"
              size="sm"
              onClick={() =>
                void onRerunStep(jobId, {
                  stage: rerunStage,
                  ...(rerunStage === 'chunk' && chunkId ? { chunkId } : {}),
                })
              }
              disabled={rerunStage === 'chunk' && !chunkId}
            >
              <RotateCcw className="h-4 w-4" />
              {rerunStage === 'chunk'
                ? t('tasks.rerunChunkStep')
                : t('tasks.rerunStep')}
            </Button>
          )}
        </div>
      </div>
      {rerunStage === 'chunk' && chunkId && (
        <p className="mt-2 text-muted-foreground">
          {t('tasks.chunkResumeHint', { chunkId })}
        </p>
      )}
      <div className="mt-3 h-1.5 overflow-hidden rounded-full bg-muted">
        <div className="h-full rounded-full bg-primary" style={{ width: `${progress}%` }} />
      </div>
      <dl className="mt-3 grid gap-2 sm:grid-cols-3">
        <div>
          <dt className="text-muted-foreground">{t('tasks.progress')}</dt>
          <dd className="mt-1 font-medium">{t('tasks.progressPercent', { progress })}</dd>
        </div>
        <div>
          <dt className="text-muted-foreground">{t('tasks.startedAt')}</dt>
          <dd className="mt-1 font-medium">{formatTime(step.startedAt)}</dd>
        </div>
        <div>
          <dt className="text-muted-foreground">{t('tasks.completedAt')}</dt>
          <dd className="mt-1 font-medium">{formatTime(step.completedAt)}</dd>
        </div>
      </dl>
      {(inputSummary || outputSummary) && (
        <div className="mt-3 grid gap-2 sm:grid-cols-2">
          {inputSummary && <MetaPreview label={t('tasks.stepInput')} value={inputSummary} />}
          {outputSummary && <MetaPreview label={t('tasks.stepOutput')} value={outputSummary} />}
        </div>
      )}
      {qualityReport && <QualityReportCard report={qualityReport} t={t} />}
      {step.error && <p className="mt-2 text-destructive">{step.error}</p>}
    </div>
  );
}

function getStepQualityReport(step: CloudCodegenJobStep) {
  const report = step.output?.report;
  if (!report || typeof report !== 'object') return null;
  const candidate = report as {
    status?: unknown;
    framework?: unknown;
    checkedAt?: unknown;
    summary?: unknown;
    issues?: unknown;
  };
  if (
    typeof candidate.status !== 'string' ||
    typeof candidate.framework !== 'string' ||
    !candidate.summary ||
    typeof candidate.summary !== 'object'
  ) {
    return null;
  }
  const summary = candidate.summary as Record<string, unknown>;
  const issues = Array.isArray(candidate.issues)
    ? candidate.issues
        .map((issue) => normalizeQualityIssue(issue))
        .filter((issue): issue is NonNullable<ReturnType<typeof normalizeQualityIssue>> => Boolean(issue))
    : [];
  return {
    status: candidate.status,
    framework: candidate.framework,
    checkedAt: typeof candidate.checkedAt === 'string' ? candidate.checkedAt : undefined,
    summary: {
      fileCount: numberFromUnknown(summary.fileCount),
      errorCount: numberFromUnknown(summary.errorCount),
      warningCount: numberFromUnknown(summary.warningCount),
      missingTextCount: numberFromUnknown(summary.missingTextCount),
      missingAssetCount: numberFromUnknown(summary.missingAssetCount),
    },
    issues,
  };
}

function normalizeQualityIssue(issue: unknown) {
  if (!issue || typeof issue !== 'object') return null;
  const value = issue as Record<string, unknown>;
  if (
    typeof value.code !== 'string' ||
    typeof value.severity !== 'string' ||
    typeof value.message !== 'string'
  ) {
    return null;
  }
  return {
    code: value.code,
    severity: value.severity,
    message: value.message,
    filePath: typeof value.filePath === 'string' ? value.filePath : undefined,
  };
}

function numberFromUnknown(value: unknown): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : 0;
}

function getStepChunkId(step: CloudCodegenJobStep): string | null {
  const values = [
    step.input?.chunkId,
    step.output?.chunkId,
    (step.output?.latestCheckpoint as { data?: { chunkId?: unknown } } | undefined)?.data?.chunkId,
  ];
  const chunkId = values.find(
    (value): value is string => typeof value === 'string' && value.length > 0,
  );
  return chunkId ?? null;
}

function MetaPreview({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0 rounded-md border border-border bg-card px-2 py-2">
      <p className="text-muted-foreground">{label}</p>
      <p className="mt-1 truncate font-mono text-[11px] text-foreground">{value}</p>
    </div>
  );
}

function compactJson(value: Record<string, unknown>): string | null {
  return compactUnknownJson(value, 140);
}

function compactUnknownJson(value: unknown, limit: number): string | null {
  if (!value || Object.keys(value).length === 0) return null;
  try {
    const text = JSON.stringify(value);
    return text.length > limit ? `${text.slice(0, Math.max(0, limit - 3))}...` : text;
  } catch {
    return null;
  }
}
