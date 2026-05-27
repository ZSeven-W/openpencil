import { useCloudAuthStore } from '@/stores/cloud-auth-store';

export type CloudFetchInit = RequestInit & {
  cacheTtlMs?: number;
  dedupe?: boolean;
  force?: boolean;
};

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

type CachedPayload = {
  expiresAt: number;
  value: unknown;
};

const DEFAULT_GET_CACHE_TTL_MS = 10_000;
const TASK_GET_CACHE_TTL_MS = 5_000;
const CLOUD_FILE_LIST_GET_CACHE_TTL_MS = 15_000;
const DETAIL_GET_CACHE_TTL_MS = 30_000;
const DEFAULT_SLOW_REQUEST_MS = 800;

const cloudJsonCache = new Map<string, CachedPayload>();
const cloudJsonInFlight = new Map<string, Promise<unknown>>();

function getMethod(init: RequestInit): string {
  return (init.method ?? 'GET').toUpperCase();
}

function isCloudApiPath(path: string): boolean {
  return path.startsWith('/api/cloud') || path.includes('/api/cloud/');
}

function isDevRuntime(): boolean {
  return Boolean(import.meta.env?.DEV) && import.meta.env?.MODE !== 'test';
}

function getSlowRequestThresholdMs(): number {
  const raw = import.meta.env?.VITE_OPENPENCIL_CLOUD_SLOW_MS;
  const parsed = raw ? Number(raw) : Number.NaN;
  return Number.isFinite(parsed) && parsed > 0 ? parsed : DEFAULT_SLOW_REQUEST_MS;
}

function cacheKeyFor(path: string, method: string, headers: Headers): string {
  return [
    method,
    path,
    headers.get('Authorization') ?? '',
    headers.get('Accept') ?? '',
    headers.get('Content-Type') ?? '',
  ].join('\n');
}

function shouldUseJsonCache(path: string, method: string, init: CloudFetchInit): boolean {
  return method === 'GET' && isCloudApiPath(path) && init.force !== true;
}

function getCloudPathname(path: string): string {
  try {
    return new URL(path, 'http://openpencil.local').pathname;
  } catch {
    return path.split('?')[0] ?? path;
  }
}

function isTaskListPath(pathname: string): boolean {
  return (
    pathname === '/api/cloud/codegen-jobs' ||
    pathname === '/api/cloud/codegen-workers' ||
    pathname === '/api/cloud/codegen-worker-stats' ||
    pathname === '/api/cloud/codegen-queue-access' ||
    pathname === '/api/cloud/codegen-provider-configs' ||
    pathname === '/api/cloud/task-notifications'
  );
}

function isCloudFileManagementListPath(pathname: string): boolean {
  return (
    pathname === '/api/cloud/files' ||
    pathname === '/api/cloud/projects' ||
    pathname === '/api/cloud/folders' ||
    pathname === '/api/cloud/code-generations'
  );
}

function isCloudDetailPath(pathname: string): boolean {
  return (
    /^\/api\/cloud\/files\/[^/]+(?:\/(?:versions|activity|shares))?$/.test(pathname) ||
    /^\/api\/cloud\/codegen-jobs\/[^/]+$/.test(pathname) ||
    /^\/api\/cloud\/code-generations\/[^/]+(?:\/files)?$/.test(pathname)
  );
}

function getDefaultCacheTtlMs(path: string): number {
  const pathname = getCloudPathname(path);
  if (isTaskListPath(pathname)) return TASK_GET_CACHE_TTL_MS;
  if (isCloudFileManagementListPath(pathname)) return CLOUD_FILE_LIST_GET_CACHE_TTL_MS;
  if (isCloudDetailPath(pathname)) return DETAIL_GET_CACHE_TTL_MS;
  return DEFAULT_GET_CACHE_TTL_MS;
}

function getCacheTtlMs(path: string, method: string, init: CloudFetchInit): number {
  if (method !== 'GET' || !isCloudApiPath(path)) return 0;
  if (init.cacheTtlMs !== undefined) return Math.max(0, init.cacheTtlMs);
  return getDefaultCacheTtlMs(path);
}

function logCloudRequest(input: {
  path: string;
  method: string;
  status?: number;
  durationMs: number;
  cacheStatus?: 'cache-hit' | 'dedupe-hit';
}) {
  if (!isDevRuntime()) return;

  const rounded = Math.round(input.durationMs);
  if (input.cacheStatus) {
    console.debug(
      `[cloudFetch] ${input.cacheStatus} ${input.method} ${input.path} (${rounded}ms)`,
    );
    return;
  }

  if (rounded >= getSlowRequestThresholdMs()) {
    console.warn(
      `[cloudFetch] slow ${input.method} ${input.path} ${input.status ?? '-'} (${rounded}ms)`,
    );
  } else {
    console.debug(
      `[cloudFetch] ${input.method} ${input.path} ${input.status ?? '-'} (${rounded}ms)`,
    );
  }
}

