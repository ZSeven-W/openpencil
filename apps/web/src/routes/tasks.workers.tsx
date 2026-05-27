import { createFileRoute } from '@tanstack/react-router';
import { z } from 'zod';
import { AuthGate } from '@/components/cloud/auth-gate';
import { WorkerManagement } from '@/components/tasks/worker-management';

export const Route = createFileRoute('/tasks/workers')({
  validateSearch: z.object({
    view: z.enum(['workers', 'providers', 'audit']).optional(),
  }),
  component: WorkerManagementPage,
  ssr: false,
  head: () => ({
    meta: [{ title: 'OpenPencil Worker Management' }],
  }),
});

function WorkerManagementPage() {
  const { view } = Route.useSearch();
  return (
    <AuthGate>
      <WorkerManagement view={view} />
    </AuthGate>
  );
}
