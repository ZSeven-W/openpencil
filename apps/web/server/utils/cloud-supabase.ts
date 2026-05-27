import { createClient, type SupabaseClient, type User } from '@supabase/supabase-js';
import { createError, getRequestHeader, type H3Event } from 'h3';

type CloudUser = Pick<User, 'id' | 'email'> & Partial<User>;
type JwtClaims = {
  sub?: unknown;
  email?: unknown;
  exp?: unknown;
  role?: unknown;
  aud?: unknown;
};

export interface CloudSupabaseContext {
  supabase: SupabaseClient;
  user: CloudUser;
  token: string;
}

const DEFAULT_AUTH_CONTEXT_CACHE_MS = 30_000;
const authContextCache = new Map<string, { user: CloudUser; expiresAt: number }>();

export function getCloudEnv(): { url: string; anonKey: string } {
  const url = process.env.SUPABASE_URL || process.env.VITE_SUPABASE_URL;
  const anonKey = process.env.SUPABASE_ANON_KEY || process.env.VITE_SUPABASE_ANON_KEY;
  if (!url || !anonKey) {
    throw createError({
      statusCode: 500,
      statusMessage: 'Supabase is not configured. Set SUPABASE_URL and SUPABASE_ANON_KEY.',
    });
  }
  return { url, anonKey };
}

export function getCloudServiceSupabase(): SupabaseClient | null {
  const url = process.env.SUPABASE_URL || process.env.VITE_SUPABASE_URL;
  const serviceKey = process.env.SUPABASE_SERVICE_ROLE_KEY;
  if (!url || !serviceKey) return null;
  return createClient(url, serviceKey, {
    auth: {
      persistSession: false,
      autoRefreshToken: false,
      detectSessionInUrl: false,
    },
  });
}

export function getBearerToken(event: H3Event): string {
  const header = getRequestHeader(event, 'authorization');
  const match = /^Bearer\s+(.+)$/i.exec(header ?? '');
  const token = match?.[1]?.trim();
  if (!token) {
    throw createError({ statusCode: 401, statusMessage: 'Missing bearer token' });
  }
  return token;
}

function getAuthContextCacheMs(): number {
  const parsed = Number(process.env.OPENPENCIL_CLOUD_AUTH_CACHE_MS);
  return Number.isFinite(parsed) && parsed >= 0 ? parsed : DEFAULT_AUTH_CONTEXT_CACHE_MS;
}

function userFromClaims(claims: JwtClaims): CloudUser | null {
  if (typeof claims.sub !== 'string' || claims.sub.length === 0) return null;
  return {
    id: claims.sub,
    email: typeof claims.email === 'string' ? claims.email : undefined,
    role: typeof claims.role === 'string' ? claims.role : undefined,
    aud: typeof claims.aud === 'string' ? claims.aud : undefined,
  } as CloudUser;
}

function cacheUserForToken(token: string, user: CloudUser, claims?: JwtClaims): void {
  const ttlMs = getAuthContextCacheMs();
  if (ttlMs <= 0) return;
  const now = Date.now();
  const claimExpiresAt =
    typeof claims?.exp === 'number' && Number.isFinite(claims.exp) ? claims.exp * 1000 : Infinity;
  const expiresAt = Math.min(now + ttlMs, claimExpiresAt);
  if (expiresAt <= now) return;
  authContextCache.set(token, { user, expiresAt });
}

async function verifyCloudUser(supabase: SupabaseClient, token: string): Promise<CloudUser> {
  const cached = authContextCache.get(token);
  if (cached && cached.expiresAt > Date.now()) return cached.user;
  if (cached) authContextCache.delete(token);

  const auth = supabase.auth as SupabaseClient['auth'] & {
    getClaims?: (jwt?: string) => Promise<{
      data: { claims: JwtClaims } | null;
      error: Error | null;
    }>;
  };

  if (typeof auth.getClaims === 'function') {
    const { data, error } = await auth.getClaims(token);
    if (!error && data?.claims) {
      const user = userFromClaims(data.claims);
      if (user) {
        cacheUserForToken(token, user, data.claims);
        return user;
      }
    }
  }

  if (typeof supabase.auth.getUser !== 'function') {
    throw createError({ statusCode: 401, statusMessage: 'Invalid bearer token' });
  }

  const { data, error } = await supabase.auth.getUser(token);
  if (error || !data.user) {
    throw createError({ statusCode: 401, statusMessage: 'Invalid bearer token' });
  }
  cacheUserForToken(token, data.user);
  return data.user;
}

export async function getCloudSupabase(event: H3Event): Promise<CloudSupabaseContext> {
  const token = getBearerToken(event);
  const { url, anonKey } = getCloudEnv();
  const supabase = createClient(url, anonKey, {
    global: {
      headers: {
        Authorization: `Bearer ${token}`,
      },
    },
    auth: {
      persistSession: false,
      autoRefreshToken: false,
      detectSessionInUrl: false,
    },
  });
  const user = await verifyCloudUser(supabase, token);
  return { supabase, user, token };
}

export function toApiError(statusCode: number, code: string, message: string, details?: unknown) {
  return createError({
    statusCode,
    statusMessage: message,
    data: {
      error: { code, message, details },
    },
  });
}
