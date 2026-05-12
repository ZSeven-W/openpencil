import { useEffect } from 'react';
import { Outlet, createFileRoute, useLocation, useNavigate } from '@tanstack/react-router';
import { AuthGate } from '@/components/cloud/auth-gate';

export const Route = createFileRoute('/editor')({
  component: EditorPage,
  ssr: false,
  head: () => ({
    meta: [{ title: 'OpenPencil Editor' }],
  }),
});

function EditorPage() {
  const navigate = useNavigate();
  const location = useLocation();

  useEffect(() => {
    if (location.pathname === '/editor') {
      void navigate({ to: '/cloud' });
    }
  }, [location.pathname, navigate]);

  if (location.pathname !== '/editor') {
    return <Outlet />;
  }

  return (
    <AuthGate>
      <div className="min-h-screen bg-background text-foreground flex items-center justify-center">
        <span className="text-sm text-muted-foreground">Redirecting to cloud files...</span>
      </div>
    </AuthGate>
  );
}
