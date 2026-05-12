import { createClient, type Session, type SupabaseClient, type User } from '@supabase/supabase-js';

let browserClient: SupabaseClient | null = null;
const CLOUD_AUTH_STORAGE_KEY = 'openpencil-cloud-auth';

export function isElectronCloudAuthAvailable(): boolean {
  return typeof window !== 'undefined' && Boolean(window.electronAPI?.cloudAuth);
}

export function getSupabaseBrowserConfig(): { url: string; anonKey: string } | null {
  const env = import.meta.env as Record<string, string | undefined>;
  const url = env.VITE_SUPABASE_URL;
  const anonKey = env.VITE_SUPABASE_ANON_KEY;
  if (!url || !anonKey) return null;
  return { url, anonKey };
}

export function getSupabaseBrowserClient(): SupabaseClient | null {
  if (browserClient) return browserClient;
  const config = getSupabaseBrowserConfig();
  if (!config) return null;
  const isElectron = isElectronCloudAuthAvailable();
  browserClient = createClient(config.url, config.anonKey, {
    auth: {
      storageKey: CLOUD_AUTH_STORAGE_KEY,
      storage: isElectron ? window.electronAPI?.cloudAuth : undefined,
      persistSession: true,
      autoRefreshToken: true,
      detectSessionInUrl: !isElectron,
      flowType: isElectron ? 'pkce' : 'implicit',
    },
  });
  return browserClient;
}

export type { Session, User };
