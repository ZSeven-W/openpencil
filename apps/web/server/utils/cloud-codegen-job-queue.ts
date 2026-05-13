import type { SupabaseClient } from '@supabase/supabase-js';
import type { Framework } from '@zseven-w/pen-types';

const JOB_SELECT =
  'id,file_id,owner_id,generation_id,job_kind,status,framework,page_id,target_kind,node_ids,target_hash,document_revision,provider,model,priority,progress,attempts,max_attempts,locked_by,locked_until,last_heartbeat_at,next_run_at,dead_lettered_at,last_error,failure_type,input_snapshot,output,error,created_at,updated_at,started_at,completed_at,canceled_at';

export const DEFAULT_CODEGEN_WORKER_ID = `nitro-${process.pid}`;
export const DEFAULT_CODEGEN_LOCK_MS = 5 * 60_000;
export const DEFAULT_CODEGEN_RETRY_DELAY_MS = 30_000;
export const DEFAULT_PROVIDER_RATE_LIMIT_PER_MINUTE = 30;
export const DEFAULT_PROVIDER_CIRCUIT_THRESHOLD = 3;
export const DEFAULT_PROVIDER_CIRCUIT_OPEN_MS = 60_000;

export type CodegenFailureType =
  | 'rate_limit'
  | 'timeout'
  | 'provider_error'
  | 'canceled'
  | 'execution_error';

export type CodegenFailureDisposition = 'retry' | 'dead_letter';

export interface CodegenJobQueueRow {
  id: string;
  file_id: string;
  owner_id: string;
  generation_id: string | null;
  job_kind?: 'full_generation' | 'patch_generation' | null;
  status: 'pending' | 'running' | 'succeeded' | 'failed' | 'canceled';
  framework: Framework;
  page_id: string;
  target_kind: 'page' | 'selection';
  node_ids: string[];
  target_hash: string;
  document_revision: number;
  provider: string | null;
  model: string | null;
  priority: number;
  progress: number;
  attempts: number;
  max_attempts: number;
  locked_by: string | null;
  locked_until: string | null;
  last_heartbeat_at?: string | null;
  next_run_at?: string | null;
  dead_lettered_at?: string | null;
  last_error?: string | null;
  failure_type?: CodegenFailureType | null;
  input_snapshot?: Record<string, unknown> | null;
  output?: Record<string, unknown> | null;
  error: string | null;
  created_at: string;
  updated_at: string;
  started_at: string | null;
  completed_at: string | null;
  canceled_at: string | null;
}

export interface CodegenWorkerRuntimeOptions {
  workerId?: string;
  lockMs?: number;
  retryDelayMs?: number;
  heartbeatMs?: number;
  providerRateLimitPerMinute?: number;
  providerCircuitThreshold?: number;
  providerCircuitOpenMs?: number;
}

export function resolveEmbeddedCodegenWorkerMode(
  env: NodeJS.ProcessEnv = process.env,
): { enabled: boolean; reason: string | null } {
  if (env.OPENPENCIL_CODEGEN_WORKER === 'disabled') {
    return { enabled: false, reason: 'OPENPENCIL_CODEGEN_WORKER' };
  }
  if (env.VITE_OPENPENCIL_CODEGEN_WORKER === 'disabled') {
    return { enabled: false, reason: 'VITE_OPENPENCIL_CODEGEN_WORKER' };
  }
  if (env.OPENPENCIL_DEV_CLOUD === '1') {
    return { enabled: false, reason: 'OPENPENCIL_DEV_CLOUD' };
  }
  return { enabled: true, reason: null };
}

interface CodegenProviderRuntimeConfig {
  enabled: boolean;
  maxPerMinute: number;
  circuitThreshold: number;
  circuitOpenMs: number;
}

