import { describe, expect, it, vi } from 'vitest';
import type { CodegenCheckpoint } from '@zseven-w/pen-types';
import {
  appendCodegenCheckpoint,
  runCodegenCheckpointResume,
  runCodegenWorkerOnce,
  upsertCodegenJobStep,
} from '../cloud-codegen-jobs';

describe('cloud codegen job step writes', () => {
  it('updates an existing role step for the same job attempt instead of inserting duplicates', async () => {
    const maybeSingle = vi.fn(async () => ({
      data: {
        id: 'step-1',
        status: 'running',
        progress: 30,
        started_at: '2026-05-13T08:00:00.000Z',
        completed_at: null,
        input: { previous: true },
        output: {},
        error: null,
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

  it('does not regress a terminal role step when a stale running write arrives', async () => {
    const maybeSingle = vi.fn(async () => ({
      data: {
        id: 'step-1',
        status: 'succeeded',
        progress: 100,
        started_at: '2026-05-13T08:00:00.000Z',
        completed_at: '2026-05-13T08:00:05.000Z',
        input: { previous: true },
        output: { report: { status: 'passed' } },
        error: null,
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
    const from = vi.fn(() => ({ select, update, insert }));

    await upsertCodegenJobStep({
      supabase: { from } as never,
      jobId: 'job-1',
      attempt: 1,
      agentRole: 'quality_check',
      status: 'running',
      progress: 78,
      output: { latestCheckpoint: { stage: 'quality_check' } },
      error: 'stale transient error',
    });

    expect(insert).not.toHaveBeenCalled();
    expect(update).toHaveBeenCalledWith(
      expect.objectContaining({
        status: 'succeeded',
        progress: 100,
        completed_at: '2026-05-13T08:00:05.000Z',
        error: null,
        input: { previous: true },
        output: {
          report: { status: 'passed' },
          latestCheckpoint: { stage: 'quality_check' },
        },
      }),
    );
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

  it('stores one checkpoint per stage and attempt in job output', async () => {
    const maybeSingle = vi.fn(async () => ({
      data: {
        output: {
          checkpoints: [
            {
              stage: 'quality_check',
              status: 'failed',
              attempt: 1,
              data: { report: { old: true } },
              createdAt: '2026-05-13T08:00:00.000Z',
            },
            {
              stage: 'assembly',
              status: 'succeeded',
              attempt: 1,
              data: { code: '<html />' },
              createdAt: '2026-05-13T08:00:00.000Z',
            },
          ],
        },
      },
      error: null,
    }));
    const selectEq = vi.fn(() => ({ maybeSingle }));
    const select = vi.fn(() => ({ eq: selectEq }));
    const updateSingle = vi.fn(async () => ({
      data: {
        id: 'job-1',
        file_id: 'file-1',
        owner_id: 'user-1',
        generation_id: null,
        status: 'running',
        framework: 'html',
        page_id: 'page-1',
        target_kind: 'page',
        node_ids: [],
        target_hash: 'hash-1',
        document_revision: 1,
        provider: 'openai',
        model: 'gpt-5.4',
        priority: 0,
        progress: 0,
        attempts: 1,
        max_attempts: 2,
        locked_by: null,
        locked_until: null,
        input_snapshot: {},
        output: {},
        error: null,
        created_at: '2026-05-13T08:00:00.000Z',
        updated_at: '2026-05-13T08:00:00.000Z',
        started_at: null,
        completed_at: null,
        canceled_at: null,
      },
      error: null,
    }));
    const updateEq = vi.fn(() => ({ select: vi.fn(() => ({ single: updateSingle })) }));
    const update = vi.fn(() => ({ eq: updateEq }));
    const from = vi.fn((table: string) => {
      expect(table).toBe('codegen_jobs');
      return { select, update };
    });

    await appendCodegenCheckpoint({
      supabase: { from } as never,
      jobId: 'job-1',
      attempt: 1,
      stage: 'quality_check',
      status: 'succeeded',
      data: { report: { new: true } },
    });

    expect(update).toHaveBeenCalled();
    const patch = vi.mocked(update).mock.calls.at(0)?.at(0) as unknown as {
      output: Record<string, unknown>;
    };
    expect(patch.output.checkpoints).toHaveLength(2);
    expect(patch.output.checkpoints).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ stage: 'assembly', status: 'succeeded' }),
        expect.objectContaining({
          stage: 'quality_check',
          status: 'succeeded',
          data: { report: { new: true } },
        }),
      ]),
    );
    expect(patch.output.latestCheckpoint).toMatchObject({
      stage: 'quality_check',
      status: 'succeeded',
      data: { report: { new: true } },
    });
  });

  it('reruns quality_check from assembly checkpoint without calling a model', async () => {
    const collectText = vi.fn();
    const supabase = createCheckpointResumeSupabase({
      checkpoints: [
        checkpoint('assembly', {
          code: '<!doctype html><html><head><style>.hero{}</style></head><body>Ship</body></html>',
          files: [
            {
              path: 'design.html',
              language: 'html',
              role: 'entry',
              content: '<!doctype html><html><head><style>.hero{}</style></head><body>Ship</body></html>',
            },
          ],
        }),
      ],
    });

    const result = await runCodegenCheckpointResume({
      supabase: supabase as never,
      job: checkpointJob('quality_check'),
      workerId: 'worker-1',
      attempt: 2,
      mode: 'quality_check',
      heartbeat: vi.fn(async () => {}),
      assertNotCanceled: vi.fn(async () => {}),
      collectText,
    });

    expect(result?.qualityReport?.status).toBe('passed');
    expect(result?.code).toContain('Ship');
    expect(collectText).not.toHaveBeenCalled();
    expect(supabase.updatedPatches).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          output: expect.objectContaining({
            latestCheckpoint: expect.objectContaining({ stage: 'quality_check' }),
          }),
        }),
      ]),
    );
  });

  it('reruns repair from assembly and quality checkpoints without rerunning planning', async () => {
    const collectText = vi.fn(async () =>
      '<!doctype html><html><head><style>.hero{}</style></head><body>Ship faster</body></html>',
    );
    const report = {
      status: 'failed',
      framework: 'html',
      issues: [
        {
          code: 'missing_text',
          severity: 'error',
          message: 'Generated output is missing design text: Ship faster',
        },
      ],
      checkedAt: '2026-05-13T08:00:00.000Z',
      summary: {
        fileCount: 1,
        errorCount: 1,
        warningCount: 0,
        missingTextCount: 1,
        missingAssetCount: 0,
      },
    };
    const supabase = createCheckpointResumeSupabase({
      checkpoints: [
        checkpoint('planning', {
          designIR: {
            version: 1,
            target: { width: 320, height: 640, platformHint: 'mobile' },
            summary: {
              nodeCount: 1,
              textCount: 1,
              imageCount: 0,
              assetCount: 0,
              textContent: ['Ship faster'],
              semanticKinds: {},
            },
            assets: [],
            nodes: [
              {
                id: 'title',
                type: 'text',
                name: 'Title',
                bounds: { x: 0, y: 0, width: 100, height: 24 },
                text: { content: 'Ship faster' },
                appearance: {},
                assetRefs: [],
                semanticHints: [],
              },
            ],
          },
        }),
        checkpoint('assembly', {
          code: '<!doctype html><html><head><style>.hero{}</style></head><body></body></html>',
          files: [
            {
              path: 'design.html',
              language: 'html',
              role: 'entry',
              content: '<!doctype html><html><head><style>.hero{}</style></head><body></body></html>',
            },
          ],
        }),
        checkpoint('quality_check', { report }),
      ],
    });

    const result = await runCodegenCheckpointResume({
      supabase: supabase as never,
      job: checkpointJob('repair'),
      workerId: 'worker-1',
      attempt: 2,
      mode: 'repair',
      heartbeat: vi.fn(async () => {}),
      assertNotCanceled: vi.fn(async () => {}),
      collectText,
    });

    expect(collectText).toHaveBeenCalledTimes(1);
    expect(result?.code).toContain('Ship faster');
    expect(result?.repairAttempts).toHaveLength(1);
    expect(result?.qualityReport?.status).toBe('repaired');
    expect(supabase.updatedPatches).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          output: expect.objectContaining({
            latestCheckpoint: expect.objectContaining({ stage: 'repair' }),
          }),
        }),
      ]),
    );
  });

  it('worker reruns quality_check from checkpoint without reserving provider capacity', async () => {
    const collectText = vi.fn();
    const supabase = createWorkerResumeSupabase({
      mode: 'quality_check',
      checkpoints: [
        checkpoint('assembly', {
          code: '<!doctype html><html><head><style>.hero{}</style></head><body>Ship</body></html>',
          files: [
            {
              path: 'design.html',
              language: 'html',
              role: 'entry',
              content:
                '<!doctype html><html><head><style>.hero{}</style></head><body>Ship</body></html>',
            },
          ],
        }),
      ],
    });

    const result = await runCodegenWorkerOnce(supabase as never, {
      workerId: 'worker-1',
      heartbeatMs: 60_000,
      collectText,
    });

    expect(result?.status).toBe('succeeded');
    expect(supabase.rpc).not.toHaveBeenCalledWith(
      'reserve_codegen_provider_capacity',
      expect.anything(),
    );
    expect(supabase.generations).toHaveLength(1);
    expect(supabase.generations[0].final_code).toContain('Ship');
    expect(supabase.updatedPatches).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          output: expect.objectContaining({
            qualityStatus: 'passed',
          }),
        }),
      ]),
    );
  });

  it('worker reruns repair from assembly and quality report checkpoints without full generation', async () => {
    const collectText = vi.fn(async () =>
      '<!doctype html><html><head><style>.hero{}</style></head><body>Ship faster</body></html>',
    );
    vi.doMock('../server-codegen-provider', () => ({
      createChatCompletionText: collectText,
    }));
    try {
      const report = {
        status: 'failed',
        framework: 'html',
        issues: [
          {
            code: 'missing_text',
            severity: 'error',
            message: 'Generated output is missing design text: Ship faster',
          },
        ],
        checkedAt: '2026-05-13T08:00:00.000Z',
        summary: {
          fileCount: 1,
          errorCount: 1,
          warningCount: 0,
          missingTextCount: 1,
          missingAssetCount: 0,
        },
      };
      const supabase = createWorkerResumeSupabase({
        mode: 'repair',
        checkpoints: [
          checkpoint('planning', {
            designIR: {
              version: 1,
              target: { width: 320, height: 640, platformHint: 'mobile' },
              summary: {
                nodeCount: 1,
                textCount: 1,
                imageCount: 0,
                assetCount: 0,
                textContent: ['Ship faster'],
                semanticKinds: {},
              },
              assets: [],
              nodes: [
                {
                  id: 'title',
                  type: 'text',
                  name: 'Title',
                  bounds: { x: 0, y: 0, width: 100, height: 24 },
                  text: { content: 'Ship faster' },
                  appearance: {},
                  assetRefs: [],
                  semanticHints: [],
                },
              ],
            },
          }),
          checkpoint('assembly', {
            code: '<!doctype html><html><head><style>.hero{}</style></head><body></body></html>',
            files: [
              {
                path: 'design.html',
                language: 'html',
                role: 'entry',
                content:
                  '<!doctype html><html><head><style>.hero{}</style></head><body></body></html>',
              },
            ],
          }),
          checkpoint('quality_check', { report }),
        ],
      });

    const result = await runCodegenWorkerOnce(supabase as never, {
      workerId: 'worker-1',
      heartbeatMs: 60_000,
      collectText,
    });

      expect(result?.status).toBe('succeeded');
      expect(collectText).toHaveBeenCalledTimes(1);
      expect(supabase.generations[0].final_code).toContain('Ship faster');
      expect(supabase.generations[0].metadata).toEqual(
        expect.objectContaining({
          qualityStatus: 'repaired',
          repairAttempts: expect.arrayContaining([expect.objectContaining({ attempt: 1 })]),
        }),
      );
      expect(supabase.updatedPatches).toEqual(
        expect.arrayContaining([
          expect.objectContaining({
            output: expect.objectContaining({
              latestCheckpoint: expect.objectContaining({ stage: 'repair' }),
            }),
          }),
        ]),
      );
    } finally {
      vi.doUnmock('../server-codegen-provider');
    }
  });

  it('worker reruns only the requested failed chunk and continues through assembly', async () => {
    const collectText = vi
      .fn()
      .mockResolvedValueOnce(
        [
          'function Hero(){return `<section>Hero fixed</section>`}',
          '---CONTRACT---',
          '{"chunkId":"hero","componentName":"Hero","exportedProps":[],"slots":[],"cssClasses":[],"cssVariables":[],"imports":[]}',
        ].join('\n'),
      )
      .mockResolvedValueOnce(
        '<!doctype html><html><head><style>.hero{}</style></head><body><section>Hero fixed</section><footer>Footer ready</footer></body></html>',
      );
    vi.doMock('../server-codegen-provider', () => ({
      createChatCompletionText: collectText,
    }));
    try {
      const supabase = createWorkerResumeSupabase({
        mode: 'chunk',
        chunkId: 'hero',
        checkpoints: [
          checkpoint('planning', {
            plan: {
              chunks: [
                {
                  id: 'hero',
                  name: 'Hero',
                  nodeIds: ['hero-node'],
                  role: 'section',
                  suggestedComponentName: 'Hero',
                  dependencies: [],
                },
                {
                  id: 'footer',
                  name: 'Footer',
                  nodeIds: ['footer-node'],
                  role: 'footer',
                  suggestedComponentName: 'Footer',
                  dependencies: ['hero'],
                },
              ],
              sharedStyles: [],
              rootLayout: { direction: 'vertical', gap: 0, responsive: true },
            },
            designIR: {
              version: 1,
              target: { width: 320, height: 640, platformHint: 'desktop' },
              summary: {
                nodeCount: 2,
                textCount: 2,
                imageCount: 0,
                assetCount: 0,
                textContent: ['Hero fixed', 'Footer ready'],
                semanticKinds: {},
              },
              assets: [],
              nodes: [],
            },
          }),
          checkpoint('chunk', {
            chunkId: 'hero',
            name: 'Hero',
            status: 'failed',
            error: 'provider failed',
          }),
          checkpoint('chunk', {
            chunkId: 'footer',
            name: 'Footer',
            status: 'done',
            result: {
              chunkId: 'footer',
              code: 'function Footer(){return `<footer>Footer ready</footer>`}',
              contract: {
                chunkId: 'footer',
                componentName: 'Footer',
                exportedProps: [],
                slots: [],
                cssClasses: [],
                cssVariables: [],
                imports: [],
              },
            },
          }),
        ],
      });

    const result = await runCodegenWorkerOnce(supabase as never, {
      workerId: 'worker-1',
      heartbeatMs: 60_000,
      collectText,
    });

      expect(result?.status).toBe('succeeded');
      expect(collectText).toHaveBeenCalledTimes(2);
      expect(supabase.generations[0].final_code).toContain('Hero fixed');
      expect(supabase.generations[0].final_code).toContain('Footer ready');
      expect(supabase.updatedPatches).toEqual(
        expect.arrayContaining([
          expect.objectContaining({
            output: expect.objectContaining({
              latestCheckpoint: expect.objectContaining({
                stage: 'chunk',
                data: expect.objectContaining({
                  chunkId: 'hero',
                  status: 'done',
                }),
              }),
            }),
          }),
        ]),
      );
    } finally {
      vi.doUnmock('../server-codegen-provider');
    }
  });

  it('worker preserves failed chunk checkpoint when requested chunk rerun fails', async () => {
    const collectText = vi.fn(async () => {
      throw new Error('chunk model failed again');
    });
    vi.doMock('../server-codegen-provider', () => ({
      createChatCompletionText: collectText,
    }));
    try {
      const supabase = createWorkerResumeSupabase({
        mode: 'chunk',
        chunkId: 'hero',
        checkpoints: [
          checkpoint('planning', {
            plan: {
              chunks: [
                {
                  id: 'hero',
                  name: 'Hero',
                  nodeIds: ['hero-node'],
                  role: 'section',
                  suggestedComponentName: 'Hero',
                  dependencies: [],
                },
              ],
              sharedStyles: [],
              rootLayout: { direction: 'vertical', gap: 0, responsive: true },
            },
          }),
          checkpoint('chunk', {
            chunkId: 'hero',
            name: 'Hero',
            status: 'failed',
            error: 'provider failed',
          }),
        ],
      });

      const result = await runCodegenWorkerOnce(supabase as never, {
        workerId: 'worker-1',
        heartbeatMs: 60_000,
        collectText,
      });

      expect(result?.status).toBe('failed');
      expect(supabase.generations).toHaveLength(0);
      expect(supabase.updatedPatches).toEqual(
        expect.arrayContaining([
          expect.objectContaining({
            output: expect.objectContaining({
              latestCheckpoint: expect.objectContaining({
                stage: 'chunk',
                status: 'failed',
                data: expect.objectContaining({
                  chunkId: 'hero',
                  error: expect.stringContaining('chunk model failed again'),
                }),
              }),
            }),
          }),
        ]),
      );
    } finally {
      vi.doUnmock('../server-codegen-provider');
    }
  });
});

