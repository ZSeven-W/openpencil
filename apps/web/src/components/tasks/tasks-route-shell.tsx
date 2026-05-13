import { Outlet, useLocation } from '@tanstack/react-router';
import { AuthGate } from '@/components/cloud/auth-gate';
import { TaskCenter } from '@/components/tasks/task-center';

interface TasksRouteShellProps {
  fileId?: string;
}

export function TasksRouteShell({ fileId }: TasksRouteShellProps) {
  const location = useLocation();

  if (location.pathname !== '/tasks') {
    return <Outlet />;
  }

  return (
    <AuthGate>
      <TaskCenter fileId={fileId} />
    </AuthGate>
  );
}