export function resolveCodegenWorkerOptions(
  options: CodegenWorkerRuntimeOptions = {},
): Required<CodegenWorkerRuntimeOptions> {
  return {
    workerId: options.workerId ?? DEFAULT_CODEGEN_WORKER_ID,
    lockMs: options.lockMs ?? DEFAULT_CODEGEN_LOCK_MS,
    retryDelayMs: options.retryDelayMs ?? DEFAULT_CODEGEN_RETRY_DELAY_MS,
    heartbeatMs: options.heartbeatMs ?? Math.max(5_000, Math.floor((options.lockMs ?? DEFAULT_CODEGEN_LOCK_MS) / 3)),
    providerRateLimitPerMinute:
      options.providerRateLimitPerMinute ?? DEFAULT_PROVIDER_RATE_LIMIT_PER_MINUTE,
    providerCircuitThreshold:
      options.providerCircuitThreshold ?? DEFAULT_PROVIDER_CIRCUIT_THRESHOLD,
    providerCircuitOpenMs: options.providerCircuitOpenMs ?? DEFAULT_PROVIDER_CIRCUIT_OPEN_MS,
  };
}

function normalizeProviderRuntimeConfig(row?: {
  enabled?: boolean | null;
  max_per_minute?: number | null;
  circuit_threshold?: number | null;
  circuit_open_ms?: number | null;
} | null): CodegenProviderRuntimeConfig {
  return {
    enabled: row?.enabled ?? true,
    maxPerMinute: Number(row?.max_per_minute ?? DEFAULT_PROVIDER_RATE_LIMIT_PER_MINUTE),
    circuitThreshold: Number(row?.circuit_threshold ?? DEFAULT_PROVIDER_CIRCUIT_THRESHOLD),
    circuitOpenMs: Number(row?.circuit_open_ms ?? DEFAULT_PROVIDER_CIRCUIT_OPEN_MS),
  };
}

function secondsFromMs(ms: number): number {
  return Math.max(1, Math.ceil(ms / 1000));
}

export function classifyCodegenJobFailure(error: unknown): CodegenFailureType {
  const message = error instanceof Error ? error.message : String(error ?? '');
  const text = message.toLowerCase();
  if (text.includes('abort') || text.includes('cancel')) return 'canceled';
  if (text.includes('lock was lost')) return 'canceled';
  if (text.includes('rate limit') || text.includes('429') || text.includes('quota')) {
    return 'rate_limit';
  }
  if (text.includes('timeout') || text.includes('timed out') || text.includes('deadline')) {
    return 'timeout';
  }
  if (
    text.includes('provider') ||
    text.includes('api key') ||
    text.includes('upstream') ||
    text.includes('service unavailable')
  ) {
    return 'provider_error';
  }
  return 'execution_error';
}

export function resolveCodegenFailureDisposition(input: {
  attempts: number;
  max_attempts: number;
}): CodegenFailureDisposition {
  return Number(input.attempts) < Number(input.max_attempts) ? 'retry' : 'dead_letter';
}

export async function claimNextCodegenJob(input: {
  supabase: SupabaseClient;
  workerId: string;
  lockMs: number;
}): Promise<CodegenJobQueueRow | null> {
  const row = await input.supabase
    .rpc('claim_next_codegen_job', {
      p_lock_seconds: secondsFromMs(input.lockMs),
      p_worker_id: input.workerId,
    })
    .select(JOB_SELECT)
    .maybeSingle();

  if (row.error) {
    throw row.error;
  }
  return (row.data as CodegenJobQueueRow | null) ?? null;
}

export async function requeueStaleCodegenJobs(input: { supabase: SupabaseClient }): Promise<number> {
  const result = await input.supabase.rpc('requeue_stale_codegen_jobs');
  if (result.error) {
    throw result.error;
  }
  return Number(result.data ?? 0);
}

export async function heartbeatCodegenJob(input: {
  supabase: SupabaseClient;
  jobId: string;
  workerId: string;
  lockMs: number;
}): Promise<boolean> {
  const result = await input.supabase.rpc('heartbeat_codegen_job', {
    p_job_id: input.jobId,
    p_lock_seconds: secondsFromMs(input.lockMs),
    p_worker_id: input.workerId,
  });
  if (result.error) {
    throw result.error;
  }
  return Boolean(result.data);
}

export async function waitForProviderCapacity(input: {
  supabase: SupabaseClient;
  provider: string;
  maxPerMinute?: number;
}): Promise<{ allowed: boolean; retryAfterMs: number }> {
  const result = await input.supabase.rpc('reserve_codegen_provider_capacity', {
    p_max_per_minute: input.maxPerMinute ?? DEFAULT_PROVIDER_RATE_LIMIT_PER_MINUTE,
    p_provider: input.provider,
  });
  if (result.error) {
    throw result.error;
  }
  const allowed = Boolean(result.data);
  return { allowed, retryAfterMs: allowed ? 0 : 60_000 };
}

