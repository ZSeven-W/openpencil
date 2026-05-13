import { describe, expect, it, vi } from 'vitest';
import {
  claimNextCodegenJob,
  classifyCodegenJobFailure,
  resolveEmbeddedCodegenWorkerMode,
  getProviderCodegenFailurePolicy,
  getProviderCircuitBreaker,
  getProviderRuntimeConfig,
  heartbeatCodegenJob,
  assertProviderCanRunCodegen,
  recordProviderFailure,
  recordProviderSuccess,
  resolveCodegenFailureDisposition,
  waitForProviderCapacity,
} from '../cloud-codegen-job-queue';

const jobRow = {
  id: 'job-1',
  file_id: 'file-1',
  owner_id: 'user-1',
  generation_id: null,
  status: 'running',
  framework: 'react',
  page_id: 'page-1',
  target_kind: 'page',
  node_ids: [],
  target_hash: 'hash-1',
  document_revision: 1,
  provider: 'openai',
  model: 'gpt-5.4',
  priority: 10,
  progress: 0,
  attempts: 1,
  max_attempts: 2,
  locked_by: 'worker-1',
  locked_until: '2026-05-13T08:05:00.000Z',
  last_heartbeat_at: '2026-05-13T08:00:00.000Z',
  next_run_at: '2026-05-13T08:00:00.000Z',
  dead_lettered_at: null,
  last_error: null,
  failure_type: null,
  input_snapshot: {},
  output: {},
  error: null,
  created_at: '2026-05-13T08:00:00.000Z',
  updated_at: '2026-05-13T08:00:00.000Z',
  started_at: '2026-05-13T08:00:00.000Z',
  completed_at: null,
  canceled_at: null,
};

