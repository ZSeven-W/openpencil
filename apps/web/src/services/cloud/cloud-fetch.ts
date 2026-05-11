import { useCloudAuthStore } from '@/stores/cloud-auth-store';

export class CloudApiError extends Error {
  status: number;
  code: string;
  details?: unknown;

  constructor(status: number, code: string, message: string, details?: unknown) {
    super(message);
    this.name = 'CloudApiError';
    this.status = status;
    this.code = code;
    this.details = details;
  }
}

async function parseJsonSafely(response: Response): Promise<unknown> {
  const text = await response.text();
  if (!text) return null;
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}

export async function cloudFetch<T>(path: string, init: RequestInit = {}): Promise<T> {
  const token = await useCloudAuthStore.getState().getAccessToken();
  if (!token) {
    throw new CloudApiError(401, 'unauthorized', 'Sign in to use cloud files.');
  }

  const headers = new Headers(init.headers);
  headers.set('Authorization', `Bearer ${token}`);
  if (init.body && !(init.body instanceof FormData) && !headers.has('Content-Type')) {
    headers.set('Content-Type', 'application/json');
  }

  const response = await fetch(path, { ...init, headers });
  const payload = await parseJsonSafely(response);

  if (!response.ok) {
    const err = payload as {
      error?: { code?: string; message?: string; details?: unknown };
      statusMessage?: string;
      message?: string;
    } | null;
    throw new CloudApiError(
      response.status,
      err?.error?.code ?? 'request_failed',
      err?.error?.message ?? err?.statusMessage ?? err?.message ?? response.statusText,
      err?.error?.details,
    );
  }

  return payload as T;
}

