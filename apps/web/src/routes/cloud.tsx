import { createFileRoute } from '@tanstack/react-router';
import { AuthGate } from '@/components/cloud/auth-gate';
import { CloudFileLibrary } from '@/components/cloud/cloud-file-library';

export const Route = createFileRoute('/cloud')({
  component: CloudFilesPage,
  head: () => ({
    meta: [{ title: 'OpenPencil Cloud Files' }],
  }),
});

function CloudFilesPage() {
  return (
    <AuthGate>
      <CloudFileLibrary />
    </AuthGate>
  );
}
