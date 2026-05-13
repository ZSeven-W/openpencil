import { createClient, type SupabaseClient, type User } from '@supabase/supabase-js';
import { createError, getRequestHeader, type H3Event } from 'h3';

export interface CloudSupabaseContext {
  supabase: SupabaseClient;
  user: User;
  token: string;
}

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
  const { data, error } = await supabase.auth.getUser(token);
  if (error || !data.user) {
    throw createError({ statusCode: 401, statusMessage: 'Invalid bearer token' });
  }
  return { supabase, user: data.user, token };
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