function checkpoint(stage: CodegenCheckpoint['stage'], data: unknown): CodegenCheckpoint {
  return {
    stage,
    status: stage === 'quality_check' ? 'failed' : 'succeeded',
    attempt: 1,
    data,
    createdAt: '2026-05-13T08:00:00.000Z',
  };
}

function checkpointJob(mode: 'quality_check' | 'repair') {
  return {
    id: 'job-1',
    file_id: 'file-1',
    owner_id: 'user-1',
    framework: 'html',
    provider: 'openai',
    model: 'gpt-5.4',
    output: { checkpoints: [] },
    input_snapshot: { resume: { mode } },
  };
}

function createCheckpointResumeSupabase(input: { checkpoints: unknown[] }) {
  const state = { output: { checkpoints: input.checkpoints } };
  const updatedPatches: Record<string, unknown>[] = [];
  const from = vi.fn((table: string) => {
    if (table === 'codegen_jobs') {
      return {
        select: vi.fn(() => ({
          eq: vi.fn(() => ({
            maybeSingle: vi.fn(async () => ({ data: state, error: null })),
          })),
        })),
        update: vi.fn((patch: Record<string, unknown>) => {
          updatedPatches.push(patch);
          if (patch.output && typeof patch.output === 'object') {
            state.output = patch.output as typeof state.output;
          }
          return {
            eq: vi.fn(() => ({
              eq: vi.fn(() => ({
                select: vi.fn(() => ({
                  single: vi.fn(async () => ({ data: checkpointJob('quality_check'), error: null })),
                })),
              })),
              select: vi.fn(() => ({
                single: vi.fn(async () => ({ data: checkpointJob('quality_check'), error: null })),
              })),
            })),
          };
        }),
      };
    }
    if (table === 'codegen_job_steps') {
      return {
        select: vi.fn(() => ({
          eq: vi.fn(() => ({
            eq: vi.fn(() => ({
              eq: vi.fn(() => ({
                maybeSingle: vi.fn(async () => ({ data: null, error: null })),
              })),
            })),
          })),
        })),
        insert: vi.fn(async () => ({ data: null, error: null })),
        update: vi.fn(() => ({ eq: vi.fn(async () => ({ data: null, error: null })) })),
      };
    }
    throw new Error(`Unexpected table ${table}`);
  });
  return { from, updatedPatches };
}

