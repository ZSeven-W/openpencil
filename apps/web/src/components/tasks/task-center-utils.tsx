import { CheckCircle2, Clock3, Loader2, XCircle } from 'lucide-react';
import type { ReactNode } from 'react';
import type { useTranslation } from 'react-i18next';
import { Badge } from '@/components/ui/badge';
import type {
  CloudCodegenJob,
  CodegenJobStatus,
  CodegenPipelineMode,
  TaskNotification,
} from '@/types/cloud';

export const FILTERS: Array<{ id: 'all' | CodegenJobStatus; icon: typeof Clock3 }> = [
  { id: 'all', icon: Clock3 },
  { id: 'pending', icon: Clock3 },
  { id: 'running', icon: Loader2 },
  { id: 'succeeded', icon: CheckCircle2 },
  { id: 'failed', icon: XCircle },
  { id: 'canceled', icon: XCircle },
];

export const TASK_PAGE_SIZE = 10;
export const TASK_LIST_LIMIT = 10;
export const NOTIFICATION_PAGE_SIZE = 10;

export function getVisibleStatus(job: CloudCodegenJob, t: ReturnType<typeof useTranslation>['t']) {
  if (job.deadLetteredAt) return t('tasks.status.deadLettered');
  if (job.status === 'pending' && job.nextRunAt) {
    const nextRun = new Date(job.nextRunAt).getTime();
    if (Number.isFinite(nextRun) && nextRun > Date.now()) {
      return t('tasks.status.retryScheduled');
    }
  }
  return t(`tasks.status.${job.status}`);
}

export function statusIcon(status: CodegenJobStatus) {
  if (status === 'succeeded') return <CheckCircle2 className="text-primary" />;
  if (status === 'failed') return <XCircle className="text-destructive" />;
  if (status === 'running') return <Loader2 className="animate-spin text-primary" />;
  return <Clock3 className="text-muted-foreground" />;
}

export function StatusBadge({
  status,
  children,
}: {
  status: CodegenJobStatus;
  children: ReactNode;
}) {
  const variant = status === 'failed' ? 'destructive' : status === 'succeeded' ? 'default' : 'secondary';
  return <Badge variant={variant}>{children}</Badge>;
}

export function formatTime(value: string): string {
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

export function formatDuration(ms?: number): string {
  const value = Number(ms ?? 0);
  if (!Number.isFinite(value) || value <= 0) return '0 ms';
  if (value < 1000) return `${Math.round(value)} ms`;
  if (value < 60_000) return `${(value / 1000).toFixed(1)} s`;
  const minutes = Math.floor(value / 60_000);
  const seconds = Math.round((value % 60_000) / 1000);
  return seconds > 0 ? `${minutes}m ${seconds}s` : `${minutes}m`;
}

export function durationBetween(start?: string | null, end?: string | null): number {
  const started = start ? new Date(start).getTime() : 0;
  const ended = end ? new Date(end).getTime() : Date.now();
  if (!Number.isFinite(started) || !Number.isFinite(ended) || started <= 0 || ended < started) {
    return 0;
  }
  return ended - started;
}

export function shortId(value?: string | null): string {
  if (!value) return '-';
  return value.length > 14 ? `${value.slice(0, 8)}...${value.slice(-4)}` : value;
}

function snapshotString(job: CloudCodegenJob, key: string): string | null {
  const value = job.inputSnapshot?.[key];
  if (typeof value !== 'string') return null;
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

export function getTaskFileDisplay(job: CloudCodegenJob): { primary: string; secondary: string } {
  const name = job.fileName?.trim() || snapshotString(job, 'fileName');
  return {
    primary: name ?? shortId(job.fileId),
    secondary: name ? shortId(job.fileId) : '',
  };
}

export function getTaskPageDisplay(job: CloudCodegenJob): { primary: string; secondary: string } {
  const name = job.pageName?.trim() || snapshotString(job, 'pageName');
  return {
    primary: name ?? shortId(job.pageId),
    secondary: name ? shortId(job.pageId) : '',
  };
}

export function formatProviderModel(job: CloudCodegenJob): string {
  return [job.provider, job.model].filter(Boolean).join(' / ') || '-';
}

export function clampProgress(progress: number): number {
  if (!Number.isFinite(progress)) return 0;
  return Math.min(100, Math.max(0, Math.round(progress)));
}

export function renderNotificationCopy(
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

export function isCancellable(job: CloudCodegenJob): boolean {
  return job.status === 'pending' || job.status === 'running';
}

export function describeJobKind(job: CloudCodegenJob, t: ReturnType<typeof useTranslation>['t']) {
  return t(`tasks.jobKind.${job.jobKind ?? 'full_generation'}`);
}

export function getCodegenPipelineMode(job: CloudCodegenJob): CodegenPipelineMode {
  const rowMode = normalizePipelineMode(job.pipelineMode);
  if (rowMode) return rowMode;

  const output = objectRecord(job.output);
  const explicit = normalizePipelineMode(output.pipelineMode);
  if (explicit) return explicit;

  const metadata = objectRecord(output.metadata);
  const metadataMode = normalizePipelineMode(metadata.pipelineMode);
  if (metadataMode) return metadataMode;

  for (const timing of collectTimingCandidates(output)) {
    const stages = getProviderCallStages(timing);
    if (stages.includes('direct_generation')) return 'direct_generation';
    if (stages.some((stage) => stage === 'planning' || stage === 'chunk' || stage === 'assembly')) {
      return 'full_pipeline';
    }
  }

  const checkpoints = Array.isArray(output.checkpoints) ? output.checkpoints : [];
  if (
    checkpoints.some((checkpoint) => {
      const stage = objectRecord(checkpoint).stage;
      return stage === 'planning' || stage === 'chunk' || stage === 'assembly';
    })
  ) {
    return 'full_pipeline';
  }

  return 'unknown';
}

export function describePipelineMode(
  job: CloudCodegenJob,
  t: ReturnType<typeof useTranslation>['t'],
) {
  return t(`tasks.pipeline.${getCodegenPipelineMode(job)}`);
}

function normalizePipelineMode(value: unknown): CodegenPipelineMode | null {
  return value === 'direct_generation' || value === 'full_pipeline' || value === 'unknown'
    ? value
    : null;
}

function collectTimingCandidates(output: Record<string, unknown>): Record<string, unknown>[] {
  const candidates = [
    objectRecord(output.timing),
    objectRecord(objectRecord(output.metadata).timing),
  ];
  const checkpoints = Array.isArray(output.checkpoints) ? output.checkpoints : [];
  for (const checkpoint of checkpoints) {
    const data = objectRecord(objectRecord(checkpoint).data);
    candidates.push(objectRecord(data.timing));
  }
  return candidates.filter((candidate) => Object.keys(candidate).length > 0);
}

function getProviderCallStages(timing: Record<string, unknown>): string[] {
  const calls = Array.isArray(timing.providerCalls) ? timing.providerCalls : [];
  return calls
    .map((call) => objectRecord(call).stage)
    .filter((stage): stage is string => typeof stage === 'string');
}

function objectRecord(value: unknown): Record<string, unknown> {
  return value && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}
