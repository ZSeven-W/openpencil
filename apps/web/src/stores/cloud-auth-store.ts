import { create } from 'zustand';
import type { Session, User } from '@/services/cloud/supabase-client';
import {
  getSupabaseBrowserClient,
  getSupabaseBrowserConfig,
  isElectronCloudAuthAvailable,
} from '@/services/cloud/supabase-client';

const DESKTOP_OAUTH_REDIRECT_TO = 'openpencil://auth/callback';

type AuthStatus = 'unconfigured' | 'loading' | 'authenticated' | 'anonymous';

interface CloudAuthState {
  status: AuthStatus;
  session: Session | null;
  user: User | null;
  error: string | null;
  initialized: boolean;
  initialize: () => Promise<void>;
  signIn: (email: string, password: string) => Promise<boolean>;
  signUp: (email: string, password: string) => Promise<boolean>;
  signInWithGitHub: () => Promise<boolean>;
  signOut: () => Promise<void>;
  getAccessToken: () => Promise<string | null>;
}

let authSubscriptionStarted = false;
let oauthCallbackSubscriptionStarted = false;

function deriveStatus(session: Session | null): AuthStatus {
  if (!getSupabaseBrowserConfig()) return 'unconfigured';
  return session ? 'authenticated' : 'anonymous';
}

function getOAuthRedirectTo(): string {
  if (typeof window === 'undefined') return '';
  if (isElectronCloudAuthAvailable()) return DESKTOP_OAUTH_REDIRECT_TO;
  return `${window.location.origin}${window.location.pathname}`;
}

function getOAuthCodeFromCallbackUrl(callbackUrl: string): string | null {
  try {
    const url = new URL(callbackUrl);
    if (url.protocol !== 'openpencil:' || url.hostname !== 'auth' || url.pathname !== '/callback') {
      return null;
    }
    return url.searchParams.get('code');
  } catch {
    return null;
  }
}

async function exchangeDesktopOAuthCallback(callbackUrl: string): Promise<void> {
  const client = getSupabaseBrowserClient();
  if (!client) {
    useCloudAuthStore.setState({
      status: 'unconfigured',
      error: 'Supabase is not configured.',
      initialized: true,
    });
    return;
  }

  const code = getOAuthCodeFromCallbackUrl(callbackUrl);
  if (!code) {
    useCloudAuthStore.setState({
      status: 'anonymous',
      error: 'Invalid GitHub sign-in callback.',
      initialized: true,
    });
    return;
  }

  useCloudAuthStore.setState({ status: 'loading', error: null });
  const { data: exchangeData, error: exchangeError } =
    await client.auth.exchangeCodeForSession(code);
  const nextSession = exchangeData.session ?? null;
  if (exchangeError || !nextSession) {
    useCloudAuthStore.setState({
      status: 'anonymous',
      session: null,
      user: null,
      error: exchangeError?.message ?? 'Failed to complete GitHub sign-in.',
      initialized: true,
    });
    return;
  }

  useCloudAuthStore.setState({
    status: deriveStatus(nextSession),
    session: nextSession,
    user: nextSession.user,
    error: null,
    initialized: true,
  });
}

