import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useCodegenJobStore } from '../codegen-job-store';
import * as jobsApi from '@/services/cloud/codegen-jobs';
import { CloudApiError } from '@/services/cloud/cloud-fetch';

vi.mock('@/services/cloud/codegen-jobs', () => ({
  batchCancelCloudCodegenJobs: vi.fn(),
  cancelCloudCodegenJob: vi.fn(),
  createCloudCodegenJob: vi.fn(),
  deleteCloudCodegenJob: vi.fn(),
  getCloudCodegenJob: vi.fn(),
  getCloudCodegenQueueAccess: vi.fn(),
  getCloudCodegenWorkerStats: vi.fn(),
  listCloudCodegenJobs: vi.fn(),
  listCloudCodegenJobsPage: vi.fn(),
  deleteTaskNotification: vi.fn(),
  listCloudCodegenProviderConfigAudits: vi.fn(),
  listCloudCodegenProviderConfigs: vi.fn(),
  getCloudCodegenWorkerOverview: vi.fn(),
  listTaskNotifications: vi.fn(),
  listTaskNotificationsPage: vi.fn(),
  markTaskNotificationRead: vi.fn(),
  replayFailedCloudCodegenJobs: vi.fn(),
  rerunCloudCodegenJobStep: vi.fn(),
  resumeCloudCodegenJob: vi.fn(),
  retryCloudCodegenJob: vi.fn(),
  updateCloudCodegenJobPriority: vi.fn(),
  updateCloudCodegenProviderConfig: vi.fn(),
}));

const job = {
  id: 'job-1',
  fileId: 'file-1',
  ownerId: 'user-1',
  generationId: null,
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
} as const;

function page<T>(data: T[], total = data.length, offset = 0) {
  return { data, page: { total, limit: 10, offset } };
}

beforeEach(() => {
  vi.clearAllMocks();
  useCodegenJobStore.getState().reset();
});

