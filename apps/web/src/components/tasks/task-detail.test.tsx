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
  Link: ({ children, to, ...props }: any) => (
    <a href={typeof to === 'string' ? to : '#'} {...props}>
      {children}
    </a>
  ),
  useNavigate: () => routerMocks.navigate,
}));

const job = {
  id: 'job-1',
  fileId: 'file-1',
  ownerId: 'user-1',
  generationId: 'gen-1',
  fileName: 'Design Library',
  pageName: 'Checkout Page',
  pipelineMode: 'direct_generation',
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
  inputSnapshot: { fileName: 'Design Library', pageName: 'Checkout Page' },
  output: { generationId: 'gen-1', pipelineMode: 'direct_generation' },
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
      input: { target: 'selection' },
      output: { files: 3 },
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
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });
  useCodegenJobStore.setState({
    jobs: [job as any],
    notifications: [],
    loading: false,
    error: undefined,
    loadJob: vi.fn(),
    cancelJob: vi.fn(),
    deleteJob: vi.fn(),
    retryJob: vi.fn(),
    resumeJob: vi.fn(),
    rerunJobStep: vi.fn(),
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
    expect(screen.getByText('Execution summary')).toBeTruthy();
    expect(screen.getAllByText('Direct Generation').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Design Library').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Checkout Page').length).toBeGreaterThan(0);
    expect(screen.getByText('Output generation history')).toBeTruthy();
    expect(screen.getByText('gen-1')).toBeTruthy();
    expect(screen.getByText('Planner')).toBeTruthy();
    expect(screen.getByText('{"target":"selection"}')).toBeTruthy();
    expect(screen.getByText('{"files":3}')).toBeTruthy();
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
    expect(within(stepsPanel as HTMLElement).getAllByText('Completed').length).toBeGreaterThan(0);
    expect(within(stepsPanel as HTMLElement).queryByText('Running')).toBeNull();
  });

  it('renders stale running steps as completed when the job already succeeded', () => {
    useCodegenJobStore.setState({
      jobs: [
        {
          ...job,
          status: 'succeeded',
          completedAt: '2026-05-13T08:00:05.000Z',
          steps: [
            {
              ...(job.steps[0] as any),
              id: 'step-planner-stale',
              agentRole: 'planner',
              status: 'running',
              progress: 0,
              completedAt: null,
              updatedAt: '2026-05-13T08:00:01.000Z',
            },
            {
              ...(job.steps[0] as any),
              id: 'step-bundler',
              agentRole: 'bundler_persist',
              status: 'succeeded',
              progress: 100,
              completedAt: '2026-05-13T08:00:05.000Z',
              updatedAt: '2026-05-13T08:00:05.000Z',
            },
          ],
        } as any,
      ],
    } as any);

    render(<TaskDetail jobId="job-1" />);

    const stepsPanel = screen.getByRole('heading', { name: 'Steps' }).closest('div');
    expect(stepsPanel).toBeTruthy();
    expect(within(stepsPanel as HTMLElement).getByText('Planner')).toBeTruthy();
    expect(within(stepsPanel as HTMLElement).getByText('Bundle and persist')).toBeTruthy();
    expect(within(stepsPanel as HTMLElement).getAllByText('Completed').length).toBeGreaterThan(1);
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

  it('supports cancel, resume, retry, and step rerun actions', async () => {
    useCodegenJobStore.setState({
      jobs: [{ ...job, status: 'running', generationId: null, output: {} } as any],
    } as any);
    const { rerender } = render(<TaskDetail jobId="job-1" />);

    fireEvent.click(screen.getByText('Cancel'));
    await waitFor(() => {
      expect(useCodegenJobStore.getState().cancelJob).toHaveBeenCalledWith('job-1');
    });

    useCodegenJobStore.setState({
      jobs: [
        {
          ...job,
          status: 'failed',
          generationId: null,
          output: {},
          error: 'boom',
          steps: [
            {
              ...(job.steps[0] as any),
              id: 'step-quality',
              agentRole: 'quality_check',
              status: 'failed',
              progress: 78,
              error: 'missing text',
            },
          ],
        } as any,
      ],
    } as any);
    rerender(<TaskDetail jobId="job-1" />);

    fireEvent.click(screen.getByText('Resume failed stage'));
    await waitFor(() => {
      expect(useCodegenJobStore.getState().resumeJob).toHaveBeenCalledWith('job-1');
    });
    fireEvent.click(screen.getByText('Rerun step'));
    await waitFor(() => {
      expect(useCodegenJobStore.getState().rerunJobStep).toHaveBeenCalledWith('job-1', {
        stage: 'quality_check',
      });
    });
    fireEvent.click(screen.getByText('Retry'));
    await waitFor(() => {
      expect(useCodegenJobStore.getState().retryJob).toHaveBeenCalledWith('job-1');
    });
  });

  it('reruns a failed page_codegen chunk with its chunk id', async () => {
    useCodegenJobStore.setState({
      jobs: [
        {
          ...job,
          status: 'failed',
          generationId: null,
          output: {},
          steps: [
            {
              ...(job.steps[0] as any),
              id: 'step-page',
              agentRole: 'page_codegen',
              status: 'failed',
              progress: 100,
              input: { chunkId: 'hero' },
              output: {},
              error: 'chunk failed',
            },
          ],
        } as any,
      ],
    } as any);

    render(<TaskDetail jobId="job-1" />);

    expect(
      screen.getByText('This will rerun chunk hero and reuse the other completed chunks.'),
    ).toBeTruthy();
    fireEvent.click(screen.getByText('Rerun chunk'));
    await waitFor(() => {
      expect(useCodegenJobStore.getState().rerunJobStep).toHaveBeenCalledWith('job-1', {
        stage: 'chunk',
        chunkId: 'hero',
      });
    });
  });

  it('renders checkpoint stage artifacts from job output', () => {
    useCodegenJobStore.setState({
      jobs: [
        {
          ...job,
          output: {
            generationId: 'gen-1',
            checkpoints: [
              {
                stage: 'planning',
                status: 'succeeded',
                attempt: 1,
                createdAt: '2026-05-13T08:00:01.000Z',
                data: { plan: { chunks: [{ id: 'hero' }] } },
              },
              {
                stage: 'chunk',
                status: 'failed',
                attempt: 1,
                createdAt: '2026-05-13T08:00:02.000Z',
                data: { chunkId: 'hero', name: 'Hero', status: 'failed', error: 'missing hero' },
              },
              {
                stage: 'quality_check',
                status: 'failed',
                attempt: 1,
                createdAt: '2026-05-13T08:00:03.000Z',
                data: {
                  report: {
                    status: 'failed',
                    framework: 'html',
                    issues: [{ code: 'missing_text', severity: 'error', message: 'Missing CTA' }],
                    checkedAt: '2026-05-13T08:00:03.000Z',
                    summary: {
                      fileCount: 1,
                      errorCount: 1,
                      warningCount: 0,
                      missingTextCount: 1,
                      missingAssetCount: 0,
                    },
                  },
                },
              },
              {
                stage: 'repair',
                status: 'succeeded',
                attempt: 2,
                createdAt: '2026-05-13T08:00:04.000Z',
                data: {
                  attempt: 1,
                  error: 'Repair provider timed out once',
                  issues: [
                    {
                      code: 'missing_asset',
                      severity: 'warning',
                      message: 'Hero image still missing',
                    },
                  ],
                  report: {
                    status: 'repaired',
                    framework: 'html',
                    issues: [],
                    checkedAt: '2026-05-13T08:00:04.000Z',
                    summary: {
                      fileCount: 1,
                      errorCount: 0,
                      warningCount: 0,
                      missingTextCount: 0,
                      missingAssetCount: 0,
                    },
                  },
                },
              },
            ],
          },
        } as any,
      ],
    } as any);

    render(<TaskDetail jobId="job-1" />);

    expect(screen.getByText('Stage artifacts')).toBeTruthy();
    expect(screen.getByText('planning')).toBeTruthy();
    expect(screen.getByText('chunk')).toBeTruthy();
    expect(screen.getByText('chunk: hero')).toBeTruthy();
    expect(screen.getByText('quality: failed · errors 1 · warnings 0')).toBeTruthy();
    expect(screen.getAllByText('Quality report').length).toBeGreaterThan(0);
    expect(screen.getAllByText('failed').length).toBeGreaterThan(0);
    expect(screen.getAllByText((content) => content.includes('html')).length).toBeGreaterThan(0);
    expect(screen.getAllByText('Files').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Missing text').length).toBeGreaterThan(0);
    expect(screen.getByText('missing_text')).toBeTruthy();
    expect(screen.getByText('Missing CTA')).toBeTruthy();
    expect(screen.getAllByText('repair attempt 1').length).toBeGreaterThan(0);
    expect(screen.getByText('Repair attempts')).toBeTruthy();
    expect(screen.getByText('Repair provider timed out once')).toBeTruthy();
    expect(screen.getByText('missing_asset')).toBeTruthy();
    expect(screen.getByText('Hero image still missing')).toBeTruthy();
  });

  it('expands quality warnings even when the report passed', () => {
    useCodegenJobStore.setState({
      jobs: [
        {
          ...job,
          output: {
            generationId: 'gen-1',
            pipelineMode: 'direct_generation',
            checkpoints: [
              {
                stage: 'quality_check',
                status: 'succeeded',
                attempt: 1,
                createdAt: '2026-05-13T08:00:03.000Z',
                data: {
                  report: {
                    status: 'passed',
                    framework: 'html',
                    issues: [
                      {
                        code: 'placeholder_text',
                        severity: 'warning',
                        message: 'Generated output contains placeholder text.',
                        filePath: 'index.html',
                      },
                    ],
                    checkedAt: '2026-05-13T08:00:03.000Z',
                    summary: {
                      fileCount: 1,
                      errorCount: 0,
                      warningCount: 1,
                      missingTextCount: 0,
                      missingAssetCount: 0,
                    },
                  },
                },
              },
            ],
          },
        } as any,
      ],
    } as any);

    render(<TaskDetail jobId="job-1" />);

    expect(screen.getByText('Quality warnings')).toBeTruthy();
    expect(
      screen.getByText('Passed reports can still contain warnings worth reviewing.'),
    ).toBeTruthy();
    expect(screen.getAllByText('placeholder_text').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Generated output contains placeholder text.').length).toBeGreaterThan(0);
  });

  it('renders passed warning quality reports from step output', () => {
    useCodegenJobStore.setState({
      jobs: [
        {
          ...job,
          output: {
            generationId: 'gen-1',
            pipelineMode: 'direct_generation',
          },
          steps: [
            {
              ...(job.steps[0] as any),
              id: 'step-quality',
              agentRole: 'quality_check',
              status: 'succeeded',
              progress: 100,
              output: {
                report: {
                  status: 'passed',
                  framework: 'uniapp',
                  issues: [
                    {
                      code: 'placeholder_text',
                      severity: 'warning',
                      message: 'Generated output contains placeholder text.',
                      filePath: 'pages/index/index.vue',
                    },
                  ],
                  checkedAt: '2026-05-13T08:00:03.000Z',
                  summary: {
                    fileCount: 6,
                    errorCount: 0,
                    warningCount: 1,
                    missingTextCount: 0,
                    missingAssetCount: 0,
                  },
                },
              },
            },
          ],
        } as any,
      ],
    } as any);

    render(<TaskDetail jobId="job-1" />);

    expect(screen.getByText('Quality report')).toBeTruthy();
    expect(screen.getAllByText((content) => content.includes('uniapp')).length).toBeGreaterThan(0);
    expect(screen.getByText('Quality warnings')).toBeTruthy();
    expect(
      screen.getByText('Passed reports can still contain warnings worth reviewing.'),
    ).toBeTruthy();
    expect(screen.getAllByText('placeholder_text').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Generated output contains placeholder text.').length).toBeGreaterThan(0);
    expect(screen.getAllByText('pages/index/index.vue').length).toBeGreaterThan(0);
  });

  it('renders codegen timing breakdown from job output', () => {
    useCodegenJobStore.setState({
      jobs: [
        {
          ...job,
          output: {
            generationId: 'gen-1',
            timing: {
              planningMs: 120,
              chunkMs: 2400,
              assemblyMs: 310,
              qualityCheckMs: 80,
              repairMs: 700,
              providerMs: 3220,
              totalMs: 3680,
            },
          },
        } as any,
      ],
    } as any);

    render(<TaskDetail jobId="job-1" />);

    expect(screen.getByText('Timing breakdown')).toBeTruthy();
    expect(screen.getByText('Planning')).toBeTruthy();
    expect(screen.getByText('Chunk generation')).toBeTruthy();
    expect(screen.getByText('Assembly')).toBeTruthy();
    expect(screen.getByText('Quality check')).toBeTruthy();
    expect(screen.getByText('Repair')).toBeTruthy();
    expect(screen.getByText('Provider total')).toBeTruthy();
    expect(screen.getByText('Total')).toBeTruthy();
    expect(screen.getByText('120 ms')).toBeTruthy();
    expect(screen.getByText('2.4 s')).toBeTruthy();
    expect(screen.getAllByText('3.7 s').length).toBeGreaterThan(0);
    expect(screen.getByText('Bottleneck insights')).toBeTruthy();
    expect(screen.getByText('Provider calls account for 88% of total time.')).toBeTruthy();
    expect(screen.getByText('Chunk generation is the slowest stage at 2.4 s.')).toBeTruthy();
  });

  it('renders repair bottleneck insights when repair is slow', () => {
    useCodegenJobStore.setState({
      jobs: [
        {
          ...job,
          output: {
            generationId: 'gen-1',
            timing: {
              planningMs: 100,
              chunkMs: 600,
              assemblyMs: 100,
              qualityCheckMs: 100,
              repairMs: 2600,
              providerMs: 3200,
              totalMs: 3500,
            },
          },
        } as any,
      ],
    } as any);

    render(<TaskDetail jobId="job-1" />);

    expect(screen.getByText('Repair took 2.6 s. Check repair prompt size and issue count.')).toBeTruthy();
  });

  it('deletes a task from the detail drawer and closes it', async () => {
    const onClose = vi.fn();
    render(<TaskDetail jobId="job-1" embedded onClose={onClose} />);

    fireEvent.click(screen.getByLabelText('Delete task'));
    fireEvent.click(await screen.findByRole('button', { name: 'Delete' }));

    await waitFor(() => {
      expect(useCodegenJobStore.getState().deleteJob).toHaveBeenCalledWith('job-1');
    });
    expect(onClose).toHaveBeenCalled();
  });
});