export async function getProviderRuntimeConfig(input: {
  supabase: SupabaseClient;
  provider: string;
}): Promise<CodegenProviderRuntimeConfig> {
  const row = await input.supabase
    .from('codegen_provider_configs')
    .select('enabled,max_per_minute,circuit_threshold,circuit_open_ms')
    .eq('provider', input.provider)
    .maybeSingle();
  if (row.error) {
    throw row.error;
  }
  return normalizeProviderRuntimeConfig(row.data);
}

export async function recordProviderFailure(input: {
  supabase: SupabaseClient;
  provider: string;
  failureType: CodegenFailureType;
  threshold?: number;
  openMs?: number;
  currentConsecutiveFailures?: number;
  currentFailedRequests?: number;
  message?: string;
}) {
  const now = new Date();
  const nextFailures = Number(input.currentConsecutiveFailures ?? 0) + 1;
  const nextFailedRequests = Number(input.currentFailedRequests ?? 0) + 1;
  const threshold = input.threshold ?? DEFAULT_PROVIDER_CIRCUIT_THRESHOLD;
  const circuitOpenUntil =
    nextFailures >= threshold
      ? new Date(now.getTime() + (input.openMs ?? DEFAULT_PROVIDER_CIRCUIT_OPEN_MS)).toISOString()
      : null;

  const inserted = await input.supabase.from('codegen_provider_health').upsert({
    provider: input.provider,
    updated_at: now.toISOString(),
  });
  if (inserted.error) {
    throw inserted.error;
  }

  const updated = await input.supabase
    .from('codegen_provider_health')
    .update({
      circuit_open_until: circuitOpenUntil,
      consecutive_failures: nextFailures,
      failed_requests: nextFailedRequests,
      last_error: input.message ?? null,
      last_failure_type: input.failureType,
      updated_at: now.toISOString(),
    })
    .eq('provider', input.provider);
  if (updated.error) {
    throw updated.error;
  }
}

export async function recordProviderSuccess(input: {
  supabase: SupabaseClient;
  provider: string;
}) {
  const updated = await input.supabase
    .from('codegen_provider_health')
    .update({
      circuit_open_until: null,
      consecutive_failures: 0,
      last_error: null,
      last_failure_type: null,
      updated_at: new Date().toISOString(),
    })
    .eq('provider', input.provider);
  if (updated.error) {
    throw updated.error;
  }
}

export async function getProviderCircuitBreaker(input: {
  supabase: SupabaseClient;
  provider: string;
}): Promise<{ circuitOpenUntil: string | null; consecutiveFailures: number }> {
  const row = await input.supabase
    .from('codegen_provider_health')
    .select('circuit_open_until,consecutive_failures')
    .eq('provider', input.provider)
    .maybeSingle();
  if (row.error) {
    throw row.error;
  }
  return {
    circuitOpenUntil: row.data?.circuit_open_until ?? null,
    consecutiveFailures: Number(row.data?.consecutive_failures ?? 0),
  };
}

async function getProviderFailureCounters(input: {
  supabase: SupabaseClient;
  provider: string;
}): Promise<{ consecutiveFailures: number; failedRequests: number }> {
  const row = await input.supabase
    .from('codegen_provider_health')
    .select('consecutive_failures,failed_requests')
    .eq('provider', input.provider)
    .maybeSingle();
  if (row.error) {
    throw row.error;
  }
  return {
    consecutiveFailures: Number(row.data?.consecutive_failures ?? 0),
    failedRequests: Number(row.data?.failed_requests ?? 0),
  };
}