describe('cloud codegen job queue reliability helpers', () => {
  it('claims the next runnable job through the atomic queue RPC', async () => {
    const maybeSingle = vi.fn(async () => ({ data: jobRow, error: null }));
    const select = vi.fn(() => ({ maybeSingle }));
    const rpc = vi.fn(() => ({ select }));

    const claimed = await claimNextCodegenJob({
      supabase: { rpc } as never,
      workerId: 'worker-1',
      lockMs: 120_000,
    });

    expect(rpc).toHaveBeenCalledWith('claim_next_codegen_job', {
      p_lock_seconds: 120,
      p_worker_id: 'worker-1',
    });
    expect(select).toHaveBeenCalledWith(expect.stringContaining('dead_lettered_at'));
    expect(maybeSingle).toHaveBeenCalled();
    expect(claimed?.id).toBe('job-1');
    expect(claimed?.locked_by).toBe('worker-1');
  });

  it('treats dev:cloud disable flags as authoritative for the embedded Nitro worker', () => {
    expect(
      resolveEmbeddedCodegenWorkerMode({
        OPENPENCIL_CODEGEN_WORKER: 'disabled',
      }),
    ).toEqual({ enabled: false, reason: 'OPENPENCIL_CODEGEN_WORKER' });
    expect(
      resolveEmbeddedCodegenWorkerMode({
        OPENPENCIL_DEV_CLOUD: '1',
      }),
    ).toEqual({ enabled: false, reason: 'OPENPENCIL_DEV_CLOUD' });
    expect(
      resolveEmbeddedCodegenWorkerMode({
        VITE_OPENPENCIL_CODEGEN_WORKER: 'disabled',
      }),
    ).toEqual({ enabled: false, reason: 'VITE_OPENPENCIL_CODEGEN_WORKER' });
    expect(resolveEmbeddedCodegenWorkerMode({})).toEqual({ enabled: true, reason: null });
  });

  it('renews a running job lock through the heartbeat RPC', async () => {
    const rpc = vi.fn(async () => ({ data: true, error: null }));

    await expect(
      heartbeatCodegenJob({
        supabase: { rpc } as never,
        jobId: 'job-1',
        workerId: 'worker-1',
        lockMs: 300_000,
      }),
    ).resolves.toBe(true);

    expect(rpc).toHaveBeenCalledWith('heartbeat_codegen_job', {
      p_job_id: 'job-1',
      p_lock_seconds: 300,
      p_worker_id: 'worker-1',
    });
  });

  it('keeps retryable failures pending before the max attempt and dead-letters the last attempt', () => {
    expect(resolveCodegenFailureDisposition({ attempts: 1, max_attempts: 3 })).toBe('retry');
    expect(resolveCodegenFailureDisposition({ attempts: 3, max_attempts: 3 })).toBe(
      'dead_letter',
    );
  });

  it('classifies retry and circuit breaker relevant provider errors', () => {
    expect(classifyCodegenJobFailure(new Error('429 rate limit exceeded'))).toBe('rate_limit');
    expect(classifyCodegenJobFailure(new Error('request timed out'))).toBe('timeout');
    expect(classifyCodegenJobFailure(new Error('model output invalid'))).toBe('execution_error');
  });

  it('checks provider capacity through a rate-limit RPC', async () => {
    const rpc = vi.fn(async () => ({ data: true, error: null }));

    await expect(
      waitForProviderCapacity({
        supabase: { rpc } as never,
        provider: 'openai',
        maxPerMinute: 12,
      }),
    ).resolves.toEqual({ allowed: true, retryAfterMs: 0 });

    expect(rpc).toHaveBeenCalledWith('reserve_codegen_provider_capacity', {
      p_max_per_minute: 12,
      p_provider: 'openai',
    });
  });

  it('loads provider runtime config before enforcing dynamic rate limits', async () => {
    const maybeSingle = vi.fn(async () => ({
      data: {
        enabled: true,
        max_per_minute: 9,
        circuit_threshold: 5,
        circuit_open_ms: 120_000,
      },
      error: null,
    }));
    const selectEq = vi.fn(() => ({ maybeSingle }));
    const select = vi.fn(() => ({ eq: selectEq }));
    const rpc = vi.fn(async () => ({ data: true, error: null }));
    const from = vi.fn((table: string) => {
      if (table === 'codegen_provider_configs') return { select };
      if (table === 'codegen_provider_health') {
        return {
          select: () => ({
            eq: () => ({
              maybeSingle: vi.fn(async () => ({
                data: { circuit_open_until: null, consecutive_failures: 0 },
                error: null,
              })),
            }),
          }),
        };
      }
      throw new Error(`Unexpected table ${table}`);
    });
    const supabase = { from, rpc } as never;

    await expect(getProviderRuntimeConfig({ supabase, provider: 'openai' })).resolves.toEqual({
      enabled: true,
      maxPerMinute: 9,
      circuitThreshold: 5,
      circuitOpenMs: 120_000,
    });
    await assertProviderCanRunCodegen({ supabase, provider: 'openai' });

    expect(rpc).toHaveBeenCalledWith('reserve_codegen_provider_capacity', {
      p_max_per_minute: 9,
      p_provider: 'openai',
    });
  });

  it('uses provider config for circuit breaker failure policy', async () => {
    const maybeSingle = vi.fn(async () => ({
      data: {
        enabled: true,
        max_per_minute: 30,
        circuit_threshold: 8,
        circuit_open_ms: 180_000,
      },
      error: null,
    }));
    const selectEq = vi.fn(() => ({ maybeSingle }));
    const select = vi.fn(() => ({ eq: selectEq }));
    const from = vi.fn((table: string) => {
      if (table === 'codegen_provider_configs') return { select };
      throw new Error(`Unexpected table ${table}`);
    });

    await expect(
      getProviderCodegenFailurePolicy({
        supabase: { from } as never,
        provider: 'openai',
        fallbackThreshold: 3,
        fallbackOpenMs: 60_000,
      }),
    ).resolves.toEqual({
      threshold: 8,
      openMs: 180_000,
    });
  });

  it('records provider failures and opens the circuit after threshold', async () => {
    const upsert = vi.fn(async () => ({ data: null, error: null }));
    const update = vi.fn(() => ({ eq: vi.fn(async () => ({ data: null, error: null })) }));
    const from = vi.fn((table: string) => {
      if (table === 'codegen_provider_health') return { upsert, update };
      throw new Error(`Unexpected table ${table}`);
    });

    await recordProviderFailure({
      supabase: { from } as never,
      provider: 'openai',
      failureType: 'rate_limit',
      threshold: 3,
      openMs: 60_000,
      currentConsecutiveFailures: 2,
    });

    expect(update).toHaveBeenCalledWith(
      expect.objectContaining({
        circuit_open_until: expect.any(String),
        consecutive_failures: 3,
      }),
    );
  });

  it('resets provider failure state on success and reads circuit state', async () => {
    const eq = vi.fn(async () => ({ data: null, error: null }));
    const update = vi.fn(() => ({ eq }));
    const maybeSingle = vi.fn(async () => ({
      data: { circuit_open_until: '2026-05-13T08:01:00.000Z', consecutive_failures: 3 },
      error: null,
    }));
    const selectEq = vi.fn(() => ({ maybeSingle }));
    const select = vi.fn(() => ({ eq: selectEq }));
    const from = vi.fn((table: string) => {
      if (table === 'codegen_provider_health') return { update, select };
      throw new Error(`Unexpected table ${table}`);
    });

    await recordProviderSuccess({ supabase: { from } as never, provider: 'openai' });
    await expect(
      getProviderCircuitBreaker({ supabase: { from } as never, provider: 'openai' }),
    ).resolves.toEqual({
      circuitOpenUntil: '2026-05-13T08:01:00.000Z',
      consecutiveFailures: 3,
    });
  });
});
