import { create } from 'zustand';
import type { Session, User } from '@/services/cloud/supabase-client';
import { getSupabaseBrowserClient, getSupabaseBrowserConfig } from '@/services/cloud/supabase-client';

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
  signOut: () => Promise<void>;
  getAccessToken: () => Promise<string | null>;
}

let authSubscriptionStarted = false;

function deriveStatus(session: Session | null): AuthStatus {
  if (!getSupabaseBrowserConfig()) return 'unconfigured';
  return session ? 'authenticated' : 'anonymous';
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

