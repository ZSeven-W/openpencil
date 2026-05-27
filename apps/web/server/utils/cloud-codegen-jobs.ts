import type { SupabaseClient } from '@supabase/supabase-js';
import { createError } from 'h3';
import type {
  ChunkResult,
  CodegenCheckpoint,
  CodegenPipelineMode,
  CodePlanFromAI,
  CodegenResumeMode,
  CodegenQualityReport,
  CodegenRepairAttempt,
  CodegenTimingBreakdown,
  ChunkStatus,
} from '@zseven-w/pen-types';
import type {
  CloudCodegenJob,
  CloudPagedResult,
  CodegenJobAgentRole,
  CodegenJobStatus,
  CreateCloudCodegenJobInput,
  TaskNotification,
  SaveCodegenFileInput,
} from '../../src/types/cloud';
import { mapCodegenJob, mapTaskNotification } from './cloud-file-mappers';
import { assertCloudFileReadable } from './cloud-file-management';
import { recordCloudActivity } from './cloud-activity-events';
import {
  createCodeGenerationRecord,
  getCodeGenerationDetail,
} from './cloud-code-generation-records';
import { generateCodePatch } from './cloud-codegen-patch';
import {
  generateCode,
  hasCodegenQualityErrors,
  resumeCodegenFromChunkCheckpoint,
  runCodegenQualityGate,
  runCodegenRepairPass,
  type CodegenTextCollector,
} from '../../src/services/ai/code-generation-pipeline';
import { buildCodegenFiles } from '../../src/services/ai/codegen-files';
import type { CodegenDesignIR } from '../../src/services/ai/codegen-design-ir';
import type { PenNode } from '../../src/types/pen';
import type { CodegenAssetFile } from '../../src/services/ai/codegen-assets';
import { getCloudServiceSupabase } from './cloud-supabase';
import { toCodegenDbError } from './cloud-codegen-errors';
import {
  claimNextCodegenJob,
  classifyCodegenJobFailure,
  heartbeatCodegenJob,
  isCodegenJobCanceled,
  recordCodegenJobDeadLetter,
  assertProviderCanRunCodegen,
  getProviderCodegenFailurePolicy,
  recordProviderCodegenFailure,
  recordProviderCodegenSuccess,
  recordCodegenJobRetry,
  requeueStaleCodegenJobs,
  resolveEmbeddedCodegenWorkerMode,
  resolveCodegenFailureDisposition,
  resolveCodegenWorkerOptions,
  type CodegenWorkerRuntimeOptions,
} from './cloud-codegen-job-queue';
export {
  getCodegenQueueAccess,
  getCodegenWorkerRuntimeStats,
  getCodegenWorkerOverview,
  listCodegenProviderConfigAudits,
  listCodegenProviderConfigs,
  replayFailedCodegenJobs,
  updateCodegenJobPriority,
  updateCodegenProviderConfig,
} from './cloud-codegen-worker-management';

function bytesToBase64(bytes: Uint8Array): string {
  return Buffer.from(bytes).toString('base64');
}

const JOB_DETAIL_SELECT =
  'id,file_id,owner_id,generation_id,file_name,page_name,pipeline_mode,job_kind,status,framework,page_id,target_kind,node_ids,target_hash,document_revision,provider,model,priority,progress,attempts,max_attempts,locked_by,locked_until,last_heartbeat_at,next_run_at,dead_lettered_at,last_error,failure_type,input_snapshot,output,error,created_at,updated_at,started_at,completed_at,canceled_at';
const JOB_SUMMARY_SELECT =
  'id,file_id,owner_id,generation_id,file_name,page_name,pipeline_mode,job_kind,status,framework,page_id,target_kind,node_ids,target_hash,document_revision,provider,model,priority,progress,attempts,max_attempts,locked_by,locked_until,last_heartbeat_at,next_run_at,dead_lettered_at,last_error,failure_type,error,created_at,updated_at,started_at,completed_at,canceled_at';
const STEP_SELECT =
  'id,job_id,agent_role,attempt,status,progress,input,output,error,started_at,completed_at,created_at,updated_at';
const EVENT_SELECT = 'id,job_id,type,message,metadata,created_at';
const NOTIFICATION_SELECT =
  'id,owner_id,job_id,file_id,generation_id,kind,title,message,read_at,created_at';
const JOB_DETAIL_STEP_LIMIT = 120;
const JOB_DETAIL_EVENT_LIMIT = 100;

export type ServerCodegenProvider = 'anthropic' | 'openai' | 'gemini';

export interface CloudCodegenJobContext {
  supabase: SupabaseClient;
  userId: string;
  userEmail?: string | null;
}

function isServerProvider(provider: string): provider is ServerCodegenProvider {
  return ['anthropic', 'openai', 'gemini'].includes(provider);
}

function triggerCodegenWorker() {
  if (!resolveEmbeddedCodegenWorkerMode().enabled) return;
  const serviceSupabase = getCloudServiceSupabase();
  if (!serviceSupabase) return;
  void runCodegenWorkerOnce(serviceSupabase).catch((err) => {
    console.error('[codegen-job-worker] run failed', err);
  });
}

async function loadJobParts(input: { supabase: SupabaseClient; jobId: string }) {
  const [steps, events] = await Promise.all([
    input.supabase
      .from('codegen_job_steps')
      .select(STEP_SELECT)
      .eq('job_id', input.jobId)
      .order('created_at', { ascending: true })
      .limit(JOB_DETAIL_STEP_LIMIT),
    input.supabase
      .from('codegen_job_events')
      .select(EVENT_SELECT)
      .eq('job_id', input.jobId)
      .order('created_at', { ascending: false })
      .limit(JOB_DETAIL_EVENT_LIMIT),
  ]);
  if (steps.error) throw toCodegenDbError(steps.error, 'Failed to load codegen job steps');
  if (events.error) throw toCodegenDbError(events.error, 'Failed to load codegen job events');
  return {
    steps: steps.data ?? [],
    events: [...(events.data ?? [])].sort((a, b) => a.created_at.localeCompare(b.created_at)),
  };
}

export async function getCodegenJobDetail(input: {
  supabase: SupabaseClient;
  userId: string;
  jobId: string;
}): Promise<CloudCodegenJob> {
  const job = await input.supabase
    .from('codegen_jobs')
    .select(JOB_DETAIL_SELECT)
    .eq('id', input.jobId)
    .eq('owner_id', input.userId)
    .single();

  if (job.error || !job.data) {
    throw createError({ statusCode: 404, statusMessage: 'Codegen job not found' });
  }

  return mapCodegenJob(job.data, await loadJobParts(input));
}

export async function deleteCodegenJob(input: {
  supabase: SupabaseClient;
  userId: string;
  jobId: string;
}): Promise<void> {
  await getCodegenJobDetail(input);

  const deleted = await input.supabase
    .from('codegen_jobs')
    .delete()
    .eq('id', input.jobId)
    .eq('owner_id', input.userId);

  if (deleted.error) throw toCodegenDbError(deleted.error, 'Failed to delete codegen job');
}

export async function listCodegenJobs(input: {
  supabase: SupabaseClient;
  userId: string;
  fileId?: string;
  status?: CodegenJobStatus;
  active?: boolean;
  deadLettered?: boolean;
  limit: number;
  offset?: number;
}): Promise<CloudPagedResult<CloudCodegenJob>> {
  let query = input.supabase
    .from('codegen_jobs')
    .select(JOB_SUMMARY_SELECT, { count: 'exact' })
    .eq('owner_id', input.userId);

  if (input.fileId) query = query.eq('file_id', input.fileId);
  if (input.status) query = query.eq('status', input.status);
  if (input.active) query = query.in('status', ['pending', 'running']);
  if (input.deadLettered === true) query = query.not('dead_lettered_at', 'is', null);
  if (input.deadLettered === false) query = query.is('dead_lettered_at', null);

  const jobs = await query
    .order('created_at', { ascending: false })
    .range(input.offset ?? 0, (input.offset ?? 0) + input.limit - 1);
  if (jobs.error) throw toCodegenDbError(jobs.error, 'Failed to list codegen jobs');
  return {
    data: (jobs.data ?? []).map((row) => mapCodegenJob(row)),
    page: {
      total: jobs.count ?? 0,
      limit: input.limit,
      offset: input.offset ?? 0,
    },
  };
}

