// @vitest-environment jsdom

import { cleanup, render } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useCloudAuthStore } from '@/stores/cloud-auth-store';
import { useCodegenJobStore } from '@/stores/codegen-job-store';
import '@/i18n';
import {
  TaskNotificationListener,
  shouldPollTaskNotifications,
} from './task-notification-listener';

const routerMocks = vi.hoisted(() => ({
  navigate: vi.fn(),
  location: { pathname: '/editor/file-1', search: '' },
}));

vi.mock('@tanstack/react-router', () => ({
  useLocation: () => routerMocks.location,
  useNavigate: () => routerMocks.navigate,
}));

describe('TaskNotificationListener', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    routerMocks.location = { pathname: '/editor/file-1', search: '' };
    useCloudAuthStore.setState({
      initialized: true,
      status: 'authenticated',
    } as any);
    useCodegenJobStore.setState({
      refreshNotifications: vi.fn(async () => undefined),
      notifyUnreadCompletions: vi.fn(async () => undefined),
    } as any);
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it('does not poll global notifications on task detail pages', async () => {
    routerMocks.location = { pathname: '/tasks/job-1', search: '' };

    render(<TaskNotificationListener />);
    await vi.advanceTimersByTimeAsync(20_000);

    expect(useCodegenJobStore.getState().refreshNotifications).not.toHaveBeenCalled();
    expect(useCodegenJobStore.getState().notifyUnreadCompletions).not.toHaveBeenCalled();
  });

  it('does not poll global notifications when the task drawer is open from search params', async () => {
    routerMocks.location = { pathname: '/tasks', search: '?jobId=job-1' };

    render(<TaskNotificationListener />);
    await vi.advanceTimersByTimeAsync(20_000);

    expect(useCodegenJobStore.getState().refreshNotifications).not.toHaveBeenCalled();
    expect(useCodegenJobStore.getState().notifyUnreadCompletions).not.toHaveBeenCalled();
  });

  it('keeps global completion polling on non-task pages', async () => {
    render(<TaskNotificationListener />);
    await vi.advanceTimersByTimeAsync(5_000);

    expect(useCodegenJobStore.getState().refreshNotifications).toHaveBeenCalledWith({
      limit: 10,
    });
    expect(useCodegenJobStore.getState().notifyUnreadCompletions).toHaveBeenCalledTimes(1);
  });
});

describe('shouldPollTaskNotifications', () => {
  it('skips task center and task detail routes to avoid duplicate task data refreshes', () => {
    expect(shouldPollTaskNotifications('/tasks', '')).toBe(false);
    expect(shouldPollTaskNotifications('/tasks/job-1', '')).toBe(false);
    expect(shouldPollTaskNotifications('/tasks', '?jobId=job-1')).toBe(false);
  });

  it('polls outside task routes so completion toasts still work while editing', () => {
    expect(shouldPollTaskNotifications('/editor/file-1', '')).toBe(true);
    expect(shouldPollTaskNotifications('/cloud', '')).toBe(true);
  });
});
