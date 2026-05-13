import { createFileRoute } from '@tanstack/react-router';
import { AuthGate } from '@/components/cloud/auth-gate';
import { WorkerManagement } from '@/components/tasks/worker-management';

export const Route = createFileRoute('/tasks/workers')({
  component: WorkerManagementPage,
  ssr: false,
  head: () => ({
    meta: [{ title: 'OpenPencil Worker Management' }],
  }),
});

function WorkerManagementPage() {
  return (
    <AuthGate>
      <WorkerManagement />
    </AuthGate>
  );
}