function workerJob(
  mode: 'quality_check' | 'repair' | 'chunk',
  output: Record<string, unknown>,
  chunkId?: string,
) {
  return {
    id: 'job-1',
    file_id: 'file-1',
    owner_id: 'user-1',
    generation_id: null,
    file_name: 'Design',
    page_name: 'Home',
    job_kind: 'full_generation',
    status: 'running',
    framework: 'html',
    page_id: 'page-1',
    target_kind: 'page',
    node_ids: [],
    target_hash: 'hash-1',
    document_revision: 1,
    provider: 'openai',
    model: 'gpt-5.4',
    priority: 0,
    progress: 0,
    attempts: 2,
    max_attempts: 2,
    locked_by: 'worker-1',
    locked_until: '2026-05-13T08:05:00.000Z',
    last_heartbeat_at: '2026-05-13T08:00:00.000Z',
    next_run_at: '2026-05-13T08:00:00.000Z',
    dead_lettered_at: null,
    last_error: null,
    failure_type: null,
    input_snapshot: {
      resume: { mode, chunkId: chunkId ?? null },
      nodes: [
        {
          id: 'hero-node',
          type: 'frame',
          name: 'Hero',
          x: 0,
          y: 0,
          width: 320,
          height: 240,
        },
        {
          id: 'footer-node',
          type: 'frame',
          name: 'Footer',
          x: 0,
          y: 240,
          width: 320,
          height: 80,
        },
      ],
    },
    output,
    error: null,
    created_at: '2026-05-13T08:00:00.000Z',
    updated_at: '2026-05-13T08:00:00.000Z',
    started_at: '2026-05-13T08:00:00.000Z',
    completed_at: null,
    canceled_at: null,
  };
}