async function addJobEvent(input: {
  supabase: SupabaseClient;
  jobId: string;
  type: string;
  message?: string | null;
  metadata?: Record<string, unknown>;
}) {
  const inserted = await input.supabase.from('codegen_job_events').insert({
    job_id: input.jobId,
    type: input.type,
    message: input.message ?? null,
    metadata: input.metadata ?? {},
  });
  if (inserted.error) throw toCodegenDbError(inserted.error, 'Failed to create codegen job event');
}

function isTerminalStepStatus(status?: CodegenJobStatus) {
  return status === 'succeeded' || status === 'failed' || status === 'canceled';
}

const STEP_STATUS_RANK: Record<CodegenJobStatus, number> = {
  pending: 0,
  running: 1,
  canceled: 2,
  failed: 2,
  succeeded: 3,
};

function normalizeStepStatus(status: unknown): CodegenJobStatus | null {
  return status === 'pending' ||
    status === 'running' ||
    status === 'succeeded' ||
    status === 'failed' ||
    status === 'canceled'
    ? status
    : null;
}

export async function upsertCodegenJobStep(input: {
  supabase: SupabaseClient;
  jobId: string;
  attempt: number;
  agentRole: CodegenJobAgentRole;
  status?: CodegenJobStatus;
  progress?: number;
  data?: Record<string, unknown>;
  output?: Record<string, unknown>;
  error?: string | null;
}) {
  const now = new Date().toISOString();
  const attempt = Math.max(1, Math.trunc(Number(input.attempt || 1)));
  const status = input.status ?? 'pending';
  const terminal = isTerminalStepStatus(status);
  const progress = input.progress ?? (terminal ? 100 : 0);
  const existing = await input.supabase
    .from('codegen_job_steps')
    .select('id,status,progress,started_at,completed_at,input,output,error')
    .eq('job_id', input.jobId)
    .eq('agent_role', input.agentRole)
    .eq('attempt', attempt)
    .maybeSingle();

  if (existing.error) {
    throw toCodegenDbError(existing.error, 'Failed to load codegen job step');
  }

  const existingInput =
    existing.data?.input && typeof existing.data.input === 'object'
      ? (existing.data.input as Record<string, unknown>)
      : {};
  const existingOutput =
    existing.data?.output && typeof existing.data.output === 'object'
      ? (existing.data.output as Record<string, unknown>)
      : {};
  const existingStatus = normalizeStepStatus(existing.data?.status);
  const existingProgress =
    typeof existing.data?.progress === 'number' && Number.isFinite(existing.data.progress)
      ? existing.data.progress
      : undefined;
  const shouldPreserveExistingStatus =
    existingStatus !== null && STEP_STATUS_RANK[existingStatus] > STEP_STATUS_RANK[status];
  const finalStatus = shouldPreserveExistingStatus ? existingStatus : status;
  const finalTerminal = isTerminalStepStatus(finalStatus);
  const finalProgress = shouldPreserveExistingStatus
    ? Math.max(existingProgress ?? 0, progress)
    : progress;
  const base = {
    job_id: input.jobId,
    attempt,
    agent_role: input.agentRole,
    status: finalStatus,
    progress: finalProgress,
    error: shouldPreserveExistingStatus ? existing.data?.error ?? null : input.error ?? null,
    input: input.data ? { ...existingInput, ...input.data } : existingInput,
    output: input.output ? { ...existingOutput, ...input.output } : existingOutput,
    started_at:
      existing.data?.started_at ?? (finalStatus === 'running' || finalTerminal ? now : null),
    completed_at:
      finalTerminal
        ? existing.data?.completed_at ?? now
        : finalStatus === 'running'
          ? null
          : existing.data?.completed_at ?? null,
    updated_at: now,
  };

  if (existing.data?.id) {
    const updated = await input.supabase
      .from('codegen_job_steps')
      .update(base)
      .eq('id', existing.data.id);
    if (updated.error) throw toCodegenDbError(updated.error, 'Failed to update codegen job step');
    return;
  }

  const inserted = await input.supabase.from('codegen_job_steps').insert({
    ...base,
    created_at: now,
  });
  if (inserted.error) throw toCodegenDbError(inserted.error, 'Failed to create codegen job step');
}

async function updateJob(input: {
  supabase: SupabaseClient;
  jobId: string;
  patch: Record<string, unknown>;
  lockedBy?: string | null;
}) {
  let query = input.supabase
    .from('codegen_jobs')
    .update({ ...input.patch, updated_at: new Date().toISOString() })
    .eq('id', input.jobId);
  if (input.lockedBy !== undefined) query = query.eq('locked_by', input.lockedBy);
  const updated = await query
    .select(JOB_DETAIL_SELECT)
    .single();
  if (updated.error || !updated.data) {
    throw updated.error
      ? toCodegenDbError(updated.error, 'Failed to update codegen job')
      : createError({ statusCode: 500, statusMessage: 'Failed to update codegen job' });
  }
  return updated.data;
}

async function createTaskNotification(input: {
  supabase: SupabaseClient;
  ownerId: string;
  jobId: string;
  fileId: string;
  generationId?: string | null;
  kind: 'codegen_job_succeeded' | 'codegen_job_failed' | 'codegen_job_canceled';
  title: string;
  message?: string | null;
}) {
  const inserted = await input.supabase
    .from('task_notifications')
    .insert({
      owner_id: input.ownerId,
      job_id: input.jobId,
      file_id: input.fileId,
      generation_id: input.generationId ?? null,
      kind: input.kind,
      title: input.title,
      message: input.message ?? null,
    })
    .select(NOTIFICATION_SELECT)
    .single();
  if (inserted.error || !inserted.data) {
    throw inserted.error
      ? toCodegenDbError(inserted.error, 'Failed to create task notification')
      : createError({ statusCode: 500, statusMessage: 'Failed to create task notification' });
  }
  return mapTaskNotification(inserted.data);
}

export async function createCodegenJob(
  context: CloudCodegenJobContext,
  input: CreateCloudCodegenJobInput,
): Promise<CloudCodegenJob> {
  if (!isServerProvider(input.provider)) {
    throw createError({
      statusCode: 400,
      statusMessage: 'Background jobs require a server-side AI provider.',
    });
  }

  const access = await assertCloudFileReadable({
    supabase: context.supabase,
    userId: context.userId,
    userEmail: context.userEmail,
    fileId: input.fileId,
  });
  if (access.shareRole === 'viewer') {
    throw createError({ statusCode: 403, statusMessage: 'Viewer access cannot create jobs' });
  }
  const jobKind = input.jobKind ?? 'full_generation';
  if (jobKind === 'patch_generation') {
    if (!input.baseGenerationId) {
      throw createError({ statusCode: 400, statusMessage: 'Patch jobs require baseGenerationId' });
    }
    if (!input.patchInstruction?.trim()) {
      throw createError({ statusCode: 400, statusMessage: 'Patch jobs require patchInstruction' });
    }
  }

  const inserted = await context.supabase
    .from('codegen_jobs')
    .insert({
      file_id: input.fileId,
      owner_id: access.ownerId,
      file_name: input.fileName ?? null,
      page_name: input.pageName ?? null,
      job_kind: jobKind,
      framework: input.framework,
      page_id: input.pageId,
      target_kind: input.targetKind,
      node_ids: input.nodeIds,
      target_hash: input.targetHash,
      document_revision: input.documentRevision,
      provider: input.provider,
      model: input.model,
      input_snapshot: {
        nodes: input.nodes,
        variables: input.variables ?? {},
        fileName: input.fileName ?? null,
        pageName: input.pageName ?? null,
        qualityMode: input.qualityMode ?? 'production',
        baseGenerationId: input.baseGenerationId ?? null,
        patchInstruction: input.patchInstruction?.trim() ?? null,
      },
    })
    .select(JOB_DETAIL_SELECT)
    .single();

  if (inserted.error || !inserted.data) {
    throw inserted.error
      ? toCodegenDbError(inserted.error, 'Failed to create codegen job')
      : createError({ statusCode: 500, statusMessage: 'Failed to create codegen job' });
  }

  await addJobEvent({
    supabase: context.supabase,
    jobId: inserted.data.id,
    type: 'created',
    message: 'Code generation job queued.',
  });
  await recordCloudActivity({
    supabase: context.supabase,
    ownerId: access.ownerId,
    actorId: context.userId,
    fileId: input.fileId,
    type: 'codegen_job_created',
    metadata: {
      jobId: inserted.data.id,
      framework: input.framework,
      jobKind,
      pageId: input.pageId,
      targetKind: input.targetKind,
      baseGenerationId: input.baseGenerationId ?? null,
    },
  });

  triggerCodegenWorker();

  return getCodegenJobDetail({
    supabase: context.supabase,
    userId: access.ownerId,
    jobId: inserted.data.id,
  });
}