describe('codegen-job-store', () => {
  it('creates and tracks the latest background job', async () => {
    vi.mocked(jobsApi.createCloudCodegenJob).mockResolvedValue(job as any);

    const created = await useCodegenJobStore.getState().createJob({
      fileId: 'file-1',
      pageId: 'page-1',
      framework: 'react',
      targetKind: 'page',
      nodeIds: [],
      targetHash: 'hash-1',
      documentRevision: 1,
      provider: 'openai',
      model: 'gpt-5.4',
      nodes: [],
      qualityMode: 'production',
    });

    expect(created?.id).toBe('job-1');
    expect(jobsApi.createCloudCodegenJob).toHaveBeenCalledWith(
      expect.objectContaining({ qualityMode: 'production' }),
    );
    expect(useCodegenJobStore.getState().lastCreatedJobId).toBe('job-1');
    expect(useCodegenJobStore.getState().jobs).toEqual([job]);
  });

  it('refreshes jobs and notifications independently', async () => {
    const note = {
      id: 'note-1',
      ownerId: 'user-1',
      jobId: 'job-1',
      fileId: 'file-1',
      generationId: null,
      kind: 'codegen_job_succeeded',
      title: 'done',
      message: null,
      readAt: null,
      createdAt: '2026-05-13T08:00:00.000Z',
    };
    vi.mocked(jobsApi.listCloudCodegenJobsPage).mockResolvedValue(page([job as any]));
    vi.mocked(jobsApi.listTaskNotificationsPage).mockResolvedValue(page([note as any]));

    await useCodegenJobStore.getState().refreshJobs({ active: true, deadLettered: false });
    await useCodegenJobStore.getState().refreshNotifications();

    expect(jobsApi.listCloudCodegenJobsPage).toHaveBeenCalledWith({
      active: true,
      deadLettered: false,
      limit: 10,
    });
    expect(jobsApi.listTaskNotificationsPage).toHaveBeenCalledWith({ limit: 10 });
    expect(useCodegenJobStore.getState().jobs).toHaveLength(1);
    expect(useCodegenJobStore.getState().notifications).toHaveLength(1);
  });

  it('marks and deletes notifications in the tracked list', async () => {
    const note = {
      id: 'note-1',
      ownerId: 'user-1',
      jobId: 'job-1',
      fileId: 'file-1',
      generationId: null,
      kind: 'codegen_job_succeeded',
      title: 'done',
      message: null,
      readAt: null,
      createdAt: '2026-05-13T08:00:00.000Z',
    };
    useCodegenJobStore.setState({ notifications: [note as any] });
    vi.mocked(jobsApi.markTaskNotificationRead).mockResolvedValue({
      ...note,
      readAt: '2026-05-13T08:01:00.000Z',
    } as any);
    vi.mocked(jobsApi.deleteTaskNotification).mockResolvedValue(undefined);

    await useCodegenJobStore.getState().markNotificationRead('note-1');
    expect(useCodegenJobStore.getState().notifications[0]?.readAt).toBe(
      '2026-05-13T08:01:00.000Z',
    );

    await useCodegenJobStore.getState().deleteNotification('note-1');
    expect(jobsApi.deleteTaskNotification).toHaveBeenCalledWith('note-1');
    expect(useCodegenJobStore.getState().notifications).toHaveLength(0);
  });

  it('deletes a tracked job and removes matching notifications', async () => {
    const note = {
      id: 'note-1',
      ownerId: 'user-1',
      jobId: 'job-1',
      fileId: 'file-1',
      generationId: null,
      kind: 'codegen_job_succeeded',
      title: 'done',
      message: null,
      readAt: null,
      createdAt: '2026-05-13T08:00:00.000Z',
    };
    useCodegenJobStore.setState({
      jobs: [job as any, { ...job, id: 'job-2' } as any],
      jobPage: { total: 2, limit: 10, offset: 0 },
      notifications: [note as any],
    });
    vi.mocked(jobsApi.deleteCloudCodegenJob).mockResolvedValue(undefined);

    await useCodegenJobStore.getState().deleteJob('job-1');

    expect(jobsApi.deleteCloudCodegenJob).toHaveBeenCalledWith('job-1');
    expect(useCodegenJobStore.getState().jobs.map((item) => item.id)).toEqual(['job-2']);
    expect(useCodegenJobStore.getState().jobPage.total).toBe(1);
    expect(useCodegenJobStore.getState().notifications).toHaveLength(0);
  });

  it('reuses identical in-flight job list refreshes', async () => {
    let resolveJobs!: (result: ReturnType<typeof page<any>>) => void;
    vi.mocked(jobsApi.listCloudCodegenJobsPage).mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveJobs = resolve;
        }),
    );

    const first = useCodegenJobStore.getState().refreshJobs({ active: true });
    const second = useCodegenJobStore.getState().refreshJobs({ active: true });
    await Promise.resolve();

    expect(jobsApi.listCloudCodegenJobsPage).toHaveBeenCalledTimes(1);
    resolveJobs(page([job as any]));
    await Promise.all([first, second]);
    expect(useCodegenJobStore.getState().jobs).toEqual([job]);
  });

  it('keeps separate in-flight job refreshes for different filters', async () => {
    vi.mocked(jobsApi.listCloudCodegenJobsPage).mockResolvedValue(page([job as any]));

    await Promise.all([
      useCodegenJobStore.getState().refreshJobs({ active: true }),
      useCodegenJobStore.getState().refreshJobs({ deadLettered: true }),
    ]);

    expect(jobsApi.listCloudCodegenJobsPage).toHaveBeenCalledTimes(2);
  });

  it('passes force refresh through to job and worker reads', async () => {
    vi.mocked(jobsApi.listCloudCodegenJobsPage).mockResolvedValue(page([job as any]));
    vi.mocked(jobsApi.getCloudCodegenWorkerOverview).mockResolvedValue({
      workers: [],
      metrics: {},
      providers: [],
    } as any);

    await useCodegenJobStore.getState().refreshJobs({ active: true, force: true });
    await useCodegenJobStore
      .getState()
      .refreshWorkerOverview({ workerLimit: 5, summary: true, force: true });

    expect(jobsApi.listCloudCodegenJobsPage).toHaveBeenCalledWith(
      { active: true, limit: 10 },
      { force: true },
    );
    expect(jobsApi.getCloudCodegenWorkerOverview).toHaveBeenCalledWith(
      { workerLimit: 5, summary: true },
      { force: true },
    );
  });

  it('retries a failed job and updates the tracked job', async () => {
    useCodegenJobStore.setState({ jobs: [{ ...job, status: 'failed', error: 'boom' } as any] });
    vi.mocked(jobsApi.retryCloudCodegenJob).mockResolvedValue({
      ...job,
      status: 'pending',
      error: null,
    } as any);

    await useCodegenJobStore.getState().retryJob('job-1');

    expect(jobsApi.retryCloudCodegenJob).toHaveBeenCalledWith('job-1');
    expect(useCodegenJobStore.getState().jobs[0]).toMatchObject({
      id: 'job-1',
      status: 'pending',
      error: null,
    });
  });

  it('resumes a failed job and queues a specific step rerun', async () => {
    useCodegenJobStore.setState({ jobs: [{ ...job, status: 'failed', error: 'boom' } as any] });
    vi.mocked(jobsApi.resumeCloudCodegenJob).mockResolvedValue({
      ...job,
      status: 'pending',
      error: null,
    } as any);
    vi.mocked(jobsApi.rerunCloudCodegenJobStep).mockResolvedValue({
      ...job,
      status: 'pending',
      error: null,
      inputSnapshot: {
        resume: { mode: 'quality_check' },
      },
    } as any);

    await useCodegenJobStore.getState().resumeJob('job-1');
    await useCodegenJobStore
      .getState()
      .rerunJobStep('job-1', { stage: 'quality_check' });
    await useCodegenJobStore
      .getState()
      .rerunJobStep('job-1', { stage: 'chunk', chunkId: 'hero' });

    expect(jobsApi.resumeCloudCodegenJob).toHaveBeenCalledWith('job-1', {});
    expect(jobsApi.rerunCloudCodegenJobStep).toHaveBeenCalledWith('job-1', {
      stage: 'quality_check',
    });
    expect(jobsApi.rerunCloudCodegenJobStep).toHaveBeenCalledWith('job-1', {
      stage: 'chunk',
      chunkId: 'hero',
    });
    expect(useCodegenJobStore.getState().jobs[0]).toMatchObject({
      id: 'job-1',
      status: 'pending',
      error: null,
    });
  });

  it('loads a job detail and batch cancels selected jobs', async () => {
    vi.mocked(jobsApi.getCloudCodegenJob).mockResolvedValue({
      ...job,
      steps: [],
      events: [],
    } as any);
    vi.mocked(jobsApi.batchCancelCloudCodegenJobs).mockResolvedValue([
      { ...job, id: 'job-1', status: 'canceled' },
      { ...job, id: 'job-2', status: 'canceled' },
    ] as any);
    useCodegenJobStore.setState({ jobs: [job as any, { ...job, id: 'job-2' } as any] });

    const detail = await useCodegenJobStore.getState().loadJob('job-1');
    await useCodegenJobStore.getState().batchCancelJobs(['job-1', 'job-2']);

    expect(detail?.id).toBe('job-1');
    expect(jobsApi.getCloudCodegenJob).toHaveBeenCalledWith('job-1');
    expect(jobsApi.batchCancelCloudCodegenJobs).toHaveBeenCalledWith(['job-1', 'job-2']);
    expect(useCodegenJobStore.getState().jobs.map((item) => item.status)).toEqual([
      'canceled',
      'canceled',
    ]);
  });

  it('serves recently loaded job details from the store cache', async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-05-13T08:00:00.000Z'));
    vi.mocked(jobsApi.getCloudCodegenJob).mockResolvedValue({
      ...job,
      steps: [],
      events: [],
    } as any);

    const first = await useCodegenJobStore.getState().loadJob('job-1');
    vi.setSystemTime(new Date('2026-05-13T08:00:10.000Z'));
    const second = await useCodegenJobStore.getState().loadJob('job-1');

    expect(first?.id).toBe('job-1');
    expect(second?.id).toBe('job-1');
    expect(jobsApi.getCloudCodegenJob).toHaveBeenCalledTimes(1);
    vi.useRealTimers();
  });

  it('bypasses the job detail cache when force is requested', async () => {
    vi.mocked(jobsApi.getCloudCodegenJob).mockResolvedValue({
      ...job,
      steps: [],
      events: [],
    } as any);

    await useCodegenJobStore.getState().loadJob('job-1');
    await useCodegenJobStore.getState().loadJob('job-1', { force: true });

    expect(jobsApi.getCloudCodegenJob).toHaveBeenCalledTimes(2);
    expect(jobsApi.getCloudCodegenJob).toHaveBeenLastCalledWith('job-1', {
      force: true,
    });
  });

  it('ignores empty batch cancel requests', async () => {
    await useCodegenJobStore.getState().batchCancelJobs([]);

    expect(jobsApi.batchCancelCloudCodegenJobs).not.toHaveBeenCalled();
  });

  it('loads worker overview and replays failed jobs', async () => {
    const overview = {
      workers: [{ workerId: 'worker-1' }],
      metrics: { total: 1, failed: 1 },
      providers: [{ provider: 'openai', failureRate: 1 }],
    };
    vi.mocked(jobsApi.getCloudCodegenWorkerOverview).mockResolvedValue(overview as any);
    vi.mocked(jobsApi.replayFailedCloudCodegenJobs).mockResolvedValue([
      { ...job, status: 'pending', error: null },
    ] as any);

    await useCodegenJobStore.getState().refreshWorkerOverview();
    await useCodegenJobStore.getState().replayFailedJobs({ jobIds: ['job-1'], limit: 10 });

    expect(useCodegenJobStore.getState().workerOverview).toBe(overview);
    expect(jobsApi.replayFailedCloudCodegenJobs).toHaveBeenCalledWith({
      jobIds: ['job-1'],
      limit: 10,
    });
    expect(useCodegenJobStore.getState().jobs[0]).toMatchObject({
      id: 'job-1',
      status: 'pending',
      error: null,
    });
  });

  it('reuses identical in-flight worker overview refreshes', async () => {
    let resolveOverview!: (overview: any) => void;
    const overview = {
      workers: [{ workerId: 'worker-1' }],
      metrics: { total: 1, failed: 0 },
      providers: [],
    };
    vi.mocked(jobsApi.getCloudCodegenWorkerOverview).mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveOverview = resolve;
        }),
    );

    const first = useCodegenJobStore
      .getState()
      .refreshWorkerOverview({ workerLimit: 10, workerOffset: 0 });
    const second = useCodegenJobStore
      .getState()
      .refreshWorkerOverview({ workerLimit: 10, workerOffset: 0 });
    await Promise.resolve();

    expect(jobsApi.getCloudCodegenWorkerOverview).toHaveBeenCalledTimes(1);
    resolveOverview(overview);
    await Promise.all([first, second]);
    expect(useCodegenJobStore.getState().workerOverview).toBe(overview);
  });

  it('updates pending priority and provider configs through store actions', async () => {
    const providerConfig = {
      provider: 'openai',
      enabled: true,
      maxPerMinute: 12,
      circuitThreshold: 4,
      circuitOpenMs: 120_000,
      updatedBy: 'user-1',
      createdAt: '2026-05-13T08:00:00.000Z',
      updatedAt: '2026-05-13T08:01:00.000Z',
    };
    useCodegenJobStore.setState({ jobs: [job as any] });
    vi.mocked(jobsApi.updateCloudCodegenJobPriority).mockResolvedValue({
      ...job,
      priority: 40,
    } as any);
    vi.mocked(jobsApi.listCloudCodegenProviderConfigs).mockResolvedValue([
      providerConfig,
    ] as any);
    vi.mocked(jobsApi.updateCloudCodegenProviderConfig).mockResolvedValue({
      ...providerConfig,
      maxPerMinute: 20,
    } as any);

    await useCodegenJobStore.getState().updateJobPriority('job-1', 40);
    await useCodegenJobStore.getState().refreshProviderConfigs({ provider: 'openai' });
    await useCodegenJobStore.getState().saveProviderConfig('openai', {
      enabled: true,
      maxPerMinute: 20,
      circuitThreshold: 4,
      circuitOpenMs: 120_000,
      reason: 'capacity tuning',
    });

    expect(jobsApi.updateCloudCodegenJobPriority).toHaveBeenCalledWith('job-1', 40);
    expect(jobsApi.listCloudCodegenProviderConfigs).toHaveBeenCalledWith({ provider: 'openai' });
    expect(jobsApi.updateCloudCodegenProviderConfig).toHaveBeenCalledWith('openai', {
      enabled: true,
      maxPerMinute: 20,
      circuitThreshold: 4,
      circuitOpenMs: 120_000,
      reason: 'capacity tuning',
    });
    expect(useCodegenJobStore.getState().jobs[0]).toMatchObject({ priority: 40 });
    expect(useCodegenJobStore.getState().providerConfigs[0]).toMatchObject({
      provider: 'openai',
      maxPerMinute: 20,
    });
  });

  it('refreshes queue access, runtime stats, and provider config audits', async () => {
    const access = {
      role: 'admin',
      bootstrapMode: false,
      canViewWorkers: true,
      canReplayFailed: true,
      canManageProviders: true,
    };
    const stats = {
      buckets: [{ bucket: '2026-05-13', total: 4, failed: 1 }],
      providers: [{ provider: 'openai', total: 4, failed: 1 }],
      workers: { active: 1, stale: 0 },
    };
    const audit = {
      id: 'audit-1',
      provider: 'openai',
      actorId: 'user-1',
      actorEmail: 'alice@example.com',
      beforeConfig: { maxPerMinute: 30 },
      afterConfig: { maxPerMinute: 20 },
      reason: 'capacity tuning',
      createdAt: '2026-05-13T08:00:00.000Z',
    };
    vi.mocked(jobsApi.getCloudCodegenQueueAccess).mockResolvedValue(access as any);
    vi.mocked(jobsApi.getCloudCodegenWorkerStats).mockResolvedValue(stats as any);
    vi.mocked(jobsApi.listCloudCodegenProviderConfigAudits).mockResolvedValue({
      data: [audit],
      page: { total: 1, limit: 10, offset: 0 },
    } as any);

    await useCodegenJobStore.getState().refreshQueueAccess();
    await useCodegenJobStore.getState().refreshWorkerStats({ days: 14 });
    await useCodegenJobStore.getState().refreshProviderConfigAudits({
      provider: 'openai',
      limit: 10,
    });

    expect(useCodegenJobStore.getState().queueAccess).toBe(access);
    expect(useCodegenJobStore.getState().workerStats).toBe(stats);
    expect(useCodegenJobStore.getState().providerConfigAudits).toEqual([audit]);
  });

  it('shows a localized migration hint when background task tables are missing', async () => {
    vi.mocked(jobsApi.listCloudCodegenJobsPage).mockRejectedValue(
      new CloudApiError(
        503,
        'migration_required',
        'Background task tables are not initialized',
        { migration: 'supabase/migrations/202605130001_background_codegen_jobs.sql' },
      ),
    );

    await useCodegenJobStore.getState().refreshJobs();

    expect(useCodegenJobStore.getState().error).toContain(
      'supabase/migrations/202605130001_background_codegen_jobs.sql',
    );
  });
});
