import type { SupabaseClient } from '@supabase/supabase-js';
import { createError } from 'h3';
import type {
  CloudCodegenJob,
  CodegenJobAgentRole,
  CodegenJobStatus,
  CreateCloudCodegenJobInput,
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
import { generateCode, type CodegenTextCollector } from '../../src/services/ai/code-generation-pipeline';
import { buildCodegenFiles } from '../../src/services/ai/codegen-files';
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

const JOB_SELECT =
  'id,file_id,owner_id,generation_id,job_kind,status,framework,page_id,target_kind,node_ids,target_hash,document_revision,provider,model,priority,progress,attempts,max_attempts,locked_by,locked_until,last_heartbeat_at,next_run_at,dead_lettered_at,last_error,failure_type,input_snapshot,output,error,created_at,updated_at,started_at,completed_at,canceled_at';
const STEP_SELECT =
  'id,job_id,agent_role,attempt,status,progress,input,output,error,started_at,completed_at,created_at,updated_at';
const EVENT_SELECT = 'id,job_id,type,message,metadata,created_at';
const NOTIFICATION_SELECT =
  'id,owner_id,job_id,file_id,generation_id,kind,title,message,read_at,created_at';

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
      .order('created_at', { ascending: true }),
    input.supabase
      .from('codegen_job_events')
      .select(EVENT_SELECT)
      .eq('job_id', input.jobId)
      .order('created_at', { ascending: true }),
  ]);
  if (steps.error) throw toCodegenDbError(steps.error, 'Failed to load codegen job steps');
  if (events.error) throw toCodegenDbError(events.error, 'Failed to load codegen job events');
  return { steps: steps.data ?? [], events: events.data ?? [] };
}

export async function getCodegenJobDetail(input: {
  supabase: SupabaseClient;
  userId: string;
  jobId: string;
}): Promise<CloudCodegenJob> {
  const job = await input.supabase
    .from('codegen_jobs')
    .select(JOB_SELECT)
    .eq('id', input.jobId)
    .eq('owner_id', input.userId)
    .single();

  if (job.error || !job.data) {
    throw createError({ statusCode: 404, statusMessage: 'Codegen job not found' });
  }

  return mapCodegenJob(job.data, await loadJobParts(input));
}

export async function listCodegenJobs(input: {
  supabase: SupabaseClient;
  userId: string;
  fileId?: string;
  status?: CodegenJobStatus;
  active?: boolean;
  deadLettered?: boolean;
  limit: number;
}): Promise<CloudCodegenJob[]> {
  let query = input.supabase
    .from('codegen_jobs')
    .select(JOB_SELECT)
    .eq('owner_id', input.userId);

  if (input.fileId) query = query.eq('file_id', input.fileId);
  if (input.status) query = query.eq('status', input.status);
  if (input.active) query = query.in('status', ['pending', 'running']);
  if (input.deadLettered === true) query = query.not('dead_lettered_at', 'is', null);
  if (input.deadLettered === false) query = query.is('dead_lettered_at', null);

  const jobs = await query.order('created_at', { ascending: false }).limit(input.limit);
  if (jobs.error) throw toCodegenDbError(jobs.error, 'Failed to list codegen jobs');
  return (jobs.data ?? []).map((row) => mapCodegenJob(row));
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
    .select('id,started_at,completed_at,input,output')
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
  const base = {
    job_id: input.jobId,
    attempt,
    agent_role: input.agentRole,
    status,
    progress,
    error: input.error ?? null,
    input: input.data ? { ...existingInput, ...input.data } : existingInput,
    output: input.output ? { ...existingOutput, ...input.output } : existingOutput,
    started_at:
      existing.data?.started_at ?? (status === 'running' || terminal ? now : null),
    completed_at:
      terminal
        ? existing.data?.completed_at ?? now
        : status === 'running'
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
    .select(JOB_SELECT)
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
        baseGenerationId: input.baseGenerationId ?? null,
        patchInstruction: input.patchInstruction?.trim() ?? null,
      },
    })
    .select(JOB_SELECT)
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

function serverCollectText(): CodegenTextCollector {
  return async (system, message, model, provider, abortSignal) => {
    if (!isServerProvider(provider ?? '')) {
      throw new Error('Background jobs require a server-side AI provider.');
    }
    const { createChatCompletionText } = await import('./server-codegen-provider');
    return createChatCompletionText({ system, message, model, provider, abortSignal });
  };
}

export async function runCodegenWorkerOnce(
  supabase: SupabaseClient,
  options: CodegenWorkerRuntimeOptions = {},
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
    baseGenerationId?: string | null;
    patchInstruction?: string | null;
  };
  const provider = job.provider;
  const jobKind = job.job_kind ?? 'full_generation';
  const stepAttempt = Math.max(1, Math.trunc(Number(job.attempts ?? 1)));

  try {
    await assertProviderCanRunCodegen({
      supabase,
      provider,
      maxPerMinute: workerOptions.providerRateLimitPerMinute,
    }).catch((err) => {
      throw toCodegenDbError(err, 'Failed to reserve codegen provider capacity');
    });
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
    } =
      jobKind === 'patch_generation'
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
              collectText: serverCollectText(),
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
              if (event.step === 'planning' && event.status === 'done') {
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
            { collectText: serverCollectText() },
          );

    await assertNotCanceled();
    await heartbeat();
    const files =
      'files' in result && result.files?.length
        ? result.files
        : buildCodegenFiles({ framework: job.framework, code: result.code });
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
        output: {
          generationId: generation.id,
          degraded: result.degraded,
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
  limit: number;
}) {
  const rows = await input.supabase
    .from('task_notifications')
    .select(NOTIFICATION_SELECT)
    .eq('owner_id', input.userId)
    .order('created_at', { ascending: false })
    .limit(input.limit);
  if (rows.error) throw toCodegenDbError(rows.error, 'Failed to list task notifications');
  return (rows.data ?? []).map(mapTaskNotification);
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