export async function cancelCodegenJob(input: {
  supabase: SupabaseClient;
  userId: string;
  jobId: string;
}): Promise<CloudCodegenJob> {
  const existing = await getCodegenJobDetail(input);
  if (existing.status === 'succeeded' || existing.status === 'failed' || existing.status === 'canceled') {
    return existing;
  }
  const now = new Date().toISOString();
  await updateJob({
    supabase: input.supabase,
    jobId: input.jobId,
    patch: {
      status: 'canceled',
      progress: 100,
      canceled_at: now,
      completed_at: now,
      locked_by: null,
      locked_until: null,
      last_heartbeat_at: null,
    },
  });
  await addJobEvent({
    supabase: input.supabase,
    jobId: input.jobId,
    type: 'canceled',
    message: 'Code generation job canceled.',
  });
  await createTaskNotification({
    supabase: input.supabase,
    ownerId: input.userId,
    jobId: input.jobId,
    fileId: existing.fileId,
    generationId: existing.generationId,
    kind: 'codegen_job_canceled',
    title: 'Code generation canceled',
    message: existing.framework,
  });
  await recordCloudActivity({
    supabase: input.supabase,
    ownerId: input.userId,
    actorId: input.userId,
    fileId: existing.fileId,
    type: 'codegen_job_canceled',
    metadata: { jobId: input.jobId, framework: existing.framework },
  });
  return getCodegenJobDetail(input);
}

export async function batchCancelCodegenJobs(input: {
  supabase: SupabaseClient;
  userId: string;
  jobIds: string[];
}): Promise<CloudCodegenJob[]> {
  const uniqueJobIds = Array.from(new Set(input.jobIds)).filter(Boolean);
  if (uniqueJobIds.length === 0) return [];
  if (uniqueJobIds.length > 50) {
    throw createError({
      statusCode: 400,
      statusMessage: 'At most 50 codegen jobs can be canceled at once',
    });
  }

  const canceled: CloudCodegenJob[] = [];
  for (const jobId of uniqueJobIds) {
    canceled.push(
      await cancelCodegenJob({
        supabase: input.supabase,
        userId: input.userId,
        jobId,
      }),
    );
  }
  return canceled;
}

export async function retryCodegenJob(input: {
  supabase: SupabaseClient;
  userId: string;
  jobId: string;
}): Promise<CloudCodegenJob> {
  const existing = await getCodegenJobDetail(input);
  if (existing.status === 'pending' || existing.status === 'running') return existing;
  if (existing.status === 'succeeded') {
    throw createError({
      statusCode: 409,
      statusMessage: 'Completed codegen jobs cannot be retried',
    });
  }

  const now = new Date().toISOString();
  await updateJob({
    supabase: input.supabase,
    jobId: input.jobId,
    patch: {
      status: 'pending',
      generation_id: null,
      pipeline_mode: null,
      progress: 0,
      attempts: 0,
      locked_by: null,
      locked_until: null,
      last_heartbeat_at: null,
      next_run_at: now,
      dead_lettered_at: null,
      last_error: null,
      failure_type: null,
      output: {},
      error: null,
      started_at: null,
      completed_at: null,
      canceled_at: null,
    },
  });
  await addJobEvent({
    supabase: input.supabase,
    jobId: input.jobId,
    type: 'retry_queued',
    message: 'Code generation job retry queued.',
  });
  await recordCloudActivity({
    supabase: input.supabase,
    ownerId: input.userId,
    actorId: input.userId,
    fileId: existing.fileId,
    type: 'codegen_job_created',
    metadata: {
      jobId: input.jobId,
      retry: true,
      previousStatus: existing.status,
      framework: existing.framework,
    },
  });

  triggerCodegenWorker();

  return getCodegenJobDetail(input);
}

type CodegenStageRerunMode = Exclude<CodegenResumeMode, 'from_failed_stage'>;

function isRecoverableCodegenJob(job: CloudCodegenJob) {
  return job.status === 'failed' || Boolean(job.deadLetteredAt);
}

function objectFromUnknown(value: unknown): Record<string, unknown> {
  return value && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}

function normalizeCodegenPipelineMode(value: unknown): CodegenPipelineMode | null {
  return value === 'direct_generation' || value === 'full_pipeline' || value === 'unknown'
    ? value
    : null;
}

function inferCodegenPipelineMode(timing?: CodegenTimingBreakdown | null): CodegenPipelineMode {
  const stages = timing?.providerCalls?.map((call) => call.stage) ?? [];
  if (stages.includes('direct_generation')) return 'direct_generation';
  if (stages.some((stage) => stage === 'planning' || stage === 'chunk' || stage === 'assembly')) {
    return 'full_pipeline';
  }
  return 'unknown';
}

function pipelineModeFromOutput(output: unknown): CodegenPipelineMode | null {
  const object = objectFromUnknown(output);
  const explicit = normalizeCodegenPipelineMode(object.pipelineMode);
  if (explicit) return explicit;
  const metadata = objectFromUnknown(object.metadata);
  return normalizeCodegenPipelineMode(metadata.pipelineMode);
}

function checkpointArrayFromUnknown(value: unknown): CodegenCheckpoint[] {
  return Array.isArray(value) ? (value as CodegenCheckpoint[]) : [];
}

function checkpointsFromJobOutput(output: unknown): CodegenCheckpoint[] {
  return checkpointArrayFromUnknown(objectFromUnknown(output).checkpoints);
}

function latestCheckpoint(
  checkpoints: CodegenCheckpoint[],
  stage: CodegenCheckpoint['stage'],
): CodegenCheckpoint | null {
  return checkpoints
    .filter((checkpoint) => checkpoint.stage === stage)
    .sort((a, b) => {
      const attemptDelta = Number(b.attempt ?? 0) - Number(a.attempt ?? 0);
      if (attemptDelta !== 0) return attemptDelta;
      return new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime();
    })[0] ?? null;
}

function checkpointData(checkpoint: CodegenCheckpoint | null): Record<string, unknown> {
  return objectFromUnknown(checkpoint?.data);
}

function isSaveCodegenFileArray(value: unknown): value is SaveCodegenFileInput[] {
  return (
    Array.isArray(value) &&
    value.every(
      (item) =>
        item &&
        typeof item === 'object' &&
        typeof (item as { path?: unknown }).path === 'string' &&
        typeof (item as { content?: unknown }).content === 'string',
    )
  );
}

function isDesignIR(value: unknown): value is CodegenDesignIR {
  return Boolean(value) && typeof value === 'object' && Array.isArray((value as { nodes?: unknown }).nodes);
}

function isCodePlan(value: unknown): value is CodePlanFromAI {
  return Boolean(value) && typeof value === 'object' && Array.isArray((value as { chunks?: unknown }).chunks);
}

function isCodegenQualityReport(value: unknown): value is CodegenQualityReport {
  if (!value || typeof value !== 'object') return false;
  const report = value as Partial<CodegenQualityReport>;
  return (
    typeof report.status === 'string' &&
    typeof report.framework === 'string' &&
    Array.isArray(report.issues) &&
    Boolean(report.summary) &&
    typeof report.summary === 'object'
  );
}

function isChunkStatus(value: unknown): value is ChunkStatus {
  return ['pending', 'running', 'done', 'degraded', 'failed', 'skipped'].includes(String(value));
}

function isChunkResult(value: unknown): value is ChunkResult {
  if (!value || typeof value !== 'object') return false;
  const result = value as Partial<ChunkResult>;
  return typeof result.chunkId === 'string' && typeof result.code === 'string' && Boolean(result.contract);
}

