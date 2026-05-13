// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { TaskDetail, getVisibleTaskSteps } from './task-detail';
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
  generationId: 'gen-1',
  status: 'succeeded',
  framework: 'react',
  pageId: 'page-1',
  targetKind: 'selection',
  nodeIds: ['node-1', 'node-2'],
  targetHash: 'hash-1',
  documentRevision: 9,
  provider: 'openai',
  model: 'gpt-5.4',
  priority: 0,
  progress: 100,
  attempts: 1,
  maxAttempts: 2,
  lockedBy: null,
  lockedUntil: null,
  inputSnapshot: {},
  output: { generationId: 'gen-1' },
  error: null,
  createdAt: '2026-05-13T08:00:00.000Z',
  updatedAt: '2026-05-13T08:00:03.000Z',
  startedAt: '2026-05-13T08:00:01.000Z',
  completedAt: '2026-05-13T08:00:03.000Z',
  canceledAt: null,
  steps: [
    {
      id: 'step-1',
      jobId: 'job-1',
      agentRole: 'planner',
      attempt: 1,
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
      type: 'succeeded',
      message: 'Code generation completed.',
      metadata: { generationId: 'gen-1' },
      createdAt: '2026-05-13T08:00:03.000Z',
    },
  ],
};

beforeEach(() => {
  vi.clearAllMocks();
  useCodegenJobStore.setState({
    jobs: [job as any],
    notifications: [],
    loading: false,
    error: undefined,
    loadJob: vi.fn(),
    cancelJob: vi.fn(),
    retryJob: vi.fn(),
  } as any);
});

afterEach(() => cleanup());

describe('TaskDetail', () => {
  it('loads and renders input, output, steps and events', async () => {
    render(<TaskDetail jobId="job-1" />);

    await waitFor(() => {
      expect(useCodegenJobStore.getState().loadJob).toHaveBeenCalledWith('job-1');
    });
    expect(screen.getByText('Input target')).toBeTruthy();
    expect(screen.getByText('Output generation history')).toBeTruthy();
    expect(screen.getByText('gen-1')).toBeTruthy();
    expect(screen.getByText('Planner')).toBeTruthy();
    expect(screen.getByText('Code generation completed.')).toBeTruthy();
  });

  it('renders one timeline card per agent role when stale duplicate steps exist', () => {
    useCodegenJobStore.setState({
      jobs: [
        {
          ...job,
          steps: [
            {
              ...(job.steps[0] as any),
              id: 'step-running',
              status: 'running',
              progress: 0,
              completedAt: null,
              updatedAt: '2026-05-13T08:00:01.000Z',
            },
            {
              ...(job.steps[0] as any),
              id: 'step-succeeded',
              status: 'succeeded',
              progress: 100,
              updatedAt: '2026-05-13T08:00:02.000Z',
            },
          ],
        } as any,
      ],
    } as any);

    const visibleSteps = getVisibleTaskSteps(useCodegenJobStore.getState().jobs[0]?.steps ?? []);
    expect(visibleSteps).toHaveLength(1);
    expect(visibleSteps[0]?.status).toBe('succeeded');
    expect(visibleSteps[0]?.progress).toBe(100);

    render(<TaskDetail jobId="job-1" />);

    const stepsPanel = screen.getByRole('heading', { name: 'Steps' }).closest('div');
    expect(stepsPanel).toBeTruthy();
    expect(within(stepsPanel as HTMLElement).getAllByText('Planner')).toHaveLength(1);
    expect(within(stepsPanel as HTMLElement).getByText('Completed')).toBeTruthy();
    expect(within(stepsPanel as HTMLElement).queryByText('Running')).toBeNull();
  });

  it('opens the editor with the generation id when output exists', () => {
    render(<TaskDetail jobId="job-1" />);

    fireEvent.click(screen.getAllByText('Open generation')[0]);

    expect(routerMocks.navigate).toHaveBeenCalledWith({
      to: '/editor/$fileId',
      params: { fileId: 'file-1' },
      search: { generationId: 'gen-1' },
    });
  });

  it('supports cancel and retry actions for active and failed jobs', async () => {
    useCodegenJobStore.setState({
      jobs: [{ ...job, status: 'running', generationId: null, output: {} } as any],
    } as any);
    const { rerender } = render(<TaskDetail jobId="job-1" />);

    fireEvent.click(screen.getByText('Cancel'));
    await waitFor(() => {
      expect(useCodegenJobStore.getState().cancelJob).toHaveBeenCalledWith('job-1');
    });

    useCodegenJobStore.setState({
      jobs: [{ ...job, status: 'failed', generationId: null, output: {}, error: 'boom' } as any],
    } as any);
    rerender(<TaskDetail jobId="job-1" />);

    fireEvent.click(screen.getByText('Retry'));
    await waitFor(() => {
      expect(useCodegenJobStore.getState().retryJob).toHaveBeenCalledWith('job-1');
    });
  });
});