export async function assertProviderCanRunCodegen(input: {
  supabase: SupabaseClient;
  provider: string | null;
  maxPerMinute?: number;
}) {
  if (!input.provider) return;
  const config = await getProviderRuntimeConfig({
    supabase: input.supabase,
    provider: input.provider,
  }).catch(() => normalizeProviderRuntimeConfig({
    max_per_minute: input.maxPerMinute,
  }));
  if (!config.enabled) {
    throw new Error(`Provider ${input.provider} is disabled for background codegen.`);
  }
  const circuit = await getProviderCircuitBreaker({
    supabase: input.supabase,
    provider: input.provider,
  });
  const circuitOpenUntil = circuit.circuitOpenUntil
    ? new Date(circuit.circuitOpenUntil).getTime()
    : 0;
  if (Number.isFinite(circuitOpenUntil) && circuitOpenUntil > Date.now()) {
    throw new Error(`Provider circuit open until ${circuit.circuitOpenUntil}`);
  }
  const capacity = await waitForProviderCapacity({
    supabase: input.supabase,
    provider: input.provider,
    maxPerMinute: input.maxPerMinute ?? config.maxPerMinute,
  });
  if (!capacity.allowed) {
    throw new Error('Provider rate limit capacity exhausted.');
  }
}

export async function getProviderCodegenFailurePolicy(input: {
  supabase: SupabaseClient;
  provider: string | null;
  fallbackThreshold: number;
  fallbackOpenMs: number;
}): Promise<{ threshold: number; openMs: number }> {
  if (!input.provider) {
    return { threshold: input.fallbackThreshold, openMs: input.fallbackOpenMs };
  }
  const config = await getProviderRuntimeConfig({
    supabase: input.supabase,
    provider: input.provider,
  }).catch(() => normalizeProviderRuntimeConfig({
    circuit_threshold: input.fallbackThreshold,
    circuit_open_ms: input.fallbackOpenMs,
  }));
  return {
    threshold: config.circuitThreshold,
    openMs: config.circuitOpenMs,
  };
}

export async function recordProviderCodegenFailure(input: {
  supabase: SupabaseClient;
  provider: string | null;
  failureType: CodegenFailureType;
  threshold: number;
  openMs: number;
  message: string;
}) {
  if (!input.provider || input.failureType === 'canceled') return;
  const counters = await getProviderFailureCounters({
    supabase: input.supabase,
    provider: input.provider,
  }).catch(() => ({
    consecutiveFailures: 0,
    failedRequests: 0,
  }));
  await recordProviderFailure({
    supabase: input.supabase,
    provider: input.provider,
    failureType: input.failureType,
    threshold: input.threshold,
    openMs: input.openMs,
    currentConsecutiveFailures: counters.consecutiveFailures,
    currentFailedRequests: counters.failedRequests,
    message: input.message,
  });
}

export async function recordProviderCodegenSuccess(input: {
  supabase: SupabaseClient;
  provider: string | null;
}) {
  if (!input.provider) return;
  await recordProviderSuccess({
    supabase: input.supabase,
    provider: input.provider,
  });
}

export async function isCodegenJobCanceled(input: {
  supabase: SupabaseClient;
  jobId: string;
}): Promise<boolean> {
  const row = await input.supabase
    .from('codegen_jobs')
    .select('status')
    .eq('id', input.jobId)
    .maybeSingle();
  if (row.error) {
    throw row.error;
  }
  return row.data?.status === 'canceled';
}

export async function recordCodegenJobRetry(input: {
  supabase: SupabaseClient;
  jobId: string;
  message: string;
  failureType: CodegenFailureType;
  retryDelayMs: number;
}) {
  const nextRunAt = new Date(Date.now() + input.retryDelayMs).toISOString();
  const updated = await input.supabase
    .from('codegen_jobs')
    .update({
      status: 'pending',
      locked_by: null,
      locked_until: null,
      last_heartbeat_at: null,
      next_run_at: nextRunAt,
      last_error: input.message,
      failure_type: input.failureType,
      error: input.message,
      updated_at: new Date().toISOString(),
    })
    .eq('id', input.jobId)
    .eq('status', 'running');
  if (updated.error) {
    throw updated.error;
  }
}

export async function recordCodegenJobDeadLetter(input: {
  supabase: SupabaseClient;
  jobId: string;
  message: string;
  failureType: CodegenFailureType;
}) {
  const now = new Date().toISOString();
  const updated = await input.supabase
    .from('codegen_jobs')
    .update({
      status: 'failed',
      progress: 100,
      error: input.message,
      last_error: input.message,
      failure_type: input.failureType,
      dead_lettered_at: now,
      completed_at: now,
      locked_by: null,
      locked_until: null,
      last_heartbeat_at: null,
      updated_at: now,
    })
    .eq('id', input.jobId);
  if (updated.error) {
    throw updated.error;
  }
}
