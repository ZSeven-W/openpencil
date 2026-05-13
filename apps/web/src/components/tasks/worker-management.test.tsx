// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { WorkerManagement } from './worker-management';
import { useCodegenJobStore } from '@/stores/codegen-job-store';
import '@/i18n';

const routerMocks = vi.hoisted(() => ({
  navigate: vi.fn(),
}));

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => routerMocks.navigate,
}));

beforeEach(() => {
  vi.clearAllMocks();
  useCodegenJobStore.setState({
    jobs: [
      {
        id: 'job-1',
        fileId: 'file-1',
        ownerId: 'user-1',
        generationId: null,
        status: 'failed',
        framework: 'react',
        pageId: 'page-1',
        targetKind: 'page',
        nodeIds: [],
        targetHash: 'hash-1',
        documentRevision: 1,
        provider: 'openai',
        model: 'gpt-5.4',
        priority: 0,
        progress: 100,
        attempts: 2,
        maxAttempts: 2,
        lockedBy: null,
        lockedUntil: null,
        deadLetteredAt: '2026-05-13T08:00:00.000Z',
        failureType: 'rate_limit',
        inputSnapshot: {},
        output: {},
        error: 'rate limit',
        createdAt: '2026-05-13T08:00:00.000Z',
        updatedAt: '2026-05-13T08:00:00.000Z',
        startedAt: '2026-05-13T08:00:00.000Z',
        completedAt: '2026-05-13T08:00:10.000Z',
        canceledAt: null,
      } as any,
    ],
    workerOverview: {
      workers: [
        {
          workerId: 'worker-1',
          lastHeartbeatAt: '2026-05-13T08:02:00.000Z',
          active: true,
        },
      ],
      metrics: {
        total: 4,
        pending: 1,
        running: 1,
        succeeded: 1,
        failed: 1,
        canceled: 0,
        deadLettered: 1,
        averageDurationMs: 1250,
        failureRate: 0.25,
      },
      providers: [
        {
          provider: 'openai',
          total: 4,
          failed: 1,
          failureRate: 0.25,
          consecutiveFailures: 2,
          circuitOpenUntil: '2099-05-13T08:05:00.000Z',
        },
      ],
      failedJobs: [],
      pages: {
        workers: { total: 1, limit: 10, offset: 0 },
        providers: { total: 1, limit: 10, offset: 0 },
        failedJobs: { total: 1, limit: 10, offset: 0 },
      },
    } as any,
    providerConfigs: [
      {
        provider: 'openai',
        enabled: true,
        maxPerMinute: 30,
        circuitThreshold: 3,
        circuitOpenMs: 60000,
        updatedBy: 'user-1',
        createdAt: '2026-05-13T08:00:00.000Z',
        updatedAt: '2026-05-13T08:01:00.000Z',
      },
    ],
    providerConfigAudits: [
      {
        id: 'audit-1',
        provider: 'openai',
        actorId: 'user-1',
        actorEmail: 'alice@example.com',
        beforeConfig: { maxPerMinute: 30 },
        afterConfig: { maxPerMinute: 18 },
        reason: 'capacity tuning',
        createdAt: '2026-05-13T08:03:00.000Z',
      },
    ],
    providerConfigAuditPage: { total: 1, limit: 20, offset: 0 },
    queueAccess: {
      role: 'admin',
      bootstrapMode: false,
      canViewWorkers: true,
      canReplayFailed: true,
      canManageProviders: true,
    },
    workerStats: {
      buckets: [
        {
          bucket: '2026-05-13',
          total: 4,
          pending: 0,
          running: 1,
          succeeded: 2,
          failed: 1,
          canceled: 0,
          averageDurationMs: 1250,
          failureRate: 0.25,
        },
      ],
      providers: [
        {
          provider: 'openai',
          total: 4,
          succeeded: 2,
          failed: 1,
          averageDurationMs: 1250,
          failureRate: 0.25,
        },
      ],
      workers: { active: 1, stale: 0 },
    },
    loading: false,
    error: undefined,
    refreshWorkerOverview: vi.fn(),
    refreshWorkerStats: vi.fn(),
    refreshQueueAccess: vi.fn(),
    refreshProviderConfigs: vi.fn(),
    refreshProviderConfigAudits: vi.fn(),
    refreshJobs: vi.fn(),
    replayFailedJobs: vi.fn(),
    saveProviderConfig: vi.fn(),
  } as any);
});

afterEach(() => cleanup());

describe('WorkerManagement', () => {
  it('renders worker metrics, provider circuit state, and replays failures', async () => {
    render(<WorkerManagement />);

    expect(screen.getByText('Worker management')).toBeTruthy();
    expect(screen.getByText('worker-1')).toBeTruthy();
    expect(screen.getAllByText('openai').length).toBeGreaterThan(0);
    expect(screen.getByText('Circuit open')).toBeTruthy();
    expect(screen.getByText('rate limit')).toBeTruthy();
    expect(screen.getByText('Queue role: admin')).toBeTruthy();
    expect(screen.getByText('Runtime statistics')).toBeTruthy();
    expect(screen.getByText('Config audit history')).toBeTruthy();
    expect(screen.getByText('capacity tuning')).toBeTruthy();

    fireEvent.click(screen.getByText('Replay visible'));

    await waitFor(() => {
      expect(useCodegenJobStore.getState().replayFailedJobs).toHaveBeenCalledWith({
        jobIds: ['job-1'],
        limit: 50,
      });
    });
  });

  it('filters dead-letter replay and saves provider limits', async () => {
    render(<WorkerManagement />);

    fireEvent.change(screen.getByLabelText('Provider'), { target: { value: 'openai' } });
    fireEvent.click(screen.getByText('Replay filtered'));
    await waitFor(() => {
      expect(useCodegenJobStore.getState().replayFailedJobs).toHaveBeenCalledWith({
        provider: 'openai',
        failureType: undefined,
        deadLetteredFrom: undefined,
        deadLetteredTo: undefined,
        limit: 50,
      });
    });

    fireEvent.change(screen.getByLabelText('Max per minute'), { target: { value: '18' } });
    fireEvent.change(screen.getByLabelText('Change reason'), { target: { value: 'capacity tuning' } });
    fireEvent.click(screen.getByText('Save config'));
    await waitFor(() => {
      expect(useCodegenJobStore.getState().saveProviderConfig).toHaveBeenCalledWith('openai', {
        enabled: true,
        maxPerMinute: 18,
        circuitThreshold: 3,
        circuitOpenMs: 60000,
        reason: 'capacity tuning',
      });
    });
  });

  it('disables replay and provider editing for viewer access', () => {
    useCodegenJobStore.setState({
      queueAccess: {
        role: 'viewer',
        bootstrapMode: false,
        canViewWorkers: true,
        canReplayFailed: false,
        canManageProviders: false,
      },
    } as any);

    render(<WorkerManagement />);

    expect(screen.getByText('Queue role: viewer')).toBeTruthy();
    expect(screen.getByText('Replay visible').closest('button')?.disabled).toBe(true);
    expect(screen.getByText('Replay filtered').closest('button')?.disabled).toBe(true);
    expect(screen.getByText('Save config').closest('button')?.disabled).toBe(true);
  });
});
