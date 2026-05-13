import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { H3Event } from 'h3';

const h3Mocks = vi.hoisted(() => ({
  body: {} as unknown,
  query: {} as Record<string, string>,
  params: {} as Record<string, string>,
  status: undefined as number | undefined,
}));

const cloudSupabaseMocks = vi.hoisted(() => ({
  getCloudSupabase: vi.fn(),
  getCloudServiceSupabase: vi.fn(),
  toApiError: vi.fn((statusCode: number, code: string, message: string, details?: unknown) => {
    const error = new Error(message) as Error & {
      statusCode: number;
      statusMessage: string;
      data?: unknown;
    };
    error.statusCode = statusCode;
    error.statusMessage = message;
    error.data = { code, details };
    return error;
  }),
}));

const jobMocks = vi.hoisted(() => ({
  createCodegenJob: vi.fn(),
  listCodegenJobs: vi.fn(),
  getCodegenJobDetail: vi.fn(),
  cancelCodegenJob: vi.fn(),
  retryCodegenJob: vi.fn(),
  batchCancelCodegenJobs: vi.fn(),
  updateCodegenJobPriority: vi.fn(),
  replayFailedCodegenJobs: vi.fn(),
  getCodegenWorkerOverview: vi.fn(),
  getCodegenQueueAccess: vi.fn(),
  getCodegenWorkerRuntimeStats: vi.fn(),
  listCodegenProviderConfigs: vi.fn(),
  listCodegenProviderConfigAudits: vi.fn(),
  updateCodegenProviderConfig: vi.fn(),
  listTaskNotifications: vi.fn(),
  markTaskNotificationRead: vi.fn(),
}));

vi.mock('h3', async () => {
  const actual = await vi.importActual<typeof import('h3')>('h3');
  return {
    ...actual,
    defineEventHandler: (handler: unknown) => handler,
    getQuery: vi.fn(() => h3Mocks.query),
    getRouterParam: vi.fn((_event: unknown, name: string) => h3Mocks.params[name]),
    readBody: vi.fn(async () => h3Mocks.body),
    setResponseStatus: vi.fn((_event: unknown, status: number) => {
      h3Mocks.status = status;
    }),
  };
});

vi.mock('../../../utils/cloud-supabase', () => cloudSupabaseMocks);
vi.mock('../../../utils/cloud-codegen-jobs', () => jobMocks);
vi.mock('../../../../utils/cloud-supabase', () => cloudSupabaseMocks);
vi.mock('../../../../utils/cloud-codegen-jobs', () => jobMocks);

const event = {} as H3Event;
const user = { id: 'user-1', email: 'alice@example.com' };
const supabase = { from: vi.fn() };
const serviceSupabase = { from: vi.fn(), rpc: vi.fn() };
const job = {
  id: '55555555-5555-4555-8555-555555555555',
  fileId: '33333333-3333-4333-8333-333333333333',
  ownerId: 'user-1',
  generationId: null,
  jobKind: 'full_generation',
  status: 'pending',
  framework: 'react',
  pageId: 'page-1',
  targetKind: 'page',
  nodeIds: [],
  targetHash: 'hash-1',
  documentRevision: 1,
  provider: 'openai',
  model: 'gpt-5.4',
  priority: 0,
  progress: 0,
  attempts: 0,
  maxAttempts: 2,
  lockedBy: null,
  lockedUntil: null,
  inputSnapshot: {},
  output: {},
  error: null,
  createdAt: '2026-05-13T08:00:00.000Z',
  updatedAt: '2026-05-13T08:00:00.000Z',
  startedAt: null,
  completedAt: null,
  canceledAt: null,
};

beforeEach(() => {
  vi.clearAllMocks();
  h3Mocks.body = {};
  h3Mocks.query = {};
  h3Mocks.params = {};
  h3Mocks.status = undefined;
  cloudSupabaseMocks.getCloudSupabase.mockResolvedValue({ supabase, user });
  cloudSupabaseMocks.getCloudServiceSupabase.mockReturnValue(serviceSupabase);
});

