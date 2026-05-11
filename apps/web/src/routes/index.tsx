import { createFileRoute } from '@tanstack/react-router';
import { AuthGate } from '@/components/cloud/auth-gate';
import { CloudFileLibrary } from '@/components/cloud/cloud-file-library';

export const Route = createFileRoute('/')({
  component: LandingPage,
  head: () => ({
    meta: [{ title: 'OpenPencil - Design as Code' }],
  }),
});

function LandingPage() {
  return (
    <AuthGate>
      <CloudFileLibrary />
    </AuthGate>
  );
}
