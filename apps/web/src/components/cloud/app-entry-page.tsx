import { AuthGate } from '@/components/cloud/auth-gate';
import { CloudFileLibrary } from '@/components/cloud/cloud-file-library';
import { DesktopStartPage } from '@/components/cloud/desktop-start-page';
import { isElectron } from '@/utils/file-operations';

export function AppEntryPage() {
  if (typeof window !== 'undefined' && isElectron()) {
    return <DesktopStartPage />;
  }

  return (
    <AuthGate>
      <CloudFileLibrary />
    </AuthGate>
  );
}