describe('cloud codegen job routes', () => {
  it('creates a background codegen job for the authenticated user', async () => {
    jobMocks.createCodegenJob.mockResolvedValue(job);
    h3Mocks.body = {
      fileId: job.fileId,
      pageId: 'page-1',
      framework: 'react',
      targetKind: 'page',
      nodeIds: [],
      targetHash: 'hash-1',
      documentRevision: 1,
      model: 'gpt-5.4',
      provider: 'openai',
      nodes: [],
      variables: {},
    };

    const handler = (await import('../codegen-jobs/index.post')).default;
    const result = await handler(event);

    expect(jobMocks.createCodegenJob).toHaveBeenCalledWith(
      { supabase, userId: user.id, userEmail: user.email },
      expect.objectContaining({
        fileId: job.fileId,
        framework: 'react',
      }),
    );
    expect(h3Mocks.status).toBe(201);
    expect(result.data).toBe(job);
  });

  it('accepts Anthropic provider for background codegen jobs', async () => {
    jobMocks.createCodegenJob.mockResolvedValue({
      ...job,
      provider: 'anthropic',
      model: 'claude-sonnet-4-6',
    });
    h3Mocks.body = {
      fileId: job.fileId,
      pageId: 'page-1',
      framework: 'react',
      targetKind: 'page',
      nodeIds: [],
      targetHash: 'hash-1',
      documentRevision: 1,
      model: 'claude-sonnet-4-6',
      provider: 'anthropic',
      nodes: [],
      variables: {},
    };

    const handler = (await import('../codegen-jobs/index.post')).default;
    const result = await handler(event);

    expect(jobMocks.createCodegenJob).toHaveBeenCalledWith(
      { supabase, userId: user.id, userEmail: user.email },
      expect.objectContaining({
        provider: 'anthropic',
        model: 'claude-sonnet-4-6',
      }),
    );
    expect(h3Mocks.status).toBe(201);
    expect(result.data).toMatchObject({ provider: 'anthropic' });
  });

  it('creates a patch codegen job when base generation and instruction are provided', async () => {
    jobMocks.createCodegenJob.mockResolvedValue({ ...job, jobKind: 'patch_generation' });
    h3Mocks.body = {
      jobKind: 'patch_generation',
      fileId: job.fileId,
      pageId: 'page-1',
      framework: 'react',
      targetKind: 'selection',
      nodeIds: ['node-1'],
      targetHash: 'hash-1',
      documentRevision: 1,
      model: 'gpt-5.4',
      provider: 'openai',
      nodes: [{ id: 'node-1', type: 'rectangle' }],
      variables: {},
      baseGenerationId: '66666666-6666-4666-8666-666666666666',
      patchInstruction: 'Fix selected spacing.',
    };

    const handler = (await import('../codegen-jobs/index.post')).default;
    const result = await handler(event);

    expect(jobMocks.createCodegenJob).toHaveBeenCalledWith(
      { supabase, userId: user.id, userEmail: user.email },
      expect.objectContaining({
        jobKind: 'patch_generation',
        baseGenerationId: '66666666-6666-4666-8666-666666666666',
        patchInstruction: 'Fix selected spacing.',
      }),
    );
    expect(h3Mocks.status).toBe(201);
    expect(result.data).toMatchObject({ jobKind: 'patch_generation' });
  });

  it('rejects patch jobs without base generation or patch instruction', async () => {
    h3Mocks.body = {
      jobKind: 'patch_generation',
      fileId: job.fileId,
      pageId: 'page-1',
      framework: 'react',
      targetKind: 'selection',
      nodeIds: ['node-1'],
      targetHash: 'hash-1',
      documentRevision: 1,
      model: 'gpt-5.4',
      provider: 'openai',
      nodes: [{ id: 'node-1', type: 'rectangle' }],
      variables: {},
    };

    const handler = (await import('../codegen-jobs/index.post')).default;

    await expect(handler(event)).rejects.toMatchObject({
      statusCode: 400,
      data: {
        code: 'validation_error',
        details: expect.objectContaining({
          fieldErrors: expect.objectContaining({
            baseGenerationId: expect.any(Array),
            patchInstruction: expect.any(Array),
          }),
        }),
      },
    });
    expect(jobMocks.createCodegenJob).not.toHaveBeenCalled();
  });

  it('lists active codegen jobs by file', async () => {
    jobMocks.listCodegenJobs.mockResolvedValue([job]);
    h3Mocks.query = { fileId: job.fileId, active: 'true', deadLettered: 'false', limit: '10' };

    const handler = (await import('../codegen-jobs/index.get')).default;
    const result = await handler(event);

    expect(jobMocks.listCodegenJobs).toHaveBeenCalledWith({
      supabase,
      userId: user.id,
      fileId: job.fileId,
      status: undefined,
      active: true,
      deadLettered: false,
      limit: 10,
    });
    expect(result.data).toEqual([job]);
  });

  it('loads, cancels, and retries a job by route id', async () => {
    jobMocks.getCodegenJobDetail.mockResolvedValue(job);
    jobMocks.cancelCodegenJob.mockResolvedValue({ ...job, status: 'canceled' });
    jobMocks.retryCodegenJob.mockResolvedValue({ ...job, status: 'pending', error: null });
    h3Mocks.params = { id: job.id };

    const detailHandler = (await import('../codegen-jobs/[id].get')).default;
    const cancelHandler = (await import('../codegen-jobs/[id]/cancel.post')).default;
    const retryHandler = (await import('../codegen-jobs/[id]/retry.post')).default;

    await expect(detailHandler(event)).resolves.toEqual({ data: job });
    await expect(cancelHandler(event)).resolves.toEqual({
      data: expect.objectContaining({ status: 'canceled' }),
    });
    await expect(retryHandler(event)).resolves.toEqual({
      data: expect.objectContaining({ status: 'pending', error: null }),
    });
    expect(jobMocks.getCodegenJobDetail).toHaveBeenCalledWith({
      supabase,
      userId: user.id,
      jobId: job.id,
    });
    expect(jobMocks.cancelCodegenJob).toHaveBeenCalledWith({
      supabase,
      userId: user.id,
      jobId: job.id,
    });
    expect(jobMocks.retryCodegenJob).toHaveBeenCalledWith({
      supabase,
      userId: user.id,
      jobId: job.id,
    });
  });

  it('batch cancels selected jobs', async () => {
    jobMocks.batchCancelCodegenJobs.mockResolvedValue([
      { ...job, id: 'job-1', status: 'canceled' },
      { ...job, id: 'job-2', status: 'canceled' },
    ]);
    h3Mocks.body = { jobIds: ['job-1', 'job-2'] };

    const handler = (await import('../codegen-jobs/batch-cancel.post')).default;
    const result = await handler(event);

    expect(jobMocks.batchCancelCodegenJobs).toHaveBeenCalledWith({
      supabase,
      userId: user.id,
      jobIds: ['job-1', 'job-2'],
    });
    expect(result.data).toHaveLength(2);
    expect(result.data[0]).toMatchObject({ id: 'job-1', status: 'canceled' });
  });

  it('updates priority only through the pending job priority route', async () => {
    jobMocks.updateCodegenJobPriority.mockResolvedValue({ ...job, priority: 42 });
    h3Mocks.params = { id: job.id };
    h3Mocks.body = { priority: 42 };

    const handler = (await import('../codegen-jobs/[id]/priority.patch')).default;
    const result = await handler(event);

    expect(jobMocks.updateCodegenJobPriority).toHaveBeenCalledWith({
      supabase,
      userId: user.id,
      jobId: job.id,
      priority: 42,
    });
    expect(result.data).toMatchObject({ id: job.id, priority: 42 });
  });

  it('replays failed jobs in bulk with dead-letter filters and audit actor', async () => {
    jobMocks.replayFailedCodegenJobs.mockResolvedValue([{ ...job, status: 'pending' }]);
    h3Mocks.body = {
      provider: 'openai',
      failureType: 'rate_limit',
      deadLetteredFrom: '2026-05-13T00:00:00.000Z',
      deadLetteredTo: '2026-05-14T00:00:00.000Z',
      limit: 20,
    };

    const handler = (await import('../codegen-jobs/replay-failed.post')).default;
    const result = await handler(event);

    expect(jobMocks.replayFailedCodegenJobs).toHaveBeenCalledWith({
      supabase,
      adminSupabase: serviceSupabase,
      userId: user.id,
      userEmail: user.email,
      jobIds: undefined,
      provider: 'openai',
      failureType: 'rate_limit',
      deadLetteredFrom: '2026-05-13T00:00:00.000Z',
      deadLetteredTo: '2026-05-14T00:00:00.000Z',
      limit: 20,
    });
    expect(result.data[0]).toMatchObject({ id: job.id, status: 'pending' });
  });

  it('loads worker overview for the authenticated cloud user', async () => {
    const overview = {
      workers: [{ workerId: 'worker-1', lastHeartbeatAt: '2026-05-13T08:00:00.000Z' }],
      metrics: { total: 3, pending: 1, running: 1, succeeded: 1, failed: 0, canceled: 0 },
      providers: [{ provider: 'openai', failureRate: 0, circuitOpenUntil: null }],
    };
    jobMocks.getCodegenWorkerOverview.mockResolvedValue(overview);
    h3Mocks.query = {
      workerLimit: '10',
      workerOffset: '20',
      providerLimit: '5',
      providerOffset: '10',
      failedLimit: '25',
      failedOffset: '50',
      provider: 'openai',
      failureType: 'rate_limit',
      deadLetteredFrom: '2026-05-13T00:00:00.000Z',
    };

    const handler = (await import('../codegen-workers/index.get')).default;
    const result = await handler(event);

    expect(jobMocks.getCodegenWorkerOverview).toHaveBeenCalledWith({
      supabase,
      adminSupabase: serviceSupabase,
      userId: user.id,
      workerLimit: 10,
      workerOffset: 20,
      providerLimit: 5,
      providerOffset: 10,
      failedLimit: 25,
      failedOffset: 50,
      provider: 'openai',
      failureType: 'rate_limit',
      deadLetteredFrom: '2026-05-13T00:00:00.000Z',
      deadLetteredTo: undefined,
    });
    expect(result.data).toBe(overview);
  });

  it('lists and updates dynamic provider rate limit configuration', async () => {
    const config = {
      provider: 'openai',
      enabled: true,
      maxPerMinute: 18,
      circuitThreshold: 4,
      circuitOpenMs: 90_000,
      updatedBy: user.id,
      createdAt: '2026-05-13T08:00:00.000Z',
      updatedAt: '2026-05-13T08:01:00.000Z',
    };
    jobMocks.listCodegenProviderConfigs.mockResolvedValue([config]);
    jobMocks.updateCodegenProviderConfig.mockResolvedValue(config);
    h3Mocks.params = { provider: 'openai' };
    h3Mocks.body = {
      enabled: true,
      maxPerMinute: 18,
      circuitThreshold: 4,
      circuitOpenMs: 90_000,
      reason: 'load test',
    };

    const listHandler = (await import('../codegen-provider-configs/index.get')).default;
    const updateHandler = (await import('../codegen-provider-configs/[provider].patch')).default;

    await expect(listHandler(event)).resolves.toEqual({ data: [config] });
    await expect(updateHandler(event)).resolves.toEqual({ data: config });
    expect(jobMocks.listCodegenProviderConfigs).toHaveBeenCalledWith({
      supabase: serviceSupabase,
      provider: undefined,
      limit: 50,
      offset: 0,
    });
    expect(jobMocks.updateCodegenProviderConfig).toHaveBeenCalledWith({
      supabase: serviceSupabase,
      actorId: user.id,
      actorEmail: user.email,
      provider: 'openai',
      enabled: true,
      maxPerMinute: 18,
      circuitThreshold: 4,
      circuitOpenMs: 90_000,
      reason: 'load test',
    });
  });

  it('loads queue access, runtime stats, and provider config audit history', async () => {
    const access = {
      role: 'admin',
      bootstrapMode: false,
      canViewWorkers: true,
      canReplayFailed: true,
      canManageProviders: true,
    };
    const stats = {
      buckets: [{ bucket: '2026-05-13', total: 4, succeeded: 3, failed: 1 }],
      providers: [{ provider: 'openai', total: 4, failed: 1, failureRate: 0.25 }],
      workers: { active: 1, stale: 0 },
    };
    const audit = {
      id: 'audit-1',
      provider: 'openai',
      actorId: user.id,
      actorEmail: user.email,
      beforeConfig: { maxPerMinute: 30 },
      afterConfig: { maxPerMinute: 18 },
      reason: 'load test',
      createdAt: '2026-05-13T08:02:00.000Z',
    };
    jobMocks.getCodegenQueueAccess.mockResolvedValue(access);
    jobMocks.getCodegenWorkerRuntimeStats.mockResolvedValue(stats);
    jobMocks.listCodegenProviderConfigAudits.mockResolvedValue({
      data: [audit],
      page: { total: 1, limit: 10, offset: 0 },
    });
    h3Mocks.query = { provider: 'openai', limit: '10', offset: '0', days: '7' };

    const accessHandler = (await import('../codegen-queue-access/index.get')).default;
    const statsHandler = (await import('../codegen-worker-stats/index.get')).default;
    const auditsHandler = (await import('../codegen-provider-config-audits/index.get')).default;

    await expect(accessHandler(event)).resolves.toEqual({ data: access });
    await expect(statsHandler(event)).resolves.toEqual({ data: stats });
    await expect(auditsHandler(event)).resolves.toEqual({
      data: [audit],
      page: { total: 1, limit: 10, offset: 0 },
    });
    expect(jobMocks.getCodegenQueueAccess).toHaveBeenCalledWith({
      supabase: serviceSupabase,
      userId: user.id,
      userEmail: user.email,
    });
    expect(jobMocks.getCodegenWorkerRuntimeStats).toHaveBeenCalledWith({
      supabase,
      adminSupabase: serviceSupabase,
      userId: user.id,
      days: 7,
    });
    expect(jobMocks.listCodegenProviderConfigAudits).toHaveBeenCalledWith({
      supabase: serviceSupabase,
      provider: 'openai',
      limit: 10,
      offset: 0,
    });
  });

  it('lists and marks task notifications read', async () => {
    const notification = {
      id: 'note-1',
      ownerId: user.id,
      jobId: job.id,
      fileId: job.fileId,
      generationId: null,
      kind: 'codegen_job_succeeded',
      title: 'Code generation completed',
      message: 'react',
      readAt: null,
      createdAt: '2026-05-13T08:00:00.000Z',
    };
    jobMocks.listTaskNotifications.mockResolvedValue([notification]);
    jobMocks.markTaskNotificationRead.mockResolvedValue({
      ...notification,
      readAt: '2026-05-13T08:01:00.000Z',
    });
    h3Mocks.params = { id: notification.id };

    const listHandler = (await import('../task-notifications/index.get')).default;
    const readHandler = (await import('../task-notifications/[id]/read.post')).default;

    await expect(listHandler(event)).resolves.toEqual({ data: [notification] });
    await expect(readHandler(event)).resolves.toEqual({
      data: expect.objectContaining({ readAt: '2026-05-13T08:01:00.000Z' }),
    });
  });
});
