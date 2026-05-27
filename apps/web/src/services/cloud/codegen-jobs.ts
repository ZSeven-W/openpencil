import type {
  CloudCodegenJob,
  CodegenJobFailureType,
  CodegenProviderConfigAudit,
  CodegenProviderConfig,
  CodegenQueueAccess,
  CodegenWorkerRuntimeStats,
  CodegenWorkerOverview,
  CreateCloudCodegenJobInput,
  RerunCloudCodegenJobStepInput,
  ResumeCloudCodegenJobInput,
  CloudPagedResult,
  TaskNotification,
} from '@/types/cloud';
import { cloudFetch } from './cloud-fetch';

interface CloudReadOptions {
  force?: boolean;
}

function cloudRead<T>(path: string, options?: CloudReadOptions): Promise<T> {
  return options?.force ? cloudFetch<T>(path, { force: true }) : cloudFetch<T>(path);
}

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
  offset?: number;
} = {}, options: CloudReadOptions = {}): Promise<CloudCodegenJob[]> {
  const res = await listCloudCodegenJobsPage(input, options);
  return res.data;
}

export async function listCloudCodegenJobsPage(input: {
  fileId?: string;
  status?: CloudCodegenJob['status'];
  active?: boolean;
  deadLettered?: boolean;
  limit?: number;
  offset?: number;
} = {}, options: CloudReadOptions = {}): Promise<CloudPagedResult<CloudCodegenJob>> {
  const params = new URLSearchParams();
  if (input.fileId) params.set('fileId', input.fileId);
  if (input.status) params.set('status', input.status);
  if (input.active !== undefined) params.set('active', String(input.active));
  if (input.deadLettered !== undefined) params.set('deadLettered', String(input.deadLettered));
  if (input.limit) params.set('limit', String(input.limit));
  if (input.offset !== undefined) params.set('offset', String(input.offset));
  const query = params.toString();
  const res = await cloudRead<CloudPagedResult<CloudCodegenJob>>(
    `/api/cloud/codegen-jobs${query ? `?${query}` : ''}`,
    options,
  );
  return {
    data: res.data,
    page: res.page ?? {
      total: res.data.length,
      limit: input.limit ?? res.data.length,
      offset: input.offset ?? 0,
    },
  };
}

export async function getCloudCodegenJob(
  jobId: string,
  options: CloudReadOptions = {},
): Promise<CloudCodegenJob> {
  const res = await cloudRead<{ data: CloudCodegenJob }>(
    `/api/cloud/codegen-jobs/${encodeURIComponent(jobId)}`,
    options,
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

export async function deleteCloudCodegenJob(jobId: string): Promise<void> {
  await cloudFetch(`/api/cloud/codegen-jobs/${encodeURIComponent(jobId)}`, {
    method: 'DELETE',
  });
}

export async function retryCloudCodegenJob(jobId: string): Promise<CloudCodegenJob> {
  const res = await cloudFetch<{ data: CloudCodegenJob }>(
    `/api/cloud/codegen-jobs/${encodeURIComponent(jobId)}/retry`,
    { method: 'POST' },
  );
  return res.data;
}

export async function resumeCloudCodegenJob(
  jobId: string,
  input: ResumeCloudCodegenJobInput = {},
): Promise<CloudCodegenJob> {
  const res = await cloudFetch<{ data: CloudCodegenJob }>(
    `/api/cloud/codegen-jobs/${encodeURIComponent(jobId)}/resume`,
    {
      method: 'POST',
      body: JSON.stringify(input),
    },
  );
  return res.data;
}

export async function rerunCloudCodegenJobStep(
  jobId: string,
  input: RerunCloudCodegenJobStepInput,
): Promise<CloudCodegenJob> {
  const res = await cloudFetch<{ data: CloudCodegenJob }>(
    `/api/cloud/codegen-jobs/${encodeURIComponent(jobId)}/rerun-step`,
    {
      method: 'POST',
      body: JSON.stringify(input),
    },
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
  summary?: boolean;
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
  options: CloudReadOptions = {},
): Promise<CodegenWorkerOverview> {
  const res = await cloudRead<{ data: CodegenWorkerOverview }>(
    `/api/cloud/codegen-workers${queryFromRecord(input)}`,
    options,
  );
  return res.data;
}

export async function getCloudCodegenQueueAccess(
  options: CloudReadOptions = {},
): Promise<CodegenQueueAccess> {
  const res = await cloudRead<{ data: CodegenQueueAccess }>(
    '/api/cloud/codegen-queue-access',
    options,
  );
  return res.data;
}

export async function getCloudCodegenWorkerStats(input: {
  days?: number;
} = {}, options: CloudReadOptions = {}): Promise<CodegenWorkerRuntimeStats> {
  const res = await cloudRead<{ data: CodegenWorkerRuntimeStats }>(
    `/api/cloud/codegen-worker-stats${queryFromRecord(input)}`,
    options,
  );
  return res.data;
}

export async function listCloudCodegenProviderConfigs(input: {
  provider?: string;
  limit?: number;
  offset?: number;
} = {}, options: CloudReadOptions = {}): Promise<CodegenProviderConfig[]> {
  const res = await cloudRead<{ data: CodegenProviderConfig[] }>(
    `/api/cloud/codegen-provider-configs${queryFromRecord(input)}`,
    options,
  );
  return res.data;
}

export async function listCloudCodegenProviderConfigAudits(input: {
  provider?: string;
  limit?: number;
  offset?: number;
} = {}, options: CloudReadOptions = {}): Promise<{
  data: CodegenProviderConfigAudit[];
  page: { total: number; limit: number; offset: number };
}> {
  return cloudRead<{
    data: CodegenProviderConfigAudit[];
    page: { total: number; limit: number; offset: number };
  }>(`/api/cloud/codegen-provider-config-audits${queryFromRecord(input)}`, options);
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

export async function listTaskNotifications(
  input: { fileId?: string; limit?: number; offset?: number } = {},
  options: CloudReadOptions = {},
): Promise<TaskNotification[]> {
  const res = await listTaskNotificationsPage(input, options);
  return res.data;
}

export async function listTaskNotificationsPage(
  input: { fileId?: string; limit?: number; offset?: number } = {},
  options: CloudReadOptions = {},
): Promise<CloudPagedResult<TaskNotification>> {
  const query = queryFromRecord(input);
  const res = await cloudRead<CloudPagedResult<TaskNotification>>(
    `/api/cloud/task-notifications${query}`,
    options,
  );
  return {
    data: res.data,
    page: res.page ?? {
      total: res.data.length,
      limit: input.limit ?? res.data.length,
      offset: input.offset ?? 0,
    },
  };
}

export async function markTaskNotificationRead(id: string): Promise<TaskNotification> {
  const res = await cloudFetch<{ data: TaskNotification }>(
    `/api/cloud/task-notifications/${encodeURIComponent(id)}/read`,
    { method: 'POST' },
  );
  return res.data;
}

export async function deleteTaskNotification(id: string): Promise<void> {
  await cloudFetch(`/api/cloud/task-notifications/${encodeURIComponent(id)}`, {
    method: 'DELETE',
  });
}