function chunkCheckpointsFromOutput(output: unknown): Array<{
  chunkId: string;
  name: string;
  status: ChunkStatus;
  result?: ChunkResult;
  error?: string;
}> {
  return checkpointsFromJobOutput(output)
    .filter((checkpoint) => checkpoint.stage === 'chunk')
    .sort((a, b) => {
      const attemptDelta = Number(b.attempt ?? 0) - Number(a.attempt ?? 0);
      if (attemptDelta !== 0) return attemptDelta;
      return new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime();
    })
    .reduce<Array<{
      chunkId: string;
      name: string;
      status: ChunkStatus;
      result?: ChunkResult;
      error?: string;
    }>>((acc, checkpoint) => {
      const data = checkpointData(checkpoint);
      const chunkId = typeof data.chunkId === 'string' ? data.chunkId : null;
      if (!chunkId || acc.some((item) => item.chunkId === chunkId)) return acc;
      const result = isChunkResult(data.result) ? data.result : undefined;
      acc.push({
        chunkId,
        name: typeof data.name === 'string' ? data.name : chunkId,
        status: isChunkStatus(data.status)
          ? data.status
          : checkpoint.status === 'failed'
            ? 'failed'
            : result
              ? 'done'
              : 'pending',
        result,
        error: typeof data.error === 'string' ? data.error : undefined,
      });
      return acc;
    }, []);
}

function hasAssemblyCheckpointForResume(output: unknown): boolean {
  const assembly = checkpointData(latestCheckpoint(checkpointsFromJobOutput(output), 'assembly'));
  return typeof assembly.code === 'string' || isSaveCodegenFileArray(assembly.files);
}

async function loadLatestCodegenJobOutput(input: {
  supabase: SupabaseClient;
  jobId: string;
  fallback?: unknown;
}): Promise<Record<string, unknown>> {
  const current = await input.supabase
    .from('codegen_jobs')
    .select('output')
    .eq('id', input.jobId)
    .maybeSingle();
  if (current.error) throw toCodegenDbError(current.error, 'Failed to load codegen checkpoints');
  const output = objectFromUnknown(current.data?.output);
  const fallback = objectFromUnknown(input.fallback);
  return {
    ...fallback,
    ...output,
    checkpoints: Array.isArray(output.checkpoints) ? output.checkpoints : fallback.checkpoints,
  };
}

export async function appendCodegenCheckpoint(input: {
  supabase: SupabaseClient;
  jobId: string;
  attempt: number;
  stage: CodegenCheckpoint['stage'];
  status: CodegenCheckpoint['status'];
  data: unknown;
}) {
  const current = await input.supabase
    .from('codegen_jobs')
    .select('output')
    .eq('id', input.jobId)
    .maybeSingle();
  if (current.error) throw toCodegenDbError(current.error, 'Failed to load codegen checkpoints');

  const output = objectFromUnknown(current.data?.output);
  const checkpoints = checkpointArrayFromUnknown(output.checkpoints);
  const nextCheckpoint: CodegenCheckpoint = {
    stage: input.stage,
    status: input.status,
    attempt: input.attempt,
    data: input.data,
    createdAt: new Date().toISOString(),
  };
  const nextCheckpoints = [
    ...checkpoints.filter(
      (checkpoint) =>
        !(checkpoint.stage === input.stage && Number(checkpoint.attempt) === input.attempt),
    ),
    nextCheckpoint,
  ];

  await updateJob({
    supabase: input.supabase,
    jobId: input.jobId,
    patch: {
      output: {
        ...output,
        checkpoints: nextCheckpoints,
        latestCheckpoint: nextCheckpoint,
      },
    },
  });
}

async function requeueCodegenJobWithResume(input: {
  supabase: SupabaseClient;
  userId: string;
  jobId: string;
  mode: CodegenResumeMode;
  chunkId?: string;
  eventType: 'resume_queued' | 'stage_rerun_queued';
  eventMessage: string;
}): Promise<CloudCodegenJob> {
  const existing = await getCodegenJobDetail(input);
  if (existing.status === 'pending' || existing.status === 'running') return existing;
  if (existing.status === 'succeeded') {
    throw createError({
      statusCode: 409,
      statusMessage: 'Completed codegen jobs cannot be resumed',
    });
  }
  if (!isRecoverableCodegenJob(existing) && existing.status !== 'canceled') {
    throw createError({
      statusCode: 409,
      statusMessage: 'Only failed codegen jobs can be resumed',
    });
  }

  const now = new Date().toISOString();
  const resume = {
    mode: input.mode,
    stage: input.mode,
    chunkId: input.chunkId ?? null,
    requestedAt: now,
    requestedBy: input.userId,
    previousStatus: existing.status,
    previousFailureType: existing.failureType ?? null,
    previousError: existing.error ?? existing.lastError ?? null,
  };
  const inputSnapshot = objectFromUnknown(existing.inputSnapshot);
  const output = objectFromUnknown(existing.output);
  const nextMaxAttempts = Math.max(existing.maxAttempts, existing.attempts + 1, 1);

  await updateJob({
    supabase: input.supabase,
    jobId: input.jobId,
    patch: {
      status: 'pending',
      progress: 0,
      pipeline_mode: null,
      max_attempts: nextMaxAttempts,
      locked_by: null,
      locked_until: null,
      last_heartbeat_at: null,
      next_run_at: now,
      dead_lettered_at: null,
      last_error: null,
      failure_type: null,
      input_snapshot: {
        ...inputSnapshot,
        resume,
      },
      output: {
        ...output,
        resume,
      },
      error: null,
      completed_at: null,
      canceled_at: null,
    },
  });
  await addJobEvent({
    supabase: input.supabase,
    jobId: input.jobId,
    type: input.eventType,
    message: input.eventMessage,
    metadata: resume,
  });
  await recordCloudActivity({
    supabase: input.supabase,
    ownerId: existing.ownerId,
    actorId: input.userId,
    fileId: existing.fileId,
    generationId: existing.generationId,
    type: 'codegen_job_replayed',
    metadata: {
      jobId: input.jobId,
      framework: existing.framework,
      jobKind: existing.jobKind,
      resume: true,
      ...resume,
    },
  });

  triggerCodegenWorker();

  return getCodegenJobDetail(input);
}

export async function resumeCodegenJob(input: {
  supabase: SupabaseClient;
  userId: string;
  jobId: string;
  stage?: CodegenResumeMode;
}): Promise<CloudCodegenJob> {
  const stage = input.stage ?? 'from_failed_stage';
  return requeueCodegenJobWithResume({
    supabase: input.supabase,
    userId: input.userId,
    jobId: input.jobId,
    mode: stage,
    eventType: 'resume_queued',
    eventMessage: 'Code generation job resume queued.',
  });
}

export async function rerunCodegenJobStep(input: {
  supabase: SupabaseClient;
  userId: string;
  jobId: string;
  stage: CodegenStageRerunMode;
  chunkId?: string;
}): Promise<CloudCodegenJob> {
  return requeueCodegenJobWithResume({
    supabase: input.supabase,
    userId: input.userId,
    jobId: input.jobId,
    mode: input.stage,
    chunkId: input.chunkId,
    eventType: 'stage_rerun_queued',
    eventMessage: 'Code generation job stage rerun queued.',
  });
}

function createServerCollectText(): CodegenTextCollector {
  return async (system, message, model, provider, abortSignal) => {
    if (!isServerProvider(provider ?? '')) {
      throw new Error('Background jobs require a server-side AI provider.');
    }
    const { createChatCompletionText } = await import('./server-codegen-provider');
    return createChatCompletionText({ system, message, model, provider, abortSignal });
  };
}

function serverCollectText(): CodegenTextCollector {
  return createServerCollectText();
}

