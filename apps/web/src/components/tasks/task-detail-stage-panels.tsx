import type { useTranslation } from 'react-i18next';
import type { CloudCodegenJob } from '@/types/cloud';
import { formatDuration } from './task-center-utils';

type StageArtifact = {
  stage: string;
  status: string;
  attempt: number;
  createdAt?: string;
  data: Record<string, unknown>;
};

type TimingBreakdown = {
  planningMs?: number;
  chunkMs?: number;
  assemblyMs?: number;
  qualityCheckMs?: number;
  repairMs?: number;
  providerMs?: number;
  providerCallTotalMs?: number;
  providerCallCount?: number;
  totalMs?: number;
};

type TimingInsight = {
  key: string;
  message: string;
};

type QualityIssue = {
  code: string;
  severity: string;
  message: string;
  filePath?: string;
};

type QualityReportPreview = {
  status: string;
  framework: string;
  checkedAt?: string;
  summary: {
    fileCount: number;
    errorCount: number;
    warningCount: number;
    missingTextCount: number;
    missingAssetCount: number;
  };
  issues: QualityIssue[];
};

type RepairAttemptPreview = {
  attempt: number;
  durationMs?: number;
  error?: string;
  issues: QualityIssue[];
};

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

function timeValue(value?: string | null) {
  if (!value) return 0;
  const time = new Date(value).getTime();
  return Number.isFinite(time) ? time : 0;
}

