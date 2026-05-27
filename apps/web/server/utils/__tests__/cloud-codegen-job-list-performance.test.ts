import { describe, expect, it, vi } from 'vitest';
import { getCodegenJobDetail, listCodegenJobs } from '../cloud-codegen-jobs';
import { getCodegenWorkerOverview } from '../cloud-codegen-worker-management';

const SUMMARY_FIELDS = [
  'id',
  'file_id',
  'owner_id',
  'generation_id',
  'file_name',
  'page_name',
  'job_kind',
  'status',
  'framework',
  'page_id',
  'target_kind',
  'node_ids',
  'target_hash',
  'document_revision',
  'provider',
  'model',
  'priority',
  'progress',
  'attempts',
  'max_attempts',
  'locked_by',
  'locked_until',
  'last_heartbeat_at',
  'next_run_at',
  'dead_lettered_at',
  'last_error',
  'failure_type',
  'error',
  'created_at',
  'updated_at',
  'started_at',
  'completed_at',
  'canceled_at',
];

function expectSummarySelect(selectArg: string) {
  expect(selectArg).not.toContain('input_snapshot');
  expect(selectArg).not.toContain('output');
  for (const field of SUMMARY_FIELDS) {
    expect(selectArg.split(',')).toContain(field);
  }
}

describe('cloud codegen list query performance', () => {
  it('lists codegen jobs without loading heavy input snapshots or output payloads', async () => {
    const range = vi.fn(async () => ({ data: [], error: null }));
    const order = vi.fn(() => ({ range }));
    const eqOwner = vi.fn(() => ({ order }));
    const select = vi.fn((_fields: string) => ({ eq: eqOwner }));
    const from = vi.fn((_table: string) => ({ select }));

    await listCodegenJobs({
      supabase: { from } as never,
      userId: 'user-1',
      limit: 50,
    });

    expect(from).toHaveBeenCalledWith('codegen_jobs');
    expectSummarySelect(select.mock.calls[0][0]);
    expect(eqOwner).toHaveBeenCalledWith('owner_id', 'user-1');
    expect(range).toHaveBeenCalledWith(0, 49);
  });

  it('loads dead-lettered worker overview jobs with the same lightweight summary fields', async () => {
    const failedRange = vi.fn(async () => ({ data: [], error: null, count: 0 }));
    const failedOrder = vi.fn(() => ({ range: failedRange }));
    const failedNot = vi.fn(() => ({ order: failedOrder }));
    const failedEqStatus = vi.fn(() => ({ not: failedNot }));
    const failedEqOwner = vi.fn(() => ({ eq: failedEqStatus }));
    const failedSelect = vi.fn((_fields: string, _options?: unknown) => ({ eq: failedEqOwner }));

    const metricsLimit = vi.fn(async () => ({ data: [], error: null }));
    const metricsOrder = vi.fn(() => ({ limit: metricsLimit }));
    const metricsEqOwner = vi.fn(() => ({ order: metricsOrder }));
    const metricsSelect = vi.fn((_fields: string) => ({ eq: metricsEqOwner }));

    const from = vi.fn((table: string) => {
      expect(table).toBe('codegen_jobs');
      return {
        select: vi.fn((fields: string, options?: unknown) => {
          if (options) return failedSelect(fields, options);
          return metricsSelect(fields);
        }),
      };
    });

    await getCodegenWorkerOverview({
      supabase: { from } as never,
      adminSupabase: null,
      userId: 'user-1',
    });

    expect(failedSelect).toHaveBeenCalled();
    expectSummarySelect(failedSelect.mock.calls[0][0]);
    expect(failedSelect.mock.calls[0][1]).toEqual({ count: 'exact' });
  });

  it('skips provider and failed-job page queries for summary overviews', async () => {
    const metricsLimit = vi.fn(async () => ({ data: [], error: null }));
    const metricsOrder = vi.fn(() => ({ limit: metricsLimit }));
    const metricsEqOwner = vi.fn(() => ({ order: metricsOrder }));
    const metricsSelect = vi.fn((_fields: string) => ({ eq: metricsEqOwner }));
    const from = vi.fn((table: string) => {
      expect(table).toBe('codegen_jobs');
      return { select: metricsSelect };
    });

    const workerSelect = vi.fn(() => ({
      order: vi.fn(() => ({
        limit: vi.fn(async () => ({ data: [], error: null })),
      })),
    }));

    const overview = await getCodegenWorkerOverview({
      supabase: { from } as never,
      adminSupabase: {
        from: vi.fn((table: string) => {
          expect(table).toBe('codegen_worker_heartbeats');
          return { select: workerSelect };
        }),
      } as never,
      userId: 'user-1',
      summary: true,
    });

    expect(overview.workers).toEqual([]);
    expect(overview.providers).toEqual([]);
    expect(overview.failedJobs).toEqual([]);
    expect(from).toHaveBeenCalledTimes(1);
    expect(workerSelect).toHaveBeenCalledWith(
      'worker_id,hostname,pid,metadata,started_at,last_heartbeat_at',
    );
  });

  it('loads codegen job detail with bounded step and event queries', async () => {
    const jobRow = {
      id: 'job-1',
      file_id: 'file-1',
      owner_id: 'user-1',
      generation_id: null,
      file_name: 'Landing',
      page_name: 'Home',
      pipeline_mode: 'direct_generation',
      job_kind: 'full_generation',
      status: 'succeeded',
      framework: 'html',
      page_id: 'page-1',
      target_kind: 'page',
      node_ids: [],
      target_hash: 'hash-1',
      document_revision: 1,
      provider: 'openai',
      model: 'gpt-5.4',
      priority: 0,
      progress: 100,
      attempts: 1,
      max_attempts: 2,
      locked_by: null,
      locked_until: null,
      last_heartbeat_at: null,
      next_run_at: null,
      dead_lettered_at: null,
      last_error: null,
      failure_type: null,
      input_snapshot: {},
      output: {},
      error: null,
      created_at: '2026-05-13T08:00:00.000Z',
      updated_at: '2026-05-13T08:00:01.000Z',
      started_at: '2026-05-13T08:00:00.000Z',
      completed_at: '2026-05-13T08:00:01.000Z',
      canceled_at: null,
    };

    const single = vi.fn(async () => ({ data: jobRow, error: null }));
    const eqOwner = vi.fn(() => ({ single }));
    const eqId = vi.fn(() => ({ eq: eqOwner }));
    const jobSelect = vi.fn(() => ({ eq: eqId }));

    const stepLimit = vi.fn(async () => ({ data: [], error: null }));
    const stepOrder = vi.fn(() => ({ limit: stepLimit }));
    const stepEq = vi.fn(() => ({ order: stepOrder }));
    const stepSelect = vi.fn(() => ({ eq: stepEq }));

    const eventLimit = vi.fn(async () => ({ data: [], error: null }));
    const eventOrder = vi.fn(() => ({ limit: eventLimit }));
    const eventEq = vi.fn(() => ({ order: eventOrder }));
    const eventSelect = vi.fn(() => ({ eq: eventEq }));

    const from = vi.fn((table: string) => {
      if (table === 'codegen_jobs') return { select: jobSelect };
      if (table === 'codegen_job_steps') return { select: stepSelect };
      if (table === 'codegen_job_events') return { select: eventSelect };
      throw new Error(`Unexpected table ${table}`);
    });

    const detail = await getCodegenJobDetail({
      supabase: { from } as never,
      userId: 'user-1',
      jobId: 'job-1',
    });

    expect(detail.id).toBe('job-1');
    expect(stepOrder).toHaveBeenCalledWith('created_at', { ascending: true });
    expect(stepLimit).toHaveBeenCalledWith(120);
    expect(eventOrder).toHaveBeenCalledWith('created_at', { ascending: false });
    expect(eventLimit).toHaveBeenCalledWith(100);
  });
});
