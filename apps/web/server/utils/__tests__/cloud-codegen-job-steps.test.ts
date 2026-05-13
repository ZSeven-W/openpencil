import { describe, expect, it, vi } from 'vitest';
import { upsertCodegenJobStep } from '../cloud-codegen-jobs';

describe('cloud codegen job step writes', () => {
  it('updates an existing role step for the same job attempt instead of inserting duplicates', async () => {
    const maybeSingle = vi.fn(async () => ({
      data: {
        id: 'step-1',
        started_at: '2026-05-13T08:00:00.000Z',
        input: { previous: true },
        output: {},
      },
      error: null,
    }));
    const selectEqAttempt = vi.fn(() => ({ maybeSingle }));
    const selectEqRole = vi.fn(() => ({ eq: selectEqAttempt }));
    const selectEqJob = vi.fn(() => ({ eq: selectEqRole }));
    const select = vi.fn(() => ({ eq: selectEqJob }));
    const updateEq = vi.fn(async () => ({ data: null, error: null }));
    const update = vi.fn(() => ({ eq: updateEq }));
    const insert = vi.fn(async () => ({ data: null, error: null }));
    const from = vi.fn((table: string) => {
      expect(table).toBe('codegen_job_steps');
      return { select, update, insert };
    });

    await upsertCodegenJobStep({
      supabase: { from } as never,
      jobId: 'job-1',
      attempt: 2,
      agentRole: 'planner',
      status: 'succeeded',
      progress: 100,
      data: { next: true },
    });

    expect(insert).not.toHaveBeenCalled();
    expect(update).toHaveBeenCalledWith(
      expect.objectContaining({
        attempt: 2,
        status: 'succeeded',
        progress: 100,
        started_at: '2026-05-13T08:00:00.000Z',
        completed_at: expect.any(String),
        input: { previous: true, next: true },
      }),
    );
    expect(updateEq).toHaveBeenCalledWith('id', 'step-1');
  });

  it('inserts the first role step for a job attempt', async () => {
    const maybeSingle = vi.fn(async () => ({ data: null, error: null }));
    const selectEqAttempt = vi.fn(() => ({ maybeSingle }));
    const selectEqRole = vi.fn(() => ({ eq: selectEqAttempt }));
    const selectEqJob = vi.fn(() => ({ eq: selectEqRole }));
    const select = vi.fn(() => ({ eq: selectEqJob }));
    const update = vi.fn();
    const insert = vi.fn(async () => ({ data: null, error: null }));
    const from = vi.fn(() => ({ select, update, insert }));

    await upsertCodegenJobStep({
      supabase: { from } as never,
      jobId: 'job-1',
      attempt: 1,
      agentRole: 'page_codegen',
      status: 'running',
      data: { section: 'hero' },
    });

    expect(update).not.toHaveBeenCalled();
    expect(insert).toHaveBeenCalledWith(
      expect.objectContaining({
        job_id: 'job-1',
        attempt: 1,
        agent_role: 'page_codegen',
        status: 'running',
        progress: 0,
        input: { section: 'hero' },
        started_at: expect.any(String),
        completed_at: null,
      }),
    );
  });
});