export function TimingBreakdownPanel({
  job,
  t,
}: {
  job: CloudCodegenJob;
  t: ReturnType<typeof useTranslation>['t'];
}) {
  const timing = getJobTiming(job);
  if (!timing) return null;

  const totalMs = Math.max(0, timing.totalMs ?? sumTiming(timing));
  const insights = getTimingInsights(timing, totalMs, t);
  const rows = [
    { key: 'planningMs', label: t('tasks.timing.planning'), value: timing.planningMs },
    { key: 'chunkMs', label: t('tasks.timing.chunk'), value: timing.chunkMs },
    { key: 'assemblyMs', label: t('tasks.timing.assembly'), value: timing.assemblyMs },
    {
      key: 'qualityCheckMs',
      label: t('tasks.timing.qualityCheck'),
      value: timing.qualityCheckMs,
    },
    { key: 'repairMs', label: t('tasks.timing.repair'), value: timing.repairMs },
    { key: 'providerMs', label: t('tasks.timing.provider'), value: timing.providerMs },
    {
      key: 'providerCallTotalMs',
      label: t('tasks.timing.providerCalls'),
      value: timing.providerCallTotalMs,
    },
    { key: 'totalMs', label: t('tasks.timing.total'), value: totalMs },
  ].filter((row) => typeof row.value === 'number' && Number.isFinite(row.value));

  if (rows.length === 0) return null;

  return (
    <div className="rounded-md border border-border bg-card p-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-sm font-semibold">{t('tasks.timingBreakdown')}</h2>
          <p className="mt-1 text-xs text-muted-foreground">{t('tasks.timingBreakdownHint')}</p>
        </div>
        <span className="rounded border border-border bg-background px-2 py-1 text-xs text-muted-foreground">
          {formatDuration(totalMs)}
        </span>
      </div>
      <div className="mt-3 flex flex-col gap-2">
        {rows.map((row) => {
          const value = row.value ?? 0;
          const percent = totalMs > 0 ? Math.min(100, Math.round((value / totalMs) * 100)) : 0;
          return (
            <div key={row.key} className="rounded-md border border-border bg-background px-3 py-2">
              <div className="flex items-center justify-between gap-3 text-xs">
                <span className="font-medium">{row.label}</span>
                <span className="text-muted-foreground">{formatDuration(value)}</span>
              </div>
              <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-muted">
                <div className="h-full rounded-full bg-primary" style={{ width: `${percent}%` }} />
              </div>
            </div>
          );
        })}
      </div>
      {insights.length > 0 && (
        <div className="mt-3 rounded-md border border-amber-500/20 bg-amber-500/8 px-3 py-2 text-xs">
          <p className="font-medium text-amber-700">{t('tasks.timingInsights')}</p>
          <ul className="mt-2 flex flex-col gap-1 text-muted-foreground">
            {insights.map((insight) => (
              <li key={insight.key}>{insight.message}</li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}

export function StageArtifactsPanel({
  job,
  t,
}: {
  job: CloudCodegenJob;
  t: ReturnType<typeof useTranslation>['t'];
}) {
  const artifacts = getStageArtifacts(job);
  return (
    <div className="rounded-md border border-border bg-card p-4">
      <div className="flex items-center justify-between gap-3">
        <div>
          <h2 className="text-sm font-semibold">{t('tasks.stageArtifacts')}</h2>
          <p className="mt-1 text-xs text-muted-foreground">{t('tasks.stageArtifactsHint')}</p>
        </div>
        <span className="rounded border border-border bg-background px-2 py-1 text-xs text-muted-foreground">
          {t('tasks.stageArtifactCount', { count: artifacts.length })}
        </span>
      </div>
      <div className="mt-3 flex flex-col gap-2">
        {artifacts.length === 0 ? (
          <p className="text-xs text-muted-foreground">{t('tasks.stageArtifactsEmpty')}</p>
        ) : (
          artifacts.map((artifact, index) => (
            <StageArtifactCard
              key={`${artifact.stage}:${artifact.attempt}:${index}`}
              artifact={artifact}
              t={t}
            />
          ))
        )}
      </div>
    </div>
  );
}

function StageArtifactCard({
  artifact,
  t,
}: {
  artifact: StageArtifact;
  t: ReturnType<typeof useTranslation>['t'];
}) {
  const summary = describeStageArtifact(artifact, t);
  const preview = compactUnknownJson(artifact.data, 420);
  const qualityReport = getArtifactQualityReport(artifact);
  const repairAttempt = getArtifactRepairAttempt(artifact);
  return (
    <details className="rounded-md border border-border bg-background px-3 py-2 text-xs">
      <summary className="cursor-pointer list-none">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="min-w-0">
            <p className="font-medium">{artifact.stage}</p>
            <p className="mt-0.5 text-muted-foreground">
              {t('tasks.stepAttempt', { attempt: artifact.attempt })} · {formatTime(artifact.createdAt)}
            </p>
          </div>
          <span className="rounded border border-border bg-card px-2 py-1 text-muted-foreground">
            {artifact.status}
          </span>
        </div>
        {summary && <p className="mt-2 text-muted-foreground">{summary}</p>}
      </summary>
      {preview && (
        <pre className="mt-3 max-h-48 overflow-auto rounded-md border border-border bg-card p-3 font-mono text-[11px] leading-relaxed text-foreground">
          {preview}
        </pre>
      )}
      {qualityReport && <QualityReportCard report={qualityReport} t={t} />}
      {repairAttempt && <RepairAttemptCard attempt={repairAttempt} t={t} />}
    </details>
  );
}

export function QualityReportCard({
  report,
  t,
}: {
  report: QualityReportPreview;
  t: ReturnType<typeof useTranslation>['t'];
}) {
  const metrics = [
    { label: t('tasks.quality.files'), value: report.summary.fileCount },
    { label: t('tasks.quality.errors'), value: report.summary.errorCount },
    { label: t('tasks.quality.warnings'), value: report.summary.warningCount },
    { label: t('tasks.quality.missingText'), value: report.summary.missingTextCount },
    { label: t('tasks.quality.missingAssets'), value: report.summary.missingAssetCount },
  ];
  const warningIssues = report.issues.filter((issue) => issue.severity === 'warning');

  return (
    <div className="mt-3 rounded-md border border-border bg-card p-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div>
          <p className="font-medium">{t('tasks.qualityReport')}</p>
          <p className="mt-0.5 text-muted-foreground">
            {report.framework} · {formatTime(report.checkedAt)}
          </p>
        </div>
        <span className="rounded border border-border bg-background px-2 py-1 text-muted-foreground">
          {report.status}
        </span>
      </div>
      <dl className="mt-3 grid gap-2 sm:grid-cols-5">
        {metrics.map((metric) => (
          <div key={metric.label} className="rounded border border-border bg-background px-2 py-2">
            <dt className="text-muted-foreground">{metric.label}</dt>
            <dd className="mt-1 font-medium">{metric.value}</dd>
          </div>
        ))}
      </dl>
      {report.summary.warningCount > 0 && (
        <div className="mt-3 rounded-md border border-amber-500/20 bg-amber-500/8 px-3 py-2">
          <p className="font-medium text-amber-700">{t('tasks.qualityWarnings')}</p>
          <p className="mt-1 text-muted-foreground">{t('tasks.qualityWarningsHint')}</p>
          <div className="mt-2 flex flex-col gap-2">
            {warningIssues.length > 0 ? (
              warningIssues.map((issue, index) => (
                <QualityIssueCard
                  key={`warning:${issue.code}:${issue.filePath ?? ''}:${index}`}
                  issue={issue}
                />
              ))
            ) : (
              <p className="text-muted-foreground">{t('tasks.qualityWarningsNoDetails')}</p>
            )}
          </div>
        </div>
      )}
      <div className="mt-3 flex flex-col gap-2">
        {report.issues.length === 0 ? (
          <p className="text-muted-foreground">{t('tasks.qualityIssuesEmpty')}</p>
        ) : (
          report.issues.slice(0, 6).map((issue, index) => (
            <QualityIssueCard
              key={`${issue.code}:${issue.filePath ?? ''}:${index}`}
              issue={issue}
            />
          ))
        )}
        {report.issues.length > 6 && (
          <p className="text-muted-foreground">
            {t('tasks.qualityMoreIssues', { count: report.issues.length - 6 })}
          </p>
        )}
      </div>
    </div>
  );
}

function QualityIssueCard({ issue }: { issue: QualityIssue }) {
  return (
    <div className="rounded border border-border bg-background px-2 py-2">
      <div className="flex flex-wrap items-center gap-2">
        <span className="font-medium">{issue.code}</span>
        <span className="rounded bg-muted px-1.5 py-0.5 text-muted-foreground">
          {issue.severity}
        </span>
        {issue.filePath && (
          <span className="break-all text-muted-foreground">{issue.filePath}</span>
        )}
      </div>
      <p className="mt-1 text-muted-foreground">{issue.message}</p>
    </div>
  );
}

function RepairAttemptCard({
  attempt,
  t,
}: {
  attempt: RepairAttemptPreview;
  t: ReturnType<typeof useTranslation>['t'];
}) {
  return (
    <div className="mt-3 rounded-md border border-border bg-card p-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div>
          <p className="font-medium">{t('tasks.repairAttempts')}</p>
          <p className="mt-0.5 text-muted-foreground">
            {t('tasks.stageArtifact.repair', { attempt: attempt.attempt })}
            {typeof attempt.durationMs === 'number'
              ? ` · ${t('tasks.repairDuration', { ms: attempt.durationMs })}`
              : ''}
          </p>
        </div>
      </div>
      {attempt.error && <p className="mt-2 text-destructive">{attempt.error}</p>}
      <div className="mt-3 flex flex-col gap-2">
        {attempt.issues.length === 0 ? (
          <p className="text-muted-foreground">{t('tasks.repairIssuesEmpty')}</p>
        ) : (
          attempt.issues.slice(0, 6).map((issue, index) => (
            <div
              key={`${issue.code}:${issue.filePath ?? ''}:${index}`}
              className="rounded border border-border bg-background px-2 py-2"
            >
              <div className="flex flex-wrap items-center gap-2">
                <span className="font-medium">{issue.code}</span>
                <span className="rounded bg-muted px-1.5 py-0.5 text-muted-foreground">
                  {issue.severity}
                </span>
                {issue.filePath && (
                  <span className="break-all text-muted-foreground">{issue.filePath}</span>
                )}
              </div>
              <p className="mt-1 text-muted-foreground">{issue.message}</p>
            </div>
          ))
        )}
      </div>
    </div>
  );
}

function getStageArtifacts(job: CloudCodegenJob): StageArtifact[] {
  const checkpoints = Array.isArray(job.output?.checkpoints) ? job.output.checkpoints : [];
  return checkpoints
    .map((checkpoint): StageArtifact | null => {
      if (!checkpoint || typeof checkpoint !== 'object') return null;
      const item = checkpoint as {
        stage?: unknown;
        status?: unknown;
        attempt?: unknown;
        createdAt?: unknown;
        data?: unknown;
      };
      if (typeof item.stage !== 'string') return null;
      return {
        stage: item.stage,
        status: typeof item.status === 'string' ? item.status : '-',
        attempt: Number(item.attempt ?? 1),
        createdAt: typeof item.createdAt === 'string' ? item.createdAt : undefined,
        data: objectRecord(item.data),
      };
    })
    .filter((artifact): artifact is StageArtifact => Boolean(artifact))
    .sort((a, b) => {
      const attemptDelta = a.attempt - b.attempt;
      if (attemptDelta !== 0) return attemptDelta;
      return timeValue(a.createdAt) - timeValue(b.createdAt);
    });
}

function getJobTiming(job: CloudCodegenJob): TimingBreakdown | null {
  const outputTiming = objectRecord(job.output?.timing);
  const metadata = objectRecord(job.output?.metadata);
  const metadataTiming = objectRecord(metadata.timing);
  const checkpointTiming = getTimingFromCheckpoints(job);
  const timing = { ...checkpointTiming, ...metadataTiming, ...outputTiming };
  const hasTiming = Object.values(timing).some((value) => typeof value === 'number');
  return hasTiming ? normalizeTiming(timing) : null;
}

function getTimingFromCheckpoints(job: CloudCodegenJob): TimingBreakdown {
  const checkpoints = Array.isArray(job.output?.checkpoints) ? job.output.checkpoints : [];
  const timing: TimingBreakdown = {};
  for (const checkpoint of checkpoints) {
    if (!checkpoint || typeof checkpoint !== 'object') continue;
    const item = checkpoint as { stage?: unknown; data?: unknown };
    const data = objectRecord(item.data);
    const durationMs = Number(data.durationMs);
    if (!Number.isFinite(durationMs)) continue;
    if (item.stage === 'planning') timing.planningMs = durationMs;
    if (item.stage === 'chunk') timing.chunkMs = (timing.chunkMs ?? 0) + durationMs;
    if (item.stage === 'assembly') timing.assemblyMs = durationMs;
    if (item.stage === 'quality_check') timing.qualityCheckMs = durationMs;
    if (item.stage === 'repair') timing.repairMs = (timing.repairMs ?? 0) + durationMs;
  }
  return timing;
}

function normalizeTiming(value: Record<string, unknown>): TimingBreakdown {
  return {
    planningMs: numberValue(value.planningMs),
    chunkMs: numberValue(value.chunkMs),
    assemblyMs: numberValue(value.assemblyMs),
    qualityCheckMs: numberValue(value.qualityCheckMs),
    repairMs: numberValue(value.repairMs),
    providerMs: numberValue(value.providerMs),
    providerCallTotalMs: numberValue(value.providerCallTotalMs),
    providerCallCount: numberValue(value.providerCallCount),
    totalMs: numberValue(value.totalMs),
  };
}

function numberValue(value: unknown): number | undefined {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : undefined;
}

function sumTiming(timing: TimingBreakdown): number {
  return (
    (timing.planningMs ?? 0) +
    (timing.chunkMs ?? 0) +
    (timing.assemblyMs ?? 0) +
    (timing.qualityCheckMs ?? 0) +
    (timing.repairMs ?? 0)
  );
}

function getTimingInsights(
  timing: TimingBreakdown,
  totalMs: number,
  t: ReturnType<typeof useTranslation>['t'],
): TimingInsight[] {
  const insights: TimingInsight[] = [];
  if (totalMs <= 0) return insights;

  const providerMs = timing.providerMs ?? 0;
  const providerPercent = Math.round((providerMs / totalMs) * 100);
  if (providerMs > 0 && providerPercent >= 70) {
    insights.push({
      key: 'provider',
      message: t('tasks.timingInsight.providerDominant', { percent: providerPercent }),
    });
  }

  const repairMs = timing.repairMs ?? 0;
  if (repairMs >= 2000) {
    insights.push({
      key: 'repair',
      message: t('tasks.timingInsight.repairSlow', { duration: formatDuration(repairMs) }),
    });
  }

  const stageRows = [
    { key: 'planningMs', label: t('tasks.timing.planning'), value: timing.planningMs ?? 0 },
    { key: 'chunkMs', label: t('tasks.timing.chunk'), value: timing.chunkMs ?? 0 },
    { key: 'assemblyMs', label: t('tasks.timing.assembly'), value: timing.assemblyMs ?? 0 },
    {
      key: 'qualityCheckMs',
      label: t('tasks.timing.qualityCheck'),
      value: timing.qualityCheckMs ?? 0,
    },
    { key: 'repairMs', label: t('tasks.timing.repair'), value: repairMs },
  ].filter((row) => row.value > 0);
  const slowest = stageRows.sort((a, b) => b.value - a.value)[0];
  if (slowest && slowest.value >= 1000) {
    insights.push({
      key: `slowest-${slowest.key}`,
      message: t('tasks.timingInsight.slowestStage', {
        stage: slowest.label,
        duration: formatDuration(slowest.value),
      }),
    });
  }

  return insights.slice(0, 3);
}

function describeStageArtifact(
  artifact: StageArtifact,
  t: ReturnType<typeof useTranslation>['t'],
): string | null {
  if (artifact.stage === 'chunk') {
    const chunkId = typeof artifact.data.chunkId === 'string' ? artifact.data.chunkId : null;
    return chunkId ? t('tasks.stageArtifact.chunk', { chunkId }) : null;
  }
  if (artifact.stage === 'quality_check') {
    const report = objectRecord(artifact.data.report);
    const summary = objectRecord(report.summary);
    const status = typeof report.status === 'string' ? report.status : artifact.status;
    return t('tasks.stageArtifact.quality', {
      status,
      errors: Number(summary.errorCount ?? 0),
      warnings: Number(summary.warningCount ?? 0),
    });
  }
  if (artifact.stage === 'repair') {
    const attempt = Number(artifact.data.attempt ?? artifact.attempt);
    return t('tasks.stageArtifact.repair', { attempt });
  }
  if (artifact.stage === 'assembly') {
    const files = Array.isArray(artifact.data.files) ? artifact.data.files.length : 0;
    return t('tasks.stageArtifact.assembly', { files });
  }
  if (artifact.stage === 'planning') {
    const plan = objectRecord(artifact.data.plan);
    const chunks = Array.isArray(plan.chunks) ? plan.chunks.length : 0;
    return t('tasks.stageArtifact.planning', { chunks });
  }
  return null;
}

function getArtifactQualityReport(artifact: StageArtifact): QualityReportPreview | null {
  const report =
    artifact.stage === 'quality_check'
      ? objectRecord(artifact.data.report)
      : artifact.stage === 'repair'
        ? objectRecord(artifact.data.report)
        : {};
  if (!report.status || !report.summary) return null;
  const summary = objectRecord(report.summary);
  const issues = Array.isArray(report.issues) ? report.issues : [];
  return {
    status: typeof report.status === 'string' ? report.status : artifact.status,
    framework: typeof report.framework === 'string' ? report.framework : '-',
    checkedAt: typeof report.checkedAt === 'string' ? report.checkedAt : undefined,
    summary: {
      fileCount: Number(summary.fileCount ?? 0),
      errorCount: Number(summary.errorCount ?? 0),
      warningCount: Number(summary.warningCount ?? 0),
      missingTextCount: Number(summary.missingTextCount ?? 0),
      missingAssetCount: Number(summary.missingAssetCount ?? 0),
    },
    issues: parseQualityIssues(issues),
  };
}

function getArtifactRepairAttempt(artifact: StageArtifact): RepairAttemptPreview | null {
  if (artifact.stage !== 'repair') return null;
  const issues = Array.isArray(artifact.data.issues) ? artifact.data.issues : [];
  const attempt = Number(artifact.data.attempt ?? artifact.attempt);
  const durationMs = Number(artifact.data.durationMs);
  const error = typeof artifact.data.error === 'string' ? artifact.data.error : undefined;
  if (issues.length === 0 && !error) return null;
  return {
    attempt,
    durationMs: Number.isFinite(durationMs) ? durationMs : undefined,
    error,
    issues: parseQualityIssues(issues),
  };
}

function parseQualityIssues(issues: unknown[]): QualityIssue[] {
  return issues
    .map((issue): QualityIssue | null => {
      if (!issue || typeof issue !== 'object') return null;
      const item = issue as Record<string, unknown>;
      return {
        code: typeof item.code === 'string' ? item.code : 'unknown',
        severity: typeof item.severity === 'string' ? item.severity : '-',
        message: typeof item.message === 'string' ? item.message : '',
        filePath: typeof item.filePath === 'string' ? item.filePath : undefined,
      };
    })
    .filter((issue): issue is QualityIssue => Boolean(issue));
}

function objectRecord(value: unknown): Record<string, unknown> {
  return value && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
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
