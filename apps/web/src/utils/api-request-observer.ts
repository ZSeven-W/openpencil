type ApiRequestObserverOptions = {
  duplicateWindowMs?: number;
  enabled?: boolean;
  slowMs?: number;
};

type RecentRequest = {
  count: number;
  firstSeenAt: number;
  stack?: string;
};

const DEFAULT_DUPLICATE_WINDOW_MS = 1_500;
const DEFAULT_SLOW_REQUEST_MS = 800;
const INSTALL_MARK = '__openpencilApiRequestObserverInstalled';

function isDevRuntime(): boolean {
  return Boolean(import.meta.env?.DEV) && import.meta.env?.MODE !== 'test';
}

function numericEnv(name: string, fallback: number): number {
  const raw = import.meta.env?.[name];
  const parsed = raw ? Number(raw) : Number.NaN;
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}

function stableHash(value: string): string {
  let hash = 2166136261;
  for (let i = 0; i < value.length; i += 1) {
    hash ^= value.charCodeAt(i);
    hash = Math.imul(hash, 16777619);
  }
  return (hash >>> 0).toString(16);
}

function bodyKey(init?: RequestInit): string {
  if (typeof init?.body === 'string') return stableHash(init.body);
  if (init?.body instanceof URLSearchParams) return stableHash(init.body.toString());
  return '';
}

function requestMethod(input: RequestInfo | URL, init?: RequestInit): string {
  if (init?.method) return init.method.toUpperCase();
  if (typeof Request !== 'undefined' && input instanceof Request) return input.method.toUpperCase();
  return 'GET';
}

function requestUrl(input: RequestInfo | URL): URL | null {
  try {
    const value = typeof Request !== 'undefined' && input instanceof Request ? input.url : input;
    return new URL(String(value), window.location.href);
  } catch {
    return null;
  }
}

function isLocalApiUrl(url: URL): boolean {
  return url.origin === window.location.origin && (url.pathname === '/api' || url.pathname.startsWith('/api/'));
}

function callerStack(): string | undefined {
  const stack = new Error().stack;
  if (!stack) return undefined;
  return stack
    .split('\n')
    .slice(3, 7)
    .map((line) => line.trim())
    .join(' | ');
}

export function installApiRequestObserver(options: ApiRequestObserverOptions = {}) {
  if (typeof window === 'undefined' || typeof window.fetch !== 'function') return;
  const enabled = options.enabled ?? isDevRuntime();
  if (!enabled) return;

  const currentFetch = window.fetch as typeof window.fetch & {
    [INSTALL_MARK]?: boolean;
  };
  if (currentFetch[INSTALL_MARK]) return;

  const duplicateWindowMs =
    options.duplicateWindowMs ?? numericEnv('VITE_OPENPENCIL_API_DUPLICATE_MS', DEFAULT_DUPLICATE_WINDOW_MS);
  const slowMs =
    options.slowMs ?? numericEnv('VITE_OPENPENCIL_API_SLOW_MS', DEFAULT_SLOW_REQUEST_MS);
  const originalFetch = window.fetch.bind(window);
  const recentRequests = new Map<string, RecentRequest>();

  const observedFetch: typeof window.fetch & { [INSTALL_MARK]?: boolean } = async (input, init) => {
    const url = requestUrl(input);
    if (!url || !isLocalApiUrl(url)) return originalFetch(input, init);

    const method = requestMethod(input, init);
    const path = `${url.pathname}${url.search}`;
    const key = `${method} ${path} ${bodyKey(init)}`;
    const now = Date.now();
    const previous = recentRequests.get(key);
    const stack = callerStack();
    if (previous && now - previous.firstSeenAt <= duplicateWindowMs) {
      previous.count += 1;
      console.warn(
        `[apiFetch] duplicate method=${method} path=${path} count=${previous.count} windowMs=${duplicateWindowMs}`,
        { firstCaller: previous.stack, caller: stack },
      );
    } else {
      recentRequests.set(key, { count: 1, firstSeenAt: now, stack });
    }

    for (const [recentKey, value] of recentRequests) {
      if (now - value.firstSeenAt > duplicateWindowMs) recentRequests.delete(recentKey);
    }

    const startedAt = performance.now();
    try {
      const response = await originalFetch(input, init);
      const durationMs = Math.round(performance.now() - startedAt);
      const message = `[apiFetch] method=${method} status=${response.status} durationMs=${durationMs} path=${path}`;
      if (durationMs >= slowMs) console.warn(`[apiFetch] slow ${message}`, { caller: stack });
      else console.debug(message);
      return response;
    } catch (err) {
      const durationMs = Math.round(performance.now() - startedAt);
      console.warn(`[apiFetch] failed method=${method} durationMs=${durationMs} path=${path}`, {
        caller: stack,
        error: err,
      });
      throw err;
    }
  };

  observedFetch[INSTALL_MARK] = true;
  window.fetch = observedFetch;
}
