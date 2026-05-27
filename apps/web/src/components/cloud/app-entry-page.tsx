import { AuthGate } from '@/components/cloud/auth-gate';
import { DesktopStartPage } from '@/components/cloud/desktop-start-page';
import { WorkbenchHome } from '@/components/workbench/workbench-home';
import { isElectron } from '@/utils/file-operations';

export function AppEntryPage() {
  if (typeof window !== 'undefined' && isElectron()) {
    return <DesktopStartPage />;
  }

  return (
    <AuthGate>
      <WorkbenchHome />
    </AuthGate>
  );
}