export async function runCodegenCheckpointResume(input: {
  supabase: SupabaseClient;
  job: Record<string, any>;
  workerId: string;
  attempt: number;
  mode?: CodegenResumeMode;
  heartbeat: () => Promise<void>;
  assertNotCanceled: () => Promise<void>;
  collectText?: CodegenTextCollector;
}): Promise<{
  code: string;
  degraded: boolean;
  assets: CodegenAssetFile[];
  files?: SaveCodegenFileInput[];
  qualityReport?: CodegenQualityReport;
  timing?: CodegenTimingBreakdown;
  repairAttempts?: CodegenRepairAttempt[];
  designIR?: CodegenDesignIR;
  pipelineMode?: CodegenPipelineMode;
} | null> {
  if (input.mode !== 'quality_check' && input.mode !== 'repair' && input.mode !== 'chunk') return null;

  const latestOutput = await loadLatestCodegenJobOutput({
    supabase: input.supabase,
    jobId: input.job.id,
    fallback: input.job.output,
  });
  const checkpoints = checkpointsFromJobOutput(latestOutput);
  const resume = objectFromUnknown(objectFromUnknown(input.job.input_snapshot).resume);
  if (input.mode === 'chunk') {
    const chunkId = typeof resume.chunkId === 'string' ? resume.chunkId : null;
    const planning = checkpointData(latestCheckpoint(checkpoints, 'planning'));
    const plan = isCodePlan(planning.plan) ? planning.plan : null;
    const designIR = isDesignIR(planning.designIR) ? planning.designIR : undefined;
    if (!chunkId || !plan) {
      await addJobEvent({
        supabase: input.supabase,
        jobId: input.job.id,
        type: 'resume_fallback_full_run',
        message: 'Chunk checkpoint resume fell back to full generation because planning output or chunk id is missing.',
        metadata: { mode: input.mode, chunkId },
      });
      return null;
    }

    await input.heartbeat();
    await input.assertNotCanceled();
    await upsertCodegenJobStep({
      supabase: input.supabase,
      jobId: input.job.id,
      attempt: input.attempt,
      agentRole: 'planner',
      status: 'succeeded',
      progress: 100,
      output: { resumedFromCheckpoint: true, mode: input.mode },
    });
    await upsertCodegenJobStep({
      supabase: input.supabase,
      jobId: input.job.id,
      attempt: input.attempt,
      agentRole: 'page_codegen',
      status: 'running',
      data: { chunkId, resumedFromCheckpoint: true },
    });

    try {
      const resumed = await resumeCodegenFromChunkCheckpoint({
        plan,
        nodes: Array.isArray(input.job.input_snapshot?.nodes)
          ? input.job.input_snapshot.nodes
          : [],
        framework: input.job.framework,
        variables: objectFromUnknown(input.job.input_snapshot?.variables),
        designIR,
        chunkId,
        checkpoints: chunkCheckpointsFromOutput(latestOutput),
        model: input.job.model ?? '',
        provider: input.job.provider ?? undefined,
        collectText: input.collectText ?? serverCollectText(),
      });
      await appendCodegenCheckpoint({
        supabase: input.supabase,
        jobId: input.job.id,
        attempt: input.attempt,
        stage: 'chunk',
        status: 'succeeded',
        data: {
          chunkId,
          name: resumed.chunkName,
          status: 'done',
          result: resumed.chunkResult,
          resumedFromCheckpoint: true,
        },
      });
      await upsertCodegenJobStep({
        supabase: input.supabase,
        jobId: input.job.id,
        attempt: input.attempt,
        agentRole: 'page_codegen',
        status: 'succeeded',
        progress: 100,
        output: { chunkId, resumedFromCheckpoint: true },
      });
      await upsertCodegenJobStep({
        supabase: input.supabase,
        jobId: input.job.id,
        attempt: input.attempt,
        agentRole: 'quality_check',
        status: hasCodegenQualityErrors(resumed.qualityReport!) ? 'failed' : 'succeeded',
        progress: 100,
        output: { report: resumed.qualityReport, resumedFromCheckpoint: true },
      });
      if (resumed.repairAttempts?.length) {
        await upsertCodegenJobStep({
          supabase: input.supabase,
          jobId: input.job.id,
          attempt: input.attempt,
          agentRole: 'repair',
          status: hasCodegenQualityErrors(resumed.qualityReport!) ? 'failed' : 'succeeded',
          progress: 100,
          output: {
            report: resumed.qualityReport,
            attempt: resumed.repairAttempts.at(-1)?.attempt,
            resumedFromCheckpoint: true,
          },
        });
      }
      await upsertCodegenJobStep({
        supabase: input.supabase,
        jobId: input.job.id,
        attempt: input.attempt,
        agentRole: 'bundler_persist',
        status: 'running',
      });
      await updateJob({
        supabase: input.supabase,
        jobId: input.job.id,
        patch: { progress: 90 },
        lockedBy: input.workerId,
      });
      return resumed;
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Chunk generation failed';
      await appendCodegenCheckpoint({
        supabase: input.supabase,
        jobId: input.job.id,
        attempt: input.attempt,
        stage: 'chunk',
        status: 'failed',
        data: { chunkId, status: 'failed', error: message, resumedFromCheckpoint: true },
      });
      await upsertCodegenJobStep({
        supabase: input.supabase,
        jobId: input.job.id,
        attempt: input.attempt,
        agentRole: 'page_codegen',
        status: 'failed',
        progress: 100,
        data: { chunkId, resumedFromCheckpoint: true },
        error: message,
      });
      throw err;
    }
  }

  const assembly = checkpointData(latestCheckpoint(checkpoints, 'assembly'));
  const code = typeof assembly.code === 'string' ? assembly.code : null;
  const files = isSaveCodegenFileArray(assembly.files) ? assembly.files : undefined;
  const planning = checkpointData(latestCheckpoint(checkpoints, 'planning'));
  const designIR = isDesignIR(planning.designIR) ? planning.designIR : undefined;
  const qualityCheckpoint = checkpointData(latestCheckpoint(checkpoints, 'quality_check'));
  const checkpointReport = isCodegenQualityReport(qualityCheckpoint.report)
    ? qualityCheckpoint.report
    : null;
  if (!code && !files?.length) {
    await addJobEvent({
      supabase: input.supabase,
      jobId: input.job.id,
      type: 'resume_fallback_full_run',
      message: 'Checkpoint resume fell back to full generation because assembly output is missing.',
      metadata: { mode: input.mode },
    });
    return null;
  }

  await input.heartbeat();
  await input.assertNotCanceled();
  await finishCheckpointResumePrerequisites({
    supabase: input.supabase,
    jobId: input.job.id,
    attempt: input.attempt,
    mode: input.mode,
  });

  const sourceCode = code ?? files!.map((file) => `---FILE: ${file.path}---\n${file.content}`).join('\n\n');
  const qualityGate =
    input.mode === 'repair' && checkpointReport
      ? (() => {
          const checkpointFiles =
            files ?? buildCodegenFiles({ framework: input.job.framework, code: sourceCode });
          return {
            code: sourceCode,
            files: checkpointFiles,
            qualityReport: checkpointReport,
            degraded: hasCodegenQualityErrors(checkpointReport),
          };
        })()
      : runCodegenQualityGate({
          framework: input.job.framework,
          code: sourceCode,
          files,
          designIR,
        });
  await appendCodegenCheckpoint({
    supabase: input.supabase,
    jobId: input.job.id,
    attempt: input.attempt,
    stage: 'quality_check',
    status: hasCodegenQualityErrors(qualityGate.qualityReport) ? 'failed' : 'succeeded',
    data: {
      report: qualityGate.qualityReport,
      resumedFromCheckpoint: true,
      reusedCheckpointReport: input.mode === 'repair' && Boolean(checkpointReport),
    },
  });
  await upsertCodegenJobStep({
    supabase: input.supabase,
    jobId: input.job.id,
    attempt: input.attempt,
    agentRole: 'quality_check',
    status: hasCodegenQualityErrors(qualityGate.qualityReport) ? 'failed' : 'succeeded',
    progress: 100,
    output: { report: qualityGate.qualityReport, resumedFromCheckpoint: true },
    error: hasCodegenQualityErrors(qualityGate.qualityReport)
      ? qualityGate.qualityReport.issues
          .filter((issue) => issue.severity === 'error')
          .map((issue) => issue.message)
          .join('; ')
      : null,
  });
  await updateJob({
    supabase: input.supabase,
    jobId: input.job.id,
    patch: { progress: input.mode === 'quality_check' ? 90 : 78 },
    lockedBy: input.workerId,
  });

  if (input.mode === 'quality_check' || !hasCodegenQualityErrors(qualityGate.qualityReport)) {
    await upsertCodegenJobStep({
      supabase: input.supabase,
      jobId: input.job.id,
      attempt: input.attempt,
      agentRole: 'bundler_persist',
      status: 'running',
    });
    return {
      code: qualityGate.code,
      degraded: qualityGate.degraded,
      assets: [],
      files: qualityGate.files,
      qualityReport: qualityGate.qualityReport,
      timing: { qualityCheckMs: 0, totalMs: 0, providerMs: 0 },
      repairAttempts: [],
      designIR,
      pipelineMode: pipelineModeFromOutput(latestOutput) ?? 'unknown',
    };
  }

  await input.heartbeat();
  await input.assertNotCanceled();
  await upsertCodegenJobStep({
    supabase: input.supabase,
    jobId: input.job.id,
    attempt: input.attempt,
    agentRole: 'repair',
    status: 'running',
    data: { report: qualityGate.qualityReport, resumedFromCheckpoint: true },
  });
  const repair = await runCodegenRepairPass({
    framework: input.job.framework,
    code: qualityGate.code,
    files: qualityGate.files,
    designIR,
    qualityReport: qualityGate.qualityReport,
    model: input.job.model ?? '',
    provider: input.job.provider ?? undefined,
    collectText: input.collectText ?? serverCollectText(),
  });
  const repairFailed = hasCodegenQualityErrors(repair.qualityReport);
  await appendCodegenCheckpoint({
    supabase: input.supabase,
    jobId: input.job.id,
    attempt: input.attempt,
    stage: 'repair',
    status: repairFailed ? 'failed' : 'succeeded',
    data: {
      attempt: repair.repairAttempt.attempt,
      report: repair.qualityReport,
      code: repair.code,
      files: repair.files,
      resumedFromCheckpoint: true,
    },
  });
  await upsertCodegenJobStep({
    supabase: input.supabase,
    jobId: input.job.id,
    attempt: input.attempt,
    agentRole: 'repair',
    status: repairFailed ? 'failed' : 'succeeded',
    progress: 100,
    output: {
      report: repair.qualityReport,
      attempt: repair.repairAttempt.attempt,
      resumedFromCheckpoint: true,
    },
    error: repairFailed
      ? repair.qualityReport.issues
          .filter((issue) => issue.severity === 'error')
          .map((issue) => issue.message)
          .join('; ')
      : null,
  });
  await upsertCodegenJobStep({
    supabase: input.supabase,
    jobId: input.job.id,
    attempt: input.attempt,
    agentRole: 'bundler_persist',
    status: 'running',
  });
  await updateJob({
    supabase: input.supabase,
    jobId: input.job.id,
    patch: { progress: 90 },
    lockedBy: input.workerId,
  });
  return {
    code: repair.code,
    degraded: repair.degraded,
    assets: [],
    files: repair.files,
    qualityReport: repair.qualityReport,
    timing: repair.timing,
    repairAttempts: [repair.repairAttempt],
    designIR,
    pipelineMode: pipelineModeFromOutput(latestOutput) ?? inferCodegenPipelineMode(repair.timing),
  };
}