export const useCloudAuthStore = create<CloudAuthState>((set, get) => ({
  status: getSupabaseBrowserConfig() ? 'loading' : 'unconfigured',
  session: null,
  user: null,
  error: null,
  initialized: false,

  initialize: async () => {
    const client = getSupabaseBrowserClient();
    if (!client) {
      set({ status: 'unconfigured', initialized: true });
      return;
    }

    set({ status: 'loading', error: null });
    const { data, error } = await client.auth.getSession();
    const session = data.session ?? null;
    set({
      status: deriveStatus(session),
      session,
      user: session?.user ?? null,
      error: error?.message ?? null,
      initialized: true,
    });

    if (!authSubscriptionStarted) {
      authSubscriptionStarted = true;
      client.auth.onAuthStateChange((_event, nextSession) => {
        set({
          status: deriveStatus(nextSession),
          session: nextSession,
          user: nextSession?.user ?? null,
          error: null,
          initialized: true,
        });
      });
    }

    if (!oauthCallbackSubscriptionStarted && isElectronCloudAuthAvailable()) {
      oauthCallbackSubscriptionStarted = true;
      window.electronAPI?.cloudAuth.onOAuthCallback((callbackUrl) => {
        void exchangeDesktopOAuthCallback(callbackUrl);
      });
      const pendingCallback = await window.electronAPI?.cloudAuth.getPendingOAuthCallback();
      if (pendingCallback) {
        await exchangeDesktopOAuthCallback(pendingCallback);
      }
    }
  },

  signIn: async (email, password) => {
    const client = getSupabaseBrowserClient();
    if (!client) {
      set({ status: 'unconfigured', error: 'Supabase is not configured.' });
      return false;
    }
    set({ status: 'loading', error: null });
    const { data, error } = await client.auth.signInWithPassword({ email, password });
    if (error) {
      set({ status: 'anonymous', error: error.message, session: null, user: null });
      return false;
    }
    set({
      status: deriveStatus(data.session),
      session: data.session,
      user: data.user,
      error: null,
      initialized: true,
    });
    return true;
  },

  signUp: async (email, password) => {
    const client = getSupabaseBrowserClient();
    if (!client) {
      set({ status: 'unconfigured', error: 'Supabase is not configured.' });
      return false;
    }
    set({ status: 'loading', error: null });
    const { data, error } = await client.auth.signUp({ email, password });
    if (error) {
      set({ status: 'anonymous', error: error.message, session: null, user: null });
      return false;
    }
    set({
      status: deriveStatus(data.session),
      session: data.session,
      user: data.user,
      error: data.session ? null : 'Check your email to confirm your account, then sign in.',
      initialized: true,
    });
    return true;
  },

  signInWithGitHub: async () => {
    const client = getSupabaseBrowserClient();
    if (!client) {
      set({ status: 'unconfigured', error: 'Supabase is not configured.' });
      return false;
    }

    set({ status: 'loading', error: null });
    const { data, error } = await client.auth.signInWithOAuth({
      provider: 'github',
      options: {
        redirectTo: getOAuthRedirectTo(),
        ...(isElectronCloudAuthAvailable() ? { skipBrowserRedirect: true } : {}),
      },
    });
    if (error) {
      set({ status: 'anonymous', error: error.message, session: null, user: null });
      return false;
    }

    if (!data.url) {
      set({ status: 'anonymous', error: 'Failed to start GitHub sign-in flow.' });
      return false;
    }

    if (isElectronCloudAuthAvailable()) {
      await window.electronAPI?.cloudAuth.openOAuthUrl(data.url);
    }

    return true;
  },

  signOut: async () => {
    const client = getSupabaseBrowserClient();
    if (client) await client.auth.signOut();
    set({
      status: getSupabaseBrowserConfig() ? 'anonymous' : 'unconfigured',
      session: null,
      user: null,
      error: null,
      initialized: true,
    });
  },

  getAccessToken: async () => {
    const client = getSupabaseBrowserClient();
    if (!client) return null;
    const current = get().session;
    if (current?.access_token) return current.access_token;
    const { data } = await client.auth.getSession();
    const session = data.session ?? null;
    set({
      status: deriveStatus(session),
      session,
      user: session?.user ?? null,
      initialized: true,
    });
    return session?.access_token ?? null;
  },
}));

export const __cloudAuthTestExports = {
  DESKTOP_OAUTH_REDIRECT_TO,
  exchangeDesktopOAuthCallback,
  getOAuthCodeFromCallbackUrl,
  resetSubscriptions: () => {
    authSubscriptionStarted = false;
    oauthCallbackSubscriptionStarted = false;
  },
};