export function clearCloudFetchCache(
  match?: string | RegExp | ((key: string) => boolean),
): void {
  if (!match) {
    cloudJsonCache.clear();
    cloudJsonInFlight.clear();
    return;
  }

  const shouldDelete =
    typeof match === 'function'
      ? match
      : typeof match === 'string'
        ? (key: string) => key.includes(match)
        : (key: string) => match.test(key);

  for (const key of cloudJsonCache.keys()) {
    if (shouldDelete(key)) cloudJsonCache.delete(key);
  }
  for (const key of cloudJsonInFlight.keys()) {
    if (shouldDelete(key)) cloudJsonInFlight.delete(key);
  }
}

function invalidateCloudFetchCacheForMutation(path: string, method: string): void {
  if (method === 'GET' || !isCloudApiPath(path)) return;
  clearCloudFetchCache('/api/cloud');
}

async function getCloudRequestHeaders(init: RequestInit): Promise<Headers> {
  const token = await useCloudAuthStore.getState().getAccessToken();
  if (!token) {
    throw new CloudApiError(401, 'unauthorized', 'Sign in to use cloud files.');
  }

  const headers = new Headers(init.headers);
  headers.set('Authorization', `Bearer ${token}`);
  if (init.body && !(init.body instanceof FormData) && !headers.has('Content-Type')) {
    headers.set('Content-Type', 'application/json');
  }
  return headers;
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

async function fetchCloudResponse(
  path: string,
  init: CloudFetchInit,
  headers: Headers,
): Promise<Response> {
  const method = getMethod(init);
  const startedAt = Date.now();
  const response = await fetch(path, { ...init, headers });
  logCloudRequest({
    path,
    method,
    status: response.status,
    durationMs: Date.now() - startedAt,
  });
  return response;
}

function toCloudApiError(response: Response, payload: unknown): CloudApiError {
  const err = payload as {
    error?: { code?: string; message?: string; details?: unknown };
    statusMessage?: string;
    message?: string;
  } | null;
  return new CloudApiError(
    response.status,
    err?.error?.code ?? 'request_failed',
    err?.error?.message ?? err?.statusMessage ?? err?.message ?? response.statusText,
    err?.error?.details,
  );
}

export async function cloudFetchRaw(path: string, init: CloudFetchInit = {}): Promise<Response> {
  const headers = await getCloudRequestHeaders(init);
  const method = getMethod(init);
  const response = await fetchCloudResponse(path, init, headers);

  if (response.ok) {
    invalidateCloudFetchCacheForMutation(path, method);
    return response;
  }

  const payload = await parseJsonSafely(response);
  throw toCloudApiError(response, payload);
}

async function fetchCloudJson(path: string, init: CloudFetchInit, headers: Headers): Promise<unknown> {
  const method = getMethod(init);
  const response = await fetchCloudResponse(path, init, headers);
  const payload = await parseJsonSafely(response);
  if (!response.ok) {
    throw toCloudApiError(response, payload);
  }
  invalidateCloudFetchCacheForMutation(path, method);
  return payload;
}

export async function cloudFetch<T>(path: string, init: CloudFetchInit = {}): Promise<T> {
  const headers = await getCloudRequestHeaders(init);
  const method = getMethod(init);
  const key = cacheKeyFor(path, method, headers);
  const useCache = shouldUseJsonCache(path, method, init);
  const ttlMs = getCacheTtlMs(path, method, init);

  if (useCache) {
    const cached = cloudJsonCache.get(key);
    if (cached && cached.expiresAt > Date.now()) {
      logCloudRequest({
        path,
        method,
        durationMs: 0,
        cacheStatus: 'cache-hit',
      });
      return cached.value as T;
    }
    if (cached) cloudJsonCache.delete(key);

    const inFlight = cloudJsonInFlight.get(key);
    if (inFlight && init.dedupe !== false) {
      logCloudRequest({
        path,
        method,
        durationMs: 0,
        cacheStatus: 'dedupe-hit',
      });
      return inFlight as Promise<T>;
    }
  }

  const request = fetchCloudJson(path, init, headers).then((payload) => {
    if (useCache && ttlMs > 0) {
      cloudJsonCache.set(key, {
        value: payload,
        expiresAt: Date.now() + ttlMs,
      });
    }
    return payload;
  });

  if (useCache && init.dedupe !== false) {
    cloudJsonInFlight.set(key, request);
    const cleanup = () => {
      if (cloudJsonInFlight.get(key) === request) cloudJsonInFlight.delete(key);
    };
    void request.then(cleanup, cleanup);
  }

  return request as Promise<T>;
}
