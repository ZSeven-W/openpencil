import { createFileRoute } from '@tanstack/react-router';
import { z } from 'zod';
import { TasksRouteShell } from '@/components/tasks/tasks-route-shell';

const taskWorkbenchViewSchema = z.enum([
  'tasks',
  'notifications',
  'workers',
  'providers',
  'audit',
]);

export const Route = createFileRoute('/tasks')({
  validateSearch: z.object({
    fileId: z.string().optional(),
    jobId: z.string().optional(),
    view: taskWorkbenchViewSchema.optional(),
  }),
  component: TasksPage,
  ssr: false,
  head: () => ({
    meta: [{ title: 'OpenPencil Tasks' }],
  }),
});

function TasksPage() {
  const { fileId, jobId, view } = Route.useSearch();
  return (
    <TasksRouteShell
      fileId={fileId}
      jobId={jobId}
      view={view === 'notifications' ? 'notifications' : 'tasks'}
    />
  );
}
