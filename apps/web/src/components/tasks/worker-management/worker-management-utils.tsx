import { useId } from 'react';
import { Activity, type LucideIcon } from 'lucide-react';
import type { useTranslation } from 'react-i18next';
import { Card, CardContent } from '@/components/ui/card';
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from '@/components/ui/empty';
import { Field, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import type {
  CloudCodegenJob,
  CodegenJobFailureType,
  CodegenQueuePage,
} from '@/types/cloud';

export type PageKind = 'workers' | 'providers' | 'failedJobs';
export type WorkerView = 'workers' | 'providers' | 'audit';

export const PAGE_SIZE = 10;
export const FAILURE_OPTIONS: Array<'all' | CodegenJobFailureType> = [
  'all',
  'rate_limit',
  'timeout',
  'provider_error',
  'execution_error',
  'canceled',
];

export function formatPercent(value?: number): string {
  return `${Math.round(Number(value ?? 0) * 100)}%`;
}

export function formatDuration(ms?: number): string {
  const value = Number(ms ?? 0);
  if (!Number.isFinite(value) || value <= 0) return '0 ms';
  if (value < 1000) return `${Math.round(value)} ms`;
  return `${(value / 1000).toFixed(1)} s`;
}

export function formatTime(value?: string | null): string {
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

export function isCircuitOpen(value?: string | null): boolean {
  const timestamp = value ? new Date(value).getTime() : 0;
  return Number.isFinite(timestamp) && timestamp > Date.now();
}

export function pageInfo(page?: CodegenQueuePage) {
  return {
    total: page?.total ?? 0,
    limit: page?.limit ?? PAGE_SIZE,
    offset: page?.offset ?? 0,
  };
}

export function toInputDate(value?: string) {
  if (!value) return undefined;
  return new Date(value).toISOString();
}

export function getReplayableJobs(
  jobs: CloudCodegenJob[],
  overviewJobs: CloudCodegenJob[],
) {
  const merged = new Map<string, CloudCodegenJob>();
  for (const job of [...overviewJobs, ...jobs]) {
    if (job.status === 'failed' || job.deadLetteredAt) merged.set(job.id, job);
  }
  return Array.from(merged.values());
}

export function NumberField({
  label,
  value,
  onChange,
}: {
  label: string;
  value: number;
  onChange: (value: number) => void;
}) {
  const id = useId();
  return (
    <Field className="gap-1">
      <FieldLabel htmlFor={id} className="text-xs text-muted-foreground">{label}</FieldLabel>
      <Input
        id={id}
        type="number"
        className="h-8 text-xs"
        value={value}
        onChange={(event) => onChange(Number(event.target.value))}
      />
    </Field>
  );
}

export function QueueMetricCard({
  icon: Icon,
  label,
  value,
  caption,
}: {
  icon: LucideIcon;
  label: string;
  value: string | number;
  caption?: string;
}) {
  return (
    <Card className="rounded py-2.5 shadow-none">
      <CardContent className="px-3">
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <Icon />
          {label}
        </div>
        <p className="mt-1 text-xl font-semibold tabular-nums">{value}</p>
        {caption && <p className="mt-1 text-xs text-muted-foreground">{caption}</p>}
      </CardContent>
    </Card>
  );
}

export function EmptyPanel({
  icon: Icon = Activity,
  title,
  description,
}: {
  icon?: LucideIcon;
  title: string;
  description: string;
}) {
  return (
    <Empty className="rounded border border-border bg-background">
      <EmptyHeader>
        <EmptyMedia variant="icon">
          <Icon />
        </EmptyMedia>
        <EmptyTitle>{title}</EmptyTitle>
        <EmptyDescription>{description}</EmptyDescription>
      </EmptyHeader>
    </Empty>
  );
}

export type TFunction = ReturnType<typeof useTranslation>['t'];
