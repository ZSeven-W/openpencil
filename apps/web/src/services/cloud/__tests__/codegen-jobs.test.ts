import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  getCloudCodegenQueueAccess,
  getCloudCodegenWorkerOverview,
  getCloudCodegenWorkerStats,
  listCloudCodegenJobs,
  listCloudCodegenProviderConfigAudits,
  listCloudCodegenProviderConfigs,
  replayFailedCloudCodegenJobs,
  updateCloudCodegenJobPriority,
  updateCloudCodegenProviderConfig,
} from '../codegen-jobs';

const cloudFetchMock = vi.hoisted(() => vi.fn());

vi.mock('../cloud-fetch', () => ({
  cloudFetch: cloudFetchMock,
}));

describe('cloud codegen job service', () => {
  beforeEach(() => {
    cloudFetchMock.mockReset();
    cloudFetchMock.mockResolvedValue({ data: [] });
  });

  it('serializes task queue filters including dead-letter state', async () => {
    await listCloudCodegenJobs({
      fileId: 'file-1',
      active: true,
      deadLettered: false,
      limit: 25,
    });

    expect(cloudFetchMock).toHaveBeenCalledWith(
      '/api/cloud/codegen-jobs?fileId=file-1&active=true&deadLettered=false&limit=25',
    );
  });

  it('loads worker overview and replays failed jobs', async () => {
    cloudFetchMock
      .mockResolvedValueOnce({ data: { workers: [], metrics: {}, providers: [] } })
      .mockResolvedValueOnce({ data: [{ id: 'job-1' }] });

    await getCloudCodegenWorkerOverview();
    await replayFailedCloudCodegenJobs({ jobIds: ['job-1'], limit: 10 });

    expect(cloudFetchMock).toHaveBeenNthCalledWith(1, '/api/cloud/codegen-workers');
    expect(cloudFetchMock).toHaveBeenNthCalledWith(2, '/api/cloud/codegen-jobs/replay-failed', {
      method: 'POST',
      body: JSON.stringify({ jobIds: ['job-1'], limit: 10 }),
    });
  });

  it('serializes worker overview pagination and dead-letter replay filters', async () => {
    cloudFetchMock.mockResolvedValue({ data: [] });

    await getCloudCodegenWorkerOverview({
      workerLimit: 10,
      workerOffset: 20,
      providerLimit: 5,
      providerOffset: 10,
      failedLimit: 25,
      failedOffset: 50,
      provider: 'openai',
      failureType: 'rate_limit',
      deadLetteredFrom: '2026-05-13T00:00:00.000Z',
    });
    await replayFailedCloudCodegenJobs({
      provider: 'openai',
      failureType: 'rate_limit',
      deadLetteredFrom: '2026-05-13T00:00:00.000Z',
      limit: 20,
    });

    expect(cloudFetchMock).toHaveBeenNthCalledWith(
      1,
      '/api/cloud/codegen-workers?workerLimit=10&workerOffset=20&providerLimit=5&providerOffset=10&failedLimit=25&failedOffset=50&provider=openai&failureType=rate_limit&deadLetteredFrom=2026-05-13T00%3A00%3A00.000Z',
    );
    expect(cloudFetchMock).toHaveBeenNthCalledWith(2, '/api/cloud/codegen-jobs/replay-failed', {
      method: 'POST',
      body: JSON.stringify({
        provider: 'openai',
        failureType: 'rate_limit',
        deadLetteredFrom: '2026-05-13T00:00:00.000Z',
        limit: 20,
      }),
    });
  });

  it('updates pending job priority and provider config', async () => {
    cloudFetchMock.mockResolvedValue({ data: { id: 'job-1', priority: 30 } });

    await updateCloudCodegenJobPriority('job-1', 30);
    await listCloudCodegenProviderConfigs({ provider: 'openai', limit: 10, offset: 20 });
    await updateCloudCodegenProviderConfig('openai', {
      enabled: true,
      maxPerMinute: 12,
      circuitThreshold: 4,
      circuitOpenMs: 120_000,
      reason: 'load test',
    });

    expect(cloudFetchMock).toHaveBeenNthCalledWith(
      1,
      '/api/cloud/codegen-jobs/job-1/priority',
      {
        method: 'PATCH',
        body: JSON.stringify({ priority: 30 }),
      },
    );
    expect(cloudFetchMock).toHaveBeenNthCalledWith(
      2,
      '/api/cloud/codegen-provider-configs?provider=openai&limit=10&offset=20',
    );
    expect(cloudFetchMock).toHaveBeenNthCalledWith(
      3,
      '/api/cloud/codegen-provider-configs/openai',
      {
        method: 'PATCH',
        body: JSON.stringify({
          enabled: true,
          maxPerMinute: 12,
          circuitThreshold: 4,
          circuitOpenMs: 120_000,
          reason: 'load test',
        }),
      },
    );
  });

  it('loads queue access, worker stats, and provider config audit history', async () => {
    cloudFetchMock.mockResolvedValue({ data: [] });

    await getCloudCodegenQueueAccess();
    await getCloudCodegenWorkerStats({ days: 14 });
    await listCloudCodegenProviderConfigAudits({ provider: 'openai', limit: 10, offset: 20 });

    expect(cloudFetchMock).toHaveBeenNthCalledWith(1, '/api/cloud/codegen-queue-access');
    expect(cloudFetchMock).toHaveBeenNthCalledWith(2, '/api/cloud/codegen-worker-stats?days=14');
    expect(cloudFetchMock).toHaveBeenNthCalledWith(
      3,
      '/api/cloud/codegen-provider-config-audits?provider=openai&limit=10&offset=20',
    );
  });
});
