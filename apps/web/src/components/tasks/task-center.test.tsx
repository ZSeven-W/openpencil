// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { TaskCenter } from './task-center';
import { useCodegenJobStore } from '@/stores/codegen-job-store';
import '@/i18n';

const routerMocks = vi.hoisted(() => ({
  navigate: vi.fn(),
}));

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => routerMocks.navigate,
}));

const job = {
  id: 'job-1',
  fileId: 'file-1',
  ownerId: 'user-1',
  generationId: null,
  status: 'running',
  framework: 'react',
  pageId: 'page-1',
  targetKind: 'page',
  nodeIds: [],
  targetHash: 'hash-1',
  documentRevision: 1,
  provider: 'openai',
  model: 'gpt-5.4',
  priority: 0,
  progress: 35,
  attempts: 1,
  maxAttempts: 2,
  lockedBy: null,
  lockedUntil: null,
  inputSnapshot: {},
  output: {},
  error: null,
  createdAt: '2026-05-13T08:00:00.000Z',
  updatedAt: '2026-05-13T08:00:00.000Z',
  startedAt: '2026-05-13T08:00:01.000Z',
  completedAt: null,
  canceledAt: null,
  steps: [
    {
      id: 'step-1',
      jobId: 'job-1',
      agentRole: 'planner',
      status: 'succeeded',
      progress: 100,
      input: {},
      output: {},
      error: null,
      startedAt: '2026-05-13T08:00:01.000Z',
      completedAt: '2026-05-13T08:00:02.000Z',
      createdAt: '2026-05-13T08:00:01.000Z',
      updatedAt: '2026-05-13T08:00:02.000Z',
    },
  ],
  events: [
    {
      id: 'event-1',
      jobId: 'job-1',
      type: 'started',
      message: 'Background worker started the job.',
      metadata: {},
      createdAt: '2026-05-13T08:00:01.000Z',
    },
  ],
};

beforeEach(() => {
  vi.clearAllMocks();
  useCodegenJobStore.setState({
    jobs: [job as any],
    notifications: [
      {
        id: 'note-1',
        ownerId: 'user-1',
        jobId: 'job-1',
        fileId: 'file-1',
        generationId: null,
        kind: 'codegen_job_succeeded',
        title: 'Code generation completed',
        message: 'react',
        readAt: null,
        createdAt: '2026-05-13T08:00:00.000Z',
      } as any,
    ],
    workerOverview: {
      workers: [
        {
          workerId: 'worker-1',
          lastHeartbeatAt: '2026-05-13T08:00:00.000Z',
          active: true,
        },
      ],
      metrics: {
        total: 1,
        pending: 0,
        running: 1,
        succeeded: 0,
        failed: 0,
        canceled: 0,
        deadLettered: 0,
        averageDurationMs: 0,
        failureRate: 0,
      },
      providers: [],
      failedJobs: [],
      pages: {
        workers: { total: 1, limit: 5, offset: 0 },
        providers: { total: 0, limit: 1, offset: 0 },
        failedJobs: { total: 0, limit: 1, offset: 0 },
      },
    } as any,
    loading: false,
    error: undefined,
    refreshJobs: vi.fn(),
    refreshNotifications: vi.fn(),
    refreshWorkerOverview: vi.fn(),
    cancelJob: vi.fn(),
    batchCancelJobs: vi.fn(),
    retryJob: vi.fn(),
    updateJobPriority: vi.fn(),
    markNotificationRead: vi.fn(),
  } as any);
});

afterEach(() => {
  cleanup();
});