function createWorkerResumeSupabase(input: {
  mode: 'quality_check' | 'repair' | 'chunk';
  chunkId?: string;
  checkpoints: unknown[];
}) {
  const state = {
    job: workerJob(input.mode, { checkpoints: input.checkpoints }, input.chunkId),
    output: { checkpoints: input.checkpoints },
    generationId: 0,
  };
  const updatedPatches: Record<string, unknown>[] = [];
  const generations: Record<string, unknown>[] = [];
  const taskNotifications: Record<string, unknown>[] = [];
  const events: Record<string, unknown>[] = [];
  const activityEvents: Record<string, unknown>[] = [];

  const makeJobResponse = () => ({ data: { ...state.job, output: state.output }, error: null });
  const updateJob = vi.fn((patch: Record<string, unknown>) => {
    updatedPatches.push(patch);
    state.job = { ...state.job, ...patch } as typeof state.job;
    if (patch.output && typeof patch.output === 'object') {
      state.output = patch.output as typeof state.output;
      state.job.output = state.output;
    }
    return {
      eq: vi.fn(() => ({
        eq: vi.fn(() => ({
          select: vi.fn(() => ({ single: vi.fn(async () => makeJobResponse()) })),
        })),
        select: vi.fn(() => ({ single: vi.fn(async () => makeJobResponse()) })),
      })),
    };
  });
  const stepRows = new Map<string, Record<string, unknown>>();
  const from = vi.fn((table: string) => {
    if (table === 'codegen_jobs') {
      return {
        select: vi.fn(() => ({
          eq: vi.fn(() => ({
            maybeSingle: vi.fn(async () => ({ data: { output: state.output }, error: null })),
            eq: vi.fn(() => ({
              single: vi.fn(async () => makeJobResponse()),
            })),
          })),
          in: vi.fn(() => ({ order: vi.fn(() => ({ range: vi.fn(async () => ({ data: [], error: null, count: 0 })) })) })),
        })),
        update: updateJob,
      };
    }
    if (table === 'codegen_job_steps') {
      return {
        select: vi.fn(() => ({
          eq: vi.fn((_jobColumn: string, jobId: string) => ({
            eq: vi.fn((_roleColumn: string, role: string) => ({
              eq: vi.fn((_attemptColumn: string, attempt: number) => ({
                maybeSingle: vi.fn(async () => ({
                  data: stepRows.get(`${jobId}:${role}:${attempt}`) ?? null,
                  error: null,
                })),
              })),
            })),
            order: vi.fn(async () => ({ data: Array.from(stepRows.values()), error: null })),
          })),
        })),
        insert: vi.fn(async (row: Record<string, unknown>) => {
          const id = `step-${stepRows.size + 1}`;
          const next = { id, ...row };
          stepRows.set(`${row.job_id}:${row.agent_role}:${row.attempt}`, next);
          return { data: null, error: null };
        }),
        update: vi.fn((patch: Record<string, unknown>) => ({
          eq: vi.fn(async (_column: string, id: string) => {
            for (const [key, row] of stepRows) {
              if (row.id === id) stepRows.set(key, { ...row, ...patch });
            }
            return { data: null, error: null };
          }),
        })),
      };
    }
    if (table === 'codegen_job_events') {
      return {
        insert: vi.fn(async (row: Record<string, unknown>) => {
          events.push(row);
          return { data: null, error: null };
        }),
        select: vi.fn(() => ({
          eq: vi.fn(() => ({ order: vi.fn(async () => ({ data: events, error: null })) })),
        })),
      };
    }
    if (table === 'code_generations') {
      return {
        insert: vi.fn((row: Record<string, unknown>) => {
          const generation = {
            id: `gen-${++state.generationId}`,
            ...row,
            created_at: '2026-05-13T08:00:00.000Z',
            completed_at: '2026-05-13T08:00:00.000Z',
          };
          generations.push(generation);
          return {
            select: vi.fn(() => ({
              single: vi.fn(async () => ({ data: generation, error: null })),
            })),
          };
        }),
      };
    }
    if (
      table === 'code_generation_chunks' ||
      table === 'code_generation_assets' ||
      table === 'code_generation_files'
    ) {
      return {
        insert: vi.fn(async () => ({ data: null, error: null })),
        select: vi.fn(() => ({
          eq: vi.fn(() => ({
            order: vi.fn(async () => ({ data: [], error: null })),
          })),
        })),
      };
    }
    if (table === 'task_notifications') {
      return {
        insert: vi.fn((row: Record<string, unknown>) => {
          const notification = { id: `notification-${taskNotifications.length + 1}`, ...row };
          taskNotifications.push(notification);
          return {
            select: vi.fn(() => ({
              single: vi.fn(async () => ({ data: notification, error: null })),
            })),
          };
        }),
      };
    }
    if (table === 'activity_events') {
      return {
        insert: vi.fn(async (row: Record<string, unknown>) => {
          activityEvents.push(row);
          return { data: null, error: null };
        }),
      };
    }
    if (table === 'codegen_provider_health') {
      return {
        select: vi.fn(() => ({
          eq: vi.fn(() => ({
            maybeSingle: vi.fn(async () => ({
              data: { circuit_open_until: null, consecutive_failures: 0 },
              error: null,
            })),
          })),
        })),
        update: vi.fn(() => ({ eq: vi.fn(async () => ({ data: null, error: null })) })),
        upsert: vi.fn(async () => ({ data: null, error: null })),
      };
    }
    if (table === 'codegen_provider_configs') {
      return {
        select: vi.fn(() => ({
          eq: vi.fn(() => ({
            maybeSingle: vi.fn(async () => ({
              data: {
                enabled: true,
                max_per_minute: 30,
                circuit_threshold: 3,
                circuit_open_ms: 60_000,
              },
              error: null,
            })),
          })),
        })),
      };
    }
    throw new Error(`Unexpected table ${table}`);
  });

  const rpc = vi.fn((name: string) => {
    if (name === 'requeue_stale_codegen_jobs') {
      return Promise.resolve({ data: 0, error: null });
    }
    if (name === 'claim_next_codegen_job') {
      return {
        select: vi.fn(() => ({
          maybeSingle: vi.fn(async () => ({ data: state.job, error: null })),
        })),
      };
    }
    if (name === 'heartbeat_codegen_job') {
      return Promise.resolve({ data: true, error: null });
    }
    if (name === 'reserve_codegen_provider_capacity') {
      return Promise.resolve({ data: true, error: null });
    }
    throw new Error(`Unexpected rpc ${name}`);
  });

  return {
    from,
    rpc,
    updatedPatches,
    generations,
    taskNotifications,
    events,
    activityEvents,
  };
}
