// @vitest-environment jsdom

import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const routerMocks = vi.hoisted(() => ({
  pathname: '/tasks',
  search: {} as { fileId?: string },
}));

vi.mock('@tanstack/react-router', () => ({
  Outlet: () => <div>Nested Tasks Route</div>,
  useLocation: () => ({ pathname: routerMocks.pathname }),
}));

vi.mock('@/components/cloud/auth-gate', () => ({
  AuthGate: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="auth-gate">{children}</div>
  ),
}));

vi.mock('@/components/tasks/task-center', () => ({
  TaskCenter: ({ fileId }: { fileId?: string }) => (
    <div>Task Center Route {fileId ?? 'global'}</div>
  ),
}));

import { TasksRouteShell } from '@/components/tasks/tasks-route-shell';

beforeEach(() => {
  routerMocks.pathname = '/tasks';
  routerMocks.search = {};
});

afterEach(() => cleanup());

describe('TasksPage route shell', () => {
  it('renders the task center only for the /tasks index route', () => {
    render(<TasksRouteShell />);

    expect(screen.getByTestId('auth-gate')).toBeTruthy();
    expect(screen.getByText('Task Center Route global')).toBeTruthy();
    expect(screen.queryByText('Nested Tasks Route')).toBeNull();
  });

  it('passes fileId search to the task center on the /tasks index route', () => {
    routerMocks.search = { fileId: 'file-1' };

    render(<TasksRouteShell fileId={routerMocks.search.fileId} />);

    expect(screen.getByText('Task Center Route file-1')).toBeTruthy();
  });

  it('renders child routes for /tasks/workers instead of swallowing them with TaskCenter', () => {
    routerMocks.pathname = '/tasks/workers';

    render(<TasksRouteShell />);

    expect(screen.getByText('Nested Tasks Route')).toBeTruthy();
    expect(screen.queryByText(/Task Center Route/)).toBeNull();
  });

  it('renders child routes for /tasks/$jobId instead of swallowing them with TaskCenter', () => {
    routerMocks.pathname = '/tasks/job-1';

    render(<TasksRouteShell />);

    expect(screen.getByText('Nested Tasks Route')).toBeTruthy();
    expect(screen.queryByText(/Task Center Route/)).toBeNull();
  });
});
