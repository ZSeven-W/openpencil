import { useEffect, useState, type ReactNode } from 'react';
import { PenTool } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useCloudAuthStore } from '@/stores/cloud-auth-store';

interface AuthGateProps {
  children: ReactNode;
}

export function AuthGate({ children }: AuthGateProps) {
  const status = useCloudAuthStore((s) => s.status);
  const error = useCloudAuthStore((s) => s.error);
  const initialized = useCloudAuthStore((s) => s.initialized);
  const initialize = useCloudAuthStore((s) => s.initialize);
  const signIn = useCloudAuthStore((s) => s.signIn);
  const signUp = useCloudAuthStore((s) => s.signUp);
  const [mode, setMode] = useState<'sign-in' | 'sign-up'>('sign-in');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void initialize();
  }, [initialize]);

  if (!initialized || status === 'loading') {
    return (
      <div className="min-h-screen bg-background text-foreground flex items-center justify-center">
        <span className="text-sm text-muted-foreground">Loading cloud workspace...</span>
      </div>
    );
  }

  if (status === 'unconfigured') {
    return (
      <div className="min-h-screen bg-background text-foreground flex items-center justify-center px-6">
        <div className="w-full max-w-md border border-border bg-card p-6">
          <div className="flex items-center gap-3">
            <PenTool size={24} className="text-primary" />
            <div>
              <h1 className="text-lg font-semibold">Supabase is not configured</h1>
              <p className="mt-1 text-sm text-muted-foreground">
                Set VITE_SUPABASE_URL and VITE_SUPABASE_ANON_KEY for the renderer, plus
                SUPABASE_URL and SUPABASE_ANON_KEY for Nitro.
              </p>
            </div>
          </div>
        </div>
      </div>
    );
  }

  if (status === 'authenticated') {
    return <>{children}</>;
  }

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    setBusy(true);
    try {
      const ok =
        mode === 'sign-in' ? await signIn(email, password) : await signUp(email, password);
      if (ok) {
        setPassword('');
      }
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="min-h-screen bg-background text-foreground flex items-center justify-center px-6">
      <form onSubmit={submit} className="w-full max-w-sm border border-border bg-card p-6">
        <div className="mb-6 flex items-center gap-3">
          <PenTool size={28} className="text-primary" />
          <div>
            <h1 className="text-xl font-semibold">OpenPencil Cloud</h1>
            <p className="text-sm text-muted-foreground">Sign in to manage your design files.</p>
          </div>
        </div>

        <label className="block text-xs font-medium text-muted-foreground" htmlFor="cloud-email">
          Email
        </label>
        <input
          id="cloud-email"
          type="email"
          required
          value={email}
          onChange={(event) => setEmail(event.target.value)}
          className="mt-1 h-9 w-full border border-border bg-background px-3 text-sm outline-none focus:border-primary"
        />

        <label
          className="mt-4 block text-xs font-medium text-muted-foreground"
          htmlFor="cloud-password"
        >
          Password
        </label>
        <input
          id="cloud-password"
          type="password"
          required
          minLength={6}
          value={password}
          onChange={(event) => setPassword(event.target.value)}
          className="mt-1 h-9 w-full border border-border bg-background px-3 text-sm outline-none focus:border-primary"
        />

        {error && <p className="mt-3 text-xs text-destructive">{error}</p>}

        <Button type="submit" className="mt-5 w-full" disabled={busy}>
          {busy ? 'Working...' : mode === 'sign-in' ? 'Sign in' : 'Create account'}
        </Button>

        <button
          type="button"
          className="mt-4 w-full text-center text-xs text-muted-foreground hover:text-foreground"
          onClick={() => setMode(mode === 'sign-in' ? 'sign-up' : 'sign-in')}
        >
          {mode === 'sign-in' ? 'Create a new account' : 'Use an existing account'}
        </button>
      </form>
    </div>
  );
}