async function finishCheckpointResumePrerequisites(input: {
  supabase: SupabaseClient;
  jobId: string;
  attempt: number;
  mode: CodegenResumeMode;
}) {
  for (const agentRole of ['planner', 'page_codegen'] satisfies CodegenJobAgentRole[]) {
    await upsertCodegenJobStep({
      supabase: input.supabase,
      jobId: input.jobId,
      attempt: input.attempt,
      agentRole,
      status: 'succeeded',
      progress: 100,
      output: { resumedFromCheckpoint: true, mode: input.mode },
    });
  }
}

export async function runCodegenWorkerOnce(
  supabase: SupabaseClient,
  options: CodegenWorkerRuntimeOptions & { collectText?: CodegenTextCollector } = {},
): Promise<CloudCodegenJob | null> {
  const workerOptions = resolveCodegenWorkerOptions(options);
  await requeueStaleCodegenJobs({ supabase }).catch((err) => {
    console.warn('[codegen-job-worker] stale lock recovery failed', err);
  });
  const job = await claimNextCodegenJob({
    supabase,
    workerId: workerOptions.workerId,
    lockMs: workerOptions.lockMs,
  }).catch((err) => {
    throw toCodegenDbError(err, 'Failed to claim codegen job');
  });
  if (!job) return null;
  let heartbeatTimer: ReturnType<typeof setInterval> | undefined;
  const stopHeartbeat = () => {
    if (heartbeatTimer) clearInterval(heartbeatTimer);
  };
  const heartbeat = async () => {
    const renewed = await heartbeatCodegenJob({
      supabase,
      jobId: job.id,
      workerId: workerOptions.workerId,
      lockMs: workerOptions.lockMs,
    }).catch((err) => {
      throw toCodegenDbError(err, 'Failed to heartbeat codegen job');
    });
    if (!renewed) {
      throw new Error('Code generation job lock was lost or canceled.');
    }
  };
  heartbeatTimer = setInterval(() => {
    void heartbeat().catch((err) => {
      console.warn('[codegen-job-worker] heartbeat failed', err);
    });
  }, workerOptions.heartbeatMs);
  const assertNotCanceled = async () => {
    const canceled = await isCodegenJobCanceled({ supabase, jobId: job.id }).catch((err) => {
      throw toCodegenDbError(err, 'Failed to check codegen job status');
    });
    if (canceled) throw new Error('Code generation job canceled.');
  };

  const input = (job.input_snapshot ?? {}) as {
    nodes?: PenNode[];
    variables?: Record<string, unknown>;
    qualityMode?: 'production' | 'draft';
    baseGenerationId?: string | null;
    patchInstruction?: string | null;
    resume?: {
      mode?: CodegenResumeMode;
      stage?: CodegenResumeMode;
      chunkId?: string | null;
    };
  };
  const provider = job.provider;
  const jobKind = job.job_kind ?? 'full_generation';
  const stepAttempt = Math.max(1, Math.trunc(Number(job.attempts ?? 1)));
  const resumeMode = input.resume?.mode ?? input.resume?.stage;
  const collectText = options.collectText ?? createServerCollectText();

  try {
    const latestOutput =
      resumeMode === 'quality_check' || resumeMode === 'repair'
        ? await loadLatestCodegenJobOutput({
            supabase,
            jobId: job.id,
            fallback: job.output,
          })
        : objectFromUnknown(job.output);
    const canResumeWithoutProvider =
      resumeMode === 'quality_check' && hasAssemblyCheckpointForResume(latestOutput);
    const jobForExecution = { ...job, output: latestOutput };

    if (!canResumeWithoutProvider) {
      await assertProviderCanRunCodegen({
        supabase,
        provider,
        maxPerMinute: workerOptions.providerRateLimitPerMinute,
      }).catch((err) => {
        throw toCodegenDbError(err, 'Failed to reserve codegen provider capacity');
      });
    }
    await heartbeat();
    await assertNotCanceled();
    await addJobEvent({
      supabase,
      jobId: job.id,
      type: 'started',
      message: 'Background worker started the job.',
    });
    await upsertCodegenJobStep({
      supabase,
      jobId: job.id,
      attempt: stepAttempt,
      agentRole: 'planner',
      status: 'running',
    });
    const finishStep = (agentRole: CodegenJobAgentRole) =>
      upsertCodegenJobStep({
        supabase,
        jobId: job.id,
        attempt: stepAttempt,
        agentRole,
        status: 'succeeded',
        progress: 100,
      });

    const result: {
      code: string;
      degraded: boolean;
      assets: CodegenAssetFile[];
      files?: SaveCodegenFileInput[];
      qualityReport?: CodegenQualityReport;
      timing?: CodegenTimingBreakdown;
      repairAttempts?: CodegenRepairAttempt[];
      designIR?: CodegenDesignIR;
      pipelineMode?: CodegenPipelineMode;
    } =
      (await runCodegenCheckpointResume({
        supabase,
        job: jobForExecution,
        workerId: workerOptions.workerId,
        attempt: stepAttempt,
        mode: resumeMode,
        heartbeat,
        assertNotCanceled,
        collectText,
      })) ??
      (jobKind === 'patch_generation'
        ? await (async () => {
            if (!input.baseGenerationId || !input.patchInstruction?.trim()) {
              throw new Error('Patch job is missing base generation or instruction.');
            }
            await finishStep('planner');
            await upsertCodegenJobStep({
              supabase,
              jobId: job.id,
              attempt: stepAttempt,
              agentRole: 'page_codegen',
              status: 'running',
              data: {
                baseGenerationId: input.baseGenerationId,
                patchInstruction: input.patchInstruction,
              },
            });
            await updateJob({ supabase, jobId: job.id, patch: { progress: 25 } });
            const baseGeneration = await getCodeGenerationDetail({
              supabase,
              userId: job.owner_id,
              generationId: input.baseGenerationId,
            });
            const patched = await generateCodePatch({
              framework: job.framework,
              nodes: input.nodes ?? [],
              instruction: input.patchInstruction,
              baseGeneration,
              model: job.model ?? '',
              provider: job.provider ?? undefined,
              collectText,
            });
            await assertNotCanceled();
            await heartbeat();
            await finishStep('page_codegen');
            await finishStep('reviewer_repair');
            await upsertCodegenJobStep({
              supabase,
              jobId: job.id,
              attempt: stepAttempt,
              agentRole: 'bundler_persist',
              status: 'running',
            });
            await updateJob({ supabase, jobId: job.id, patch: { progress: 75 } });
            return { ...patched, assets: [] };
          })()
        : await generateCode(
            input.nodes ?? [],
            job.framework,
            input.variables,
            async (event) => {
              await assertNotCanceled();
              await heartbeat();
              if (event.step === 'chunk' && event.status !== 'pending' && event.status !== 'running') {
                await appendCodegenCheckpoint({
                  supabase,
                  jobId: job.id,
                  attempt: stepAttempt,
                  stage: 'chunk',
                  status: event.status === 'failed' ? 'failed' : 'succeeded',
                  data: {
                    chunkId: event.chunkId,
                    name: event.name,
                    status: event.status,
                    result: event.result ?? null,
                    error: event.error ?? null,
                  },
                });
              }
              if (event.step === 'planning' && event.status === 'done') {
                await appendCodegenCheckpoint({
                  supabase,
                  jobId: job.id,
                  attempt: stepAttempt,
                  stage: 'planning',
                  status: 'succeeded',
                  data: {
                    plan: event.plan ?? null,
                    designIR: event.designIR ?? null,
                  },
                });
                await finishStep('planner');
                await upsertCodegenJobStep({
                  supabase,
                  jobId: job.id,
                  attempt: stepAttempt,
                  agentRole: 'page_codegen',
                  status: 'running',
                });
                await updateJob({ supabase, jobId: job.id, patch: { progress: 25 } });
              }
              if (event.step === 'assembly' && event.status === 'running') {
                await finishStep('planner');
                await finishStep('page_codegen');
                await upsertCodegenJobStep({
                  supabase,
                  jobId: job.id,
                  attempt: stepAttempt,
                  agentRole: 'quality_check',
                  status: 'running',
                });
                await updateJob({ supabase, jobId: job.id, patch: { progress: 65 } });
              }
              if (event.step === 'assembly' && event.status !== 'running') {
                await appendCodegenCheckpoint({
                  supabase,
                  jobId: job.id,
                  attempt: stepAttempt,
                  stage: 'assembly',
                  status: event.status === 'done' ? 'succeeded' : 'failed',
                  data: {
                    code: event.code ?? null,
                    files: event.files ?? [],
                    error: event.error ?? null,
                  },
                });
              }
              if (event.step === 'quality_check' && event.status === 'done') {
                await appendCodegenCheckpoint({
                  supabase,
                  jobId: job.id,
                  attempt: stepAttempt,
                  stage: 'quality_check',
                  status: 'succeeded',
                  data: {
                    report: event.report ?? null,
                  },
                });
                await upsertCodegenJobStep({
                  supabase,
                  jobId: job.id,
                  attempt: stepAttempt,
                  agentRole: 'quality_check',
                  status: 'succeeded',
                  progress: 100,
                  output: event.report ? { report: event.report } : undefined,
                  error: event.error ?? null,
                });
                await updateJob({ supabase, jobId: job.id, patch: { progress: 78 } });
              }
              if (event.step === 'quality_check' && event.status === 'failed') {
                await appendCodegenCheckpoint({
                  supabase,
                  jobId: job.id,
                  attempt: stepAttempt,
                  stage: 'quality_check',
                  status: 'failed',
                  data: {
                    report: event.report ?? null,
                    error: event.error ?? null,
                  },
                });
                await upsertCodegenJobStep({
                  supabase,
                  jobId: job.id,
                  attempt: stepAttempt,
                  agentRole: 'quality_check',
                  status: 'running',
                  progress: 78,
                  output: event.report ? { report: event.report } : undefined,
                  error: event.error ?? null,
                });
                await upsertCodegenJobStep({
                  supabase,
                  jobId: job.id,
                  attempt: stepAttempt,
                  agentRole: 'repair',
                  status: 'running',
                  data: event.report ? { report: event.report } : undefined,
                });
                await updateJob({ supabase, jobId: job.id, patch: { progress: 78 } });
              }
              if (event.step === 'repair' && event.status !== 'running') {
                await appendCodegenCheckpoint({
                  supabase,
                  jobId: job.id,
                  attempt: stepAttempt,
                  stage: 'repair',
                  status:
                    event.status === 'done' || event.status === 'skipped'
                      ? 'succeeded'
                      : 'failed',
                  data: {
                    attempt: event.attempt ?? null,
                    report: event.report ?? null,
                    error: event.error ?? null,
                  },
                });
                await upsertCodegenJobStep({
                  supabase,
                  jobId: job.id,
                  attempt: stepAttempt,
                  agentRole: 'repair',
                  status:
                    event.status === 'done' || event.status === 'skipped'
                      ? 'succeeded'
                      : 'failed',
                  progress: 100,
                  output: event.report ? { report: event.report, attempt: event.attempt } : undefined,
                  error: event.error ?? null,
                });
                await updateJob({ supabase, jobId: job.id, patch: { progress: 86 } });
              }
              if (event.step === 'final_validation' && event.status === 'running') {
                await upsertCodegenJobStep({
                  supabase,
                  jobId: job.id,
                  attempt: stepAttempt,
                  agentRole: 'final_validation',
                  status: 'running',
                });
              }
              if (
                event.step === 'final_validation' &&
                event.status !== 'running' &&
                event.status !== 'failed'
              ) {
                await upsertCodegenJobStep({
                  supabase,
                  jobId: job.id,
                  attempt: stepAttempt,
                  agentRole: 'final_validation',
                  status: event.status === 'done' ? 'succeeded' : 'failed',
                  progress: 100,
                  error: event.error ?? null,
                });
                await upsertCodegenJobStep({
                  supabase,
                  jobId: job.id,
                  attempt: stepAttempt,
                  agentRole: 'bundler_persist',
                  status: 'running',
                });
                await updateJob({ supabase, jobId: job.id, patch: { progress: 90 } });
              }
              if (event.step === 'final_validation' && event.status === 'failed') {
                await appendCodegenCheckpoint({
                  supabase,
                  jobId: job.id,
                  attempt: stepAttempt,
                  stage: 'final_validation',
                  status: 'failed',
                  data: {
                    finalValidation: true,
                    error: event.error ?? null,
                  },
                });
              }
              if (event.step === 'error') {
                await addJobEvent({
                  supabase,
                  jobId: job.id,
                  type: 'error',
                  message: event.message,
                });
              }
            },
            job.model ?? '',
            job.provider ?? undefined,
            undefined,
            { collectText },
          ));

    await assertNotCanceled();
    await heartbeat();
    const files =
      'files' in result && result.files?.length
        ? result.files
        : buildCodegenFiles({ framework: job.framework, code: result.code });
    const pipelineMode =
      result.pipelineMode ??
      pipelineModeFromOutput(job.output) ??
      inferCodegenPipelineMode(result.timing);
    await appendCodegenCheckpoint({
      supabase,
      jobId: job.id,
      attempt: stepAttempt,
      stage: 'final_validation',
      status: result.qualityReport?.status === 'failed' ? 'failed' : 'succeeded',
      data: {
        finalCode: result.code,
        files,
        qualityReport: result.qualityReport ?? null,
        timing: result.timing ?? null,
        repairAttempts: result.repairAttempts ?? [],
        pipelineMode,
      },
    });
    const checkpointRow = await supabase
      .from('codegen_jobs')
      .select('output')
      .eq('id', job.id)
      .maybeSingle();
    if (checkpointRow.error) {
      throw toCodegenDbError(checkpointRow.error, 'Failed to load codegen checkpoints');
    }
    const outputWithCheckpoints = objectFromUnknown(checkpointRow.data?.output);
    const checkpoints = checkpointArrayFromUnknown(outputWithCheckpoints.checkpoints);
    const generation = await createCodeGenerationRecord({
      supabase,
      userId: job.owner_id,
      fileId: job.file_id,
      pageId: job.page_id,
      framework: job.framework,
      targetKind: job.target_kind,
      nodeIds: job.node_ids ?? [],
      targetHash: job.target_hash,
      documentRevision: job.document_revision,
      status: result.degraded ? 'degraded' : 'done',
      finalCode: result.code,
      entryFile: files[0]?.path,
      degraded: result.degraded,
      assets: result.assets.map((asset) => ({
        id: asset.id,
        relativePath: asset.relativePath,
        zipPath: asset.zipPath,
        mimeType: asset.mimeType,
        bytesBase64: bytesToBase64(asset.bytes),
        sourceNodeId: asset.sourceNodeId,
        sourceNodeName: asset.sourceNodeName,
        sourceKind: asset.sourceKind,
      })),
      files,
      model: job.model ?? undefined,
      provider: job.provider ?? undefined,
      chunks: [],
      metadata: {
        jobKind,
        qualityMode: input.qualityMode ?? 'production',
        qualityStatus: result.qualityReport?.status ?? null,
        qualityReport: result.qualityReport ?? null,
        timing: result.timing ?? null,
        repairAttempts: result.repairAttempts ?? [],
        pipelineMode,
        checkpoints,
        designIRSummary: result.designIR?.summary ?? null,
        baseGenerationId: input.baseGenerationId ?? null,
        patchInstruction: input.patchInstruction ?? null,
      },
    });
    const now = new Date().toISOString();
    await updateJob({
      supabase,
      jobId: job.id,
      patch: {
        status: 'succeeded',
        progress: 100,
        generation_id: generation.id,
        pipeline_mode: pipelineMode,
        output: {
          ...outputWithCheckpoints,
          generationId: generation.id,
          degraded: result.degraded,
          qualityStatus: result.qualityReport?.status ?? null,
          pipelineMode,
          qualityReport: result.qualityReport ?? null,
          timing: result.timing ?? null,
          repairAttempts: result.repairAttempts ?? [],
          baseGenerationId: input.baseGenerationId ?? null,
          patchInstruction: input.patchInstruction ?? null,
        },
        completed_at: now,
        locked_by: null,
        locked_until: null,
        last_heartbeat_at: null,
      },
      lockedBy: workerOptions.workerId,
    });
    await finishStep('bundler_persist');
    await addJobEvent({
      supabase,
      jobId: job.id,
      type: 'succeeded',
      message: 'Code generation completed.',
      metadata: {
        generationId: generation.id,
        jobKind,
        pipelineMode,
        baseGenerationId: input.baseGenerationId ?? null,
      },
    });
    await createTaskNotification({
      supabase,
      ownerId: job.owner_id,
      jobId: job.id,
      fileId: job.file_id,
      generationId: generation.id,
      kind: 'codegen_job_succeeded',
      title: 'Code generation completed',
      message: job.framework,
    });
    await recordCloudActivity({
      supabase,
      ownerId: job.owner_id,
      actorId: job.owner_id,
      fileId: job.file_id,
      generationId: generation.id,
      type: 'codegen_job_succeeded',
      metadata: {
        jobId: job.id,
        framework: job.framework,
        jobKind,
        pipelineMode,
        baseGenerationId: input.baseGenerationId ?? null,
      },
    });
    await recordProviderCodegenSuccess({ supabase, provider }).catch((err) => {
      console.warn('[codegen-job-worker] provider success recording failed', err);
    });
    return getCodegenJobDetail({ supabase, userId: job.owner_id, jobId: job.id });
  } catch (err) {
    const message = err instanceof Error ? err.message : 'Code generation job failed';
    const failureType = classifyCodegenJobFailure(err);
    const failurePolicy = await getProviderCodegenFailurePolicy({
      supabase,
      provider,
      fallbackThreshold: workerOptions.providerCircuitThreshold,
      fallbackOpenMs: workerOptions.providerCircuitOpenMs,
    }).catch(() => ({
      threshold: workerOptions.providerCircuitThreshold,
      openMs: workerOptions.providerCircuitOpenMs,
    }));
    await recordProviderCodegenFailure({
      supabase,
      provider,
      failureType,
      threshold: failurePolicy.threshold,
      openMs: failurePolicy.openMs,
      message,
    }).catch((providerErr) => {
      console.warn('[codegen-job-worker] provider failure recording failed', providerErr);
    });
    if (failureType === 'canceled') {
      const now = new Date().toISOString();
      await updateJob({
        supabase,
        jobId: job.id,
        patch: {
          status: 'canceled',
          progress: 100,
          error: message,
          canceled_at: now,
          completed_at: now,
          locked_by: null,
          locked_until: null,
          last_heartbeat_at: null,
        },
      });
      await addJobEvent({ supabase, jobId: job.id, type: 'canceled', message });
    } else if (
      resolveCodegenFailureDisposition({
        attempts: Number(job.attempts ?? 1),
        max_attempts: Number(job.max_attempts ?? 1),
      }) === 'retry'
    ) {
      await recordCodegenJobRetry({
        supabase,
        jobId: job.id,
        message,
        failureType,
        retryDelayMs: workerOptions.retryDelayMs,
      }).catch((dbErr) => {
        throw toCodegenDbError(dbErr, 'Failed to requeue codegen job');
      });
      await addJobEvent({
        supabase,
        jobId: job.id,
        type: 'retry_scheduled',
        message,
        metadata: { failureType, nextAttempt: Number(job.attempts ?? 1) + 1 },
      });
    } else {
      await recordCodegenJobDeadLetter({
        supabase,
        jobId: job.id,
        message,
        failureType,
      }).catch((dbErr) => {
        throw toCodegenDbError(dbErr, 'Failed to dead-letter codegen job');
      });
      await addJobEvent({
        supabase,
        jobId: job.id,
        type: 'dead_lettered',
        message,
        metadata: { failureType, attempts: job.attempts, maxAttempts: job.max_attempts },
      });
      await createTaskNotification({
        supabase,
        ownerId: job.owner_id,
        jobId: job.id,
        fileId: job.file_id,
        kind: 'codegen_job_failed',
        title: 'Code generation failed',
        message,
      });
      await recordCloudActivity({
        supabase,
        ownerId: job.owner_id,
        actorId: job.owner_id,
        fileId: job.file_id,
        type: 'codegen_job_failed',
        metadata: { jobId: job.id, framework: job.framework, error: message, failureType },
      });
    }
    return getCodegenJobDetail({ supabase, userId: job.owner_id, jobId: job.id });
  } finally {
    stopHeartbeat();
  }
}

