import { createFileRoute } from '@tanstack/react-router';
import { z } from 'zod';
import { TasksRouteShell } from '@/components/tasks/tasks-route-shell';

export const Route = createFileRoute('/tasks')({
  validateSearch: z.object({
    fileId: z.string().optional(),
  }),
  component: TasksPage,
  ssr: false,
  head: () => ({
    meta: [{ title: 'OpenPencil Tasks' }],
  }),
});

function TasksPage() {
  const { fileId } = Route.useSearch();
  return <TasksRouteShell fileId={fileId} />;
}
