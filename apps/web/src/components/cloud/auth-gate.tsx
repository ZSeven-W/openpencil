import { useEffect, useState, type ReactNode } from 'react';
import { PenTool } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { useCloudAuthStore } from '@/stores/cloud-auth-store';

interface AuthGateProps {
  children: ReactNode;
}

export function AuthGate({ children }: AuthGateProps) {
  const { t } = useTranslation();
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
        <span className="text-sm text-muted-foreground">{t('auth.loadingCloudWorkspace')}</span>
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
              <h1 className="text-lg font-semibold">{t('auth.supabaseNotConfiguredTitle')}</h1>
              <p className="mt-1 text-sm text-muted-foreground">
                {t('auth.supabaseNotConfiguredBody')}
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
            <h1 className="text-xl font-semibold">{t('auth.cloudTitle')}</h1>
            <p className="text-sm text-muted-foreground">{t('auth.cloudSubtitle')}</p>
          </div>
        </div>

        <label className="block text-xs font-medium text-muted-foreground" htmlFor="cloud-email">
          {t('auth.email')}
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
          {t('auth.password')}
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
          {busy
            ? t('auth.working')
            : mode === 'sign-in'
              ? t('auth.signIn')
              : t('auth.createAccount')}
        </Button>

        <button
          type="button"
          className="mt-4 w-full text-center text-xs text-muted-foreground hover:text-foreground"
          onClick={() => setMode(mode === 'sign-in' ? 'sign-up' : 'sign-in')}
        >
          {mode === 'sign-in' ? t('auth.createNewAccount') : t('auth.useExistingAccount')}
        </button>
      </form>
    </div>
  );
}