describe('TaskCenter', () => {
  it('renders global background tasks and notifications', async () => {
    render(<TaskCenter />);

    expect(screen.getByText('Tasks')).toBeTruthy();
    expect(screen.getByText('react code generation')).toBeTruthy();
    expect(screen.getByText('Code generation completed')).toBeTruthy();
    await waitFor(() => {
      expect(useCodegenJobStore.getState().refreshWorkerOverview).toHaveBeenCalledWith({
        workerLimit: 5,
        providerLimit: 1,
        failedLimit: 1,
      });
    });

    fireEvent.click(screen.getByText('Open file'));
    expect(routerMocks.navigate).toHaveBeenCalledWith({
      to: '/editor/$fileId',
      params: { fileId: 'file-1' },
    });

    fireEvent.click(screen.getByText('Details'));
    expect(routerMocks.navigate).toHaveBeenCalledWith({
      to: '/tasks/$jobId',
      params: { jobId: 'job-1' },
    });

    fireEvent.click(screen.getByText('Cancel'));
    await waitFor(() => {
      expect(useCodegenJobStore.getState().cancelJob).toHaveBeenCalledWith('job-1');
    });
  });

  it('filters file-level jobs and retries failed jobs', async () => {
    const refreshJobs = vi.fn();
    useCodegenJobStore.setState({
      jobs: [{ ...job, status: 'failed', error: 'boom' } as any],
      refreshJobs,
      retryJob: vi.fn(),
    } as any);

    render(<TaskCenter fileId="file-1" />);

    await waitFor(() => {
      expect(refreshJobs).toHaveBeenCalledWith({ fileId: 'file-1' });
    });

    fireEvent.click(screen.getByText('Retry'));
    await waitFor(() => {
      expect(useCodegenJobStore.getState().retryJob).toHaveBeenCalledWith('job-1');
    });
  });

  it('batch cancels selected running jobs', async () => {
    const batchCancelJobs = vi.fn();
    useCodegenJobStore.setState({
      jobs: [job as any, { ...job, id: 'job-2', status: 'pending' } as any],
      batchCancelJobs,
    } as any);

    render(<TaskCenter />);

    fireEvent.click(screen.getByRole('checkbox', { name: 'Select task job-1' }));
    fireEvent.click(screen.getByRole('checkbox', { name: 'Select task job-2' }));
    fireEvent.click(screen.getByText('Cancel selected'));

    await waitFor(() => {
      expect(batchCancelJobs).toHaveBeenCalledWith(['job-1', 'job-2']);
    });
  });

  it('updates priority for pending jobs from the task list', async () => {
    const updateJobPriority = vi.fn();
    useCodegenJobStore.setState({
      jobs: [{ ...job, status: 'pending', priority: 5 } as any],
      updateJobPriority,
    } as any);

    render(<TaskCenter />);

    fireEvent.change(screen.getByLabelText('Priority'), { target: { value: '42' } });
    fireEvent.click(screen.getByText('Save priority'));

    await waitFor(() => {
      expect(updateJobPriority).toHaveBeenCalledWith('job-1', 42);
    });
  });

  it('warns when pending jobs have no active workers and links to worker management', () => {
    useCodegenJobStore.setState({
      jobs: [{ ...job, status: 'pending', progress: 0 } as any],
      workerOverview: {
        workers: [
          {
            workerId: 'stale-worker',
            lastHeartbeatAt: '2026-05-13T07:00:00.000Z',
            active: false,
          },
        ],
        metrics: {
          total: 1,
          pending: 1,
          running: 0,
          succeeded: 0,
          failed: 0,
          canceled: 0,
          deadLettered: 0,
          averageDurationMs: 0,
          failureRate: 0,
        },
        providers: [],
        failedJobs: [],
        pages: {
          workers: { total: 1, limit: 5, offset: 0 },
          providers: { total: 0, limit: 1, offset: 0 },
          failedJobs: { total: 0, limit: 1, offset: 0 },
        },
      } as any,
    } as any);

    render(<TaskCenter />);

    expect(screen.getByText('No active worker')).toBeTruthy();
    expect(
      screen.getByText('Pending tasks will not run until a codegen worker is started.'),
    ).toBeTruthy();

    fireEvent.click(screen.getByText('Open worker management'));
    expect(routerMocks.navigate).toHaveBeenCalledWith({ to: '/tasks/workers' });
  });
});
