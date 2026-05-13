import type {
  CloudCodegenJob,
  CodegenJobFailureType,
  CodegenProviderConfigAudit,
  CodegenProviderConfig,
  CodegenQueueAccess,
  CodegenWorkerRuntimeStats,
  CodegenWorkerOverview,
  CreateCloudCodegenJobInput,
  TaskNotification,
} from '@/types/cloud';
import { cloudFetch } from './cloud-fetch';

export async function createCloudCodegenJob(
  input: CreateCloudCodegenJobInput,
): Promise<CloudCodegenJob> {
  const res = await cloudFetch<{ data: CloudCodegenJob }>('/api/cloud/codegen-jobs', {
    method: 'POST',
    body: JSON.stringify(input),
  });
  return res.data;
}

export async function listCloudCodegenJobs(input: {
  fileId?: string;
  status?: CloudCodegenJob['status'];
  active?: boolean;
  deadLettered?: boolean;
  limit?: number;
} = {}): Promise<CloudCodegenJob[]> {
  const params = new URLSearchParams();
  if (input.fileId) params.set('fileId', input.fileId);
  if (input.status) params.set('status', input.status);
  if (input.active !== undefined) params.set('active', String(input.active));
  if (input.deadLettered !== undefined) params.set('deadLettered', String(input.deadLettered));
  if (input.limit) params.set('limit', String(input.limit));
  const query = params.toString();
  const res = await cloudFetch<{ data: CloudCodegenJob[] }>(
    `/api/cloud/codegen-jobs${query ? `?${query}` : ''}`,
  );
  return res.data;
}

export async function getCloudCodegenJob(jobId: string): Promise<CloudCodegenJob> {
  const res = await cloudFetch<{ data: CloudCodegenJob }>(
    `/api/cloud/codegen-jobs/${encodeURIComponent(jobId)}`,
  );
  return res.data;
}

export async function batchCancelCloudCodegenJobs(jobIds: string[]): Promise<CloudCodegenJob[]> {
  const res = await cloudFetch<{ data: CloudCodegenJob[] }>(
    '/api/cloud/codegen-jobs/batch-cancel',
    {
      method: 'POST',
      body: JSON.stringify({ jobIds }),
    },
  );
  return res.data;
}

export async function cancelCloudCodegenJob(jobId: string): Promise<CloudCodegenJob> {
  const res = await cloudFetch<{ data: CloudCodegenJob }>(
    `/api/cloud/codegen-jobs/${encodeURIComponent(jobId)}/cancel`,
    { method: 'POST' },
  );
  return res.data;
}

export async function retryCloudCodegenJob(jobId: string): Promise<CloudCodegenJob> {
  const res = await cloudFetch<{ data: CloudCodegenJob }>(
    `/api/cloud/codegen-jobs/${encodeURIComponent(jobId)}/retry`,
    { method: 'POST' },
  );
  return res.data;
}

export interface ReplayFailedCloudCodegenJobsInput {
  jobIds?: string[];
  provider?: string;
  failureType?: CodegenJobFailureType;
  deadLetteredFrom?: string;
  deadLetteredTo?: string;
  limit?: number;
}

export interface CloudCodegenWorkerOverviewFilters {
  workerLimit?: number;
  workerOffset?: number;
  providerLimit?: number;
  providerOffset?: number;
  failedLimit?: number;
  failedOffset?: number;
  provider?: string;
  failureType?: CodegenJobFailureType;
  deadLetteredFrom?: string;
  deadLetteredTo?: string;
}

export async function updateCloudCodegenJobPriority(
  jobId: string,
  priority: number,
): Promise<CloudCodegenJob> {
  const res = await cloudFetch<{ data: CloudCodegenJob }>(
    `/api/cloud/codegen-jobs/${encodeURIComponent(jobId)}/priority`,
    {
      method: 'PATCH',
      body: JSON.stringify({ priority }),
    },
  );
  return res.data;
}

export async function replayFailedCloudCodegenJobs(
  input: ReplayFailedCloudCodegenJobsInput,
): Promise<CloudCodegenJob[]> {
  const res = await cloudFetch<{ data: CloudCodegenJob[] }>(
    '/api/cloud/codegen-jobs/replay-failed',
    {
      method: 'POST',
      body: JSON.stringify(input),
    },
  );
  return res.data;
}

function queryFromRecord(input: object) {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(input)) {
    if (value !== undefined) params.set(key, String(value));
  }
  const query = params.toString();
  return query ? `?${query}` : '';
}

export async function getCloudCodegenWorkerOverview(
  input: CloudCodegenWorkerOverviewFilters = {},
): Promise<CodegenWorkerOverview> {
  const res = await cloudFetch<{ data: CodegenWorkerOverview }>(
    `/api/cloud/codegen-workers${queryFromRecord(input)}`,
  );
  return res.data;
}

export async function getCloudCodegenQueueAccess(): Promise<CodegenQueueAccess> {
  const res = await cloudFetch<{ data: CodegenQueueAccess }>('/api/cloud/codegen-queue-access');
  return res.data;
}

export async function getCloudCodegenWorkerStats(input: {
  days?: number;
} = {}): Promise<CodegenWorkerRuntimeStats> {
  const res = await cloudFetch<{ data: CodegenWorkerRuntimeStats }>(
    `/api/cloud/codegen-worker-stats${queryFromRecord(input)}`,
  );
  return res.data;
}

export async function listCloudCodegenProviderConfigs(input: {
  provider?: string;
  limit?: number;
  offset?: number;
} = {}): Promise<CodegenProviderConfig[]> {
  const res = await cloudFetch<{ data: CodegenProviderConfig[] }>(
    `/api/cloud/codegen-provider-configs${queryFromRecord(input)}`,
  );
  return res.data;
}

export async function listCloudCodegenProviderConfigAudits(input: {
  provider?: string;
  limit?: number;
  offset?: number;
} = {}): Promise<{
  data: CodegenProviderConfigAudit[];
  page: { total: number; limit: number; offset: number };
}> {
  return cloudFetch<{
    data: CodegenProviderConfigAudit[];
    page: { total: number; limit: number; offset: number };
  }>(`/api/cloud/codegen-provider-config-audits${queryFromRecord(input)}`);
}

export async function updateCloudCodegenProviderConfig(
  provider: string,
  input: Pick<
    CodegenProviderConfig,
    'enabled' | 'maxPerMinute' | 'circuitThreshold' | 'circuitOpenMs'
  > & { reason?: string },
): Promise<CodegenProviderConfig> {
  const res = await cloudFetch<{ data: CodegenProviderConfig }>(
    `/api/cloud/codegen-provider-configs/${encodeURIComponent(provider)}`,
    {
      method: 'PATCH',
      body: JSON.stringify(input),
    },
  );
  return res.data;
}

export async function listTaskNotifications(): Promise<TaskNotification[]> {
  const res = await cloudFetch<{ data: TaskNotification[] }>('/api/cloud/task-notifications');
  return res.data;
}

export async function markTaskNotificationRead(id: string): Promise<TaskNotification> {
  const res = await cloudFetch<{ data: TaskNotification }>(
    `/api/cloud/task-notifications/${encodeURIComponent(id)}/read`,
    { method: 'POST' },
  );
  return res.data;
}