export async function listTaskNotifications(input: {
  supabase: SupabaseClient;
  userId: string;
  fileId?: string;
  limit: number;
  offset?: number;
}): Promise<CloudPagedResult<TaskNotification>> {
  let query = input.supabase
    .from('task_notifications')
    .select(NOTIFICATION_SELECT, { count: 'exact' })
    .eq('owner_id', input.userId);
  if (input.fileId) query = query.eq('file_id', input.fileId);

  const rows = await query
    .order('created_at', { ascending: false })
    .range(input.offset ?? 0, (input.offset ?? 0) + input.limit - 1);
  if (rows.error) throw toCodegenDbError(rows.error, 'Failed to list task notifications');
  return {
    data: (rows.data ?? []).map(mapTaskNotification),
    page: {
      total: rows.count ?? 0,
      limit: input.limit,
      offset: input.offset ?? 0,
    },
  };
}

export async function markTaskNotificationRead(input: {
  supabase: SupabaseClient;
  userId: string;
  id: string;
}) {
  const row = await input.supabase
    .from('task_notifications')
    .update({ read_at: new Date().toISOString() })
    .eq('id', input.id)
    .eq('owner_id', input.userId)
    .select(NOTIFICATION_SELECT)
    .single();
  if (row.error || !row.data) {
    if (row.error) {
      throw toCodegenDbError(row.error, 'Failed to mark task notification read');
    }
    throw createError({ statusCode: 404, statusMessage: 'Task notification not found' });
  }
  return mapTaskNotification(row.data);
}

export async function deleteTaskNotification(input: {
  supabase: SupabaseClient;
  userId: string;
  id: string;
}) {
  const deleted = await input.supabase
    .from('task_notifications')
    .delete()
    .eq('id', input.id)
    .eq('owner_id', input.userId);
  if (deleted.error) {
    throw toCodegenDbError(deleted.error, 'Failed to delete task notification');
  }
}
