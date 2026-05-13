import { createFileRoute } from '@tanstack/react-router';
import { AuthGate } from '@/components/cloud/auth-gate';
import { TaskDetail } from '@/components/tasks/task-detail';

export const Route = createFileRoute('/tasks/$jobId')({
  component: TaskDetailPage,
  ssr: false,
  head: () => ({
    meta: [{ title: 'OpenPencil Task Detail' }],
  }),
});

function TaskDetailPage() {
  const { jobId } = Route.useParams();
  return (
    <AuthGate>
      <TaskDetail jobId={jobId} />
    </AuthGate>
  );
}
