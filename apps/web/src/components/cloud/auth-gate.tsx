import { useEffect, useState, type ReactNode } from 'react';
import { CloudOff, PenTool } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Field, FieldGroup, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import { Skeleton } from '@/components/ui/skeleton';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
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
      <div className="flex min-h-screen items-center justify-center bg-background px-6 text-foreground">
        <Card className="w-full max-w-md rounded-md">
          <CardHeader>
            <CardTitle>{t('auth.loadingCloudWorkspace')}</CardTitle>
            <CardDescription>{t('workbench.home.description')}</CardDescription>
          </CardHeader>
          <CardContent className="flex flex-col gap-3">
            <Skeleton className="h-9 w-full" />
            <Skeleton className="h-9 w-5/6" />
            <Skeleton className="h-20 w-full" />
          </CardContent>
        </Card>
      </div>
    );
  }

  if (status === 'unconfigured') {
    return (
      <div className="flex min-h-screen items-center justify-center bg-background px-6 text-foreground">
        <Card className="w-full max-w-lg rounded-md">
          <CardHeader>
            <div className="flex items-center gap-3">
              <span className="flex size-9 items-center justify-center rounded-md border border-border bg-background">
                <CloudOff />
              </span>
              <div>
                <CardTitle>{t('auth.supabaseNotConfiguredTitle')}</CardTitle>
                <CardDescription>{t('workbench.nav.auth')}</CardDescription>
              </div>
            </div>
          </CardHeader>
          <CardContent>
            <Alert>
              <CloudOff />
              <AlertTitle>{t('auth.supabaseNotConfiguredTitle')}</AlertTitle>
              <AlertDescription>{t('auth.supabaseNotConfiguredBody')}</AlertDescription>
            </Alert>
          </CardContent>
        </Card>
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
    <div className="flex min-h-screen items-center justify-center bg-background px-6 text-foreground">
      <Card className="w-full max-w-sm rounded-md">
        <CardHeader>
          <div className="flex items-center gap-3">
            <span className="flex size-10 items-center justify-center rounded-md border border-border bg-background">
              <PenTool />
            </span>
            <div>
              <CardTitle>{t('auth.cloudTitle')}</CardTitle>
              <CardDescription>{t('auth.cloudSubtitle')}</CardDescription>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <Tabs
            value={mode}
            onValueChange={(value) => setMode(value as 'sign-in' | 'sign-up')}
            className="mb-5"
          >
            <TabsList className="w-full">
              <TabsTrigger className="flex-1" value="sign-in">
                {t('auth.signIn')}
              </TabsTrigger>
              <TabsTrigger className="flex-1" value="sign-up">
                {t('auth.createAccount')}
              </TabsTrigger>
            </TabsList>
          </Tabs>
          <form onSubmit={submit}>
            <FieldGroup className="gap-4">
              <Field>
                <FieldLabel htmlFor="cloud-email">{t('auth.email')}</FieldLabel>
                <Input
                  id="cloud-email"
                  type="email"
                  required
                  value={email}
                  onChange={(event) => setEmail(event.target.value)}
                />
              </Field>
              <Field>
                <FieldLabel htmlFor="cloud-password">{t('auth.password')}</FieldLabel>
                <Input
                  id="cloud-password"
                  type="password"
                  required
                  minLength={6}
                  value={password}
                  onChange={(event) => setPassword(event.target.value)}
                />
              </Field>
            </FieldGroup>

            {error && (
              <Alert variant="destructive" className="mt-4">
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}

            <Button type="submit" className="mt-5 w-full" disabled={busy}>
              {busy
                ? t('auth.working')
                : mode === 'sign-in'
                  ? t('auth.signIn')
                  : t('auth.createAccount')}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
