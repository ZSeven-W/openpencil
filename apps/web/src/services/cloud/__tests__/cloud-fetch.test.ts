import { beforeEach, describe, expect, it, vi } from 'vitest';

const authStoreMock = vi.hoisted(() => ({
  getAccessToken: vi.fn(),
}));

vi.mock('@/stores/cloud-auth-store', () => ({
  useCloudAuthStore: {
    getState: () => authStoreMock,
  },
}));

import { CloudApiError, clearCloudFetchCache, cloudFetch, cloudFetchRaw } from '../cloud-fetch';

describe('cloudFetchRaw', () => {
  beforeEach(() => {
    clearCloudFetchCache();
    authStoreMock.getAccessToken.mockReset();
    vi.stubGlobal('fetch', vi.fn(async () => new Response('{}', { status: 200 })));
  });

  it('sends the restored Supabase bearer token to cloud APIs', async () => {
    authStoreMock.getAccessToken.mockResolvedValue('desktop-session-token');

    await cloudFetchRaw('/api/cloud/files', {
      method: 'POST',
      body: JSON.stringify({ name: 'Cloud file' }),
    });

    const [, request] = (fetch as unknown as ReturnType<typeof vi.fn>).mock.calls[0] ?? [];
    const headers = (request as RequestInit).headers as Headers;
    expect(headers.get('Authorization')).toBe('Bearer desktop-session-token');
    expect(headers.get('Content-Type')).toBe('application/json');
  });

  it('throws unauthorized before fetch when no session is available', async () => {
    authStoreMock.getAccessToken.mockResolvedValue(null);

    await expect(cloudFetchRaw('/api/cloud/files')).rejects.toMatchObject({
      name: 'CloudApiError',
      status: 401,
      code: 'unauthorized',
    } satisfies Partial<CloudApiError>);
    expect(fetch).not.toHaveBeenCalled();
  });

  it('deduplicates identical in-flight GET requests', async () => {
    authStoreMock.getAccessToken.mockResolvedValue('desktop-session-token');
    let resolveFetch!: (response: Response) => void;
    vi.stubGlobal(
      'fetch',
      vi.fn(
        () =>
          new Promise<Response>((resolve) => {
            resolveFetch = resolve;
          }),
      ),
    );

    const first = cloudFetch<{ data: string[] }>('/api/cloud/files');
    const second = cloudFetch<{ data: string[] }>('/api/cloud/files');
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(fetch).toHaveBeenCalledTimes(1);
    resolveFetch(new Response(JSON.stringify({ data: ['file-1'] }), { status: 200 }));
    await expect(Promise.all([first, second])).resolves.toEqual([
      { data: ['file-1'] },
      { data: ['file-1'] },
    ]);
  });

  it('serves GET requests from a short TTL cache', async () => {
    authStoreMock.getAccessToken.mockResolvedValue('desktop-session-token');
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response(JSON.stringify({ data: ['file-1'] }), { status: 200 })),
    );

    await cloudFetch('/api/cloud/files', { cacheTtlMs: 1_000 });
    await cloudFetch('/api/cloud/files', { cacheTtlMs: 1_000 });

    expect(fetch).toHaveBeenCalledTimes(1);
  });

  it('allows forced GET refreshes to bypass the cache', async () => {
    authStoreMock.getAccessToken.mockResolvedValue('desktop-session-token');
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response(JSON.stringify({ data: ['file-1'] }), { status: 200 })),
    );

    await cloudFetch('/api/cloud/files', { cacheTtlMs: 1_000 });
    await cloudFetch('/api/cloud/files', { cacheTtlMs: 1_000, force: true });

    expect(fetch).toHaveBeenCalledTimes(2);
  });

  it('invalidates cached cloud GET responses after successful mutations', async () => {
    authStoreMock.getAccessToken.mockResolvedValue('desktop-session-token');
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response(JSON.stringify({ data: ['file-1'] }), { status: 200 })),
    );

    await cloudFetch('/api/cloud/files', { cacheTtlMs: 1_000 });
    await cloudFetch('/api/cloud/files', {
      method: 'POST',
      body: JSON.stringify({ name: 'New file' }),
    });
    await cloudFetch('/api/cloud/files', { cacheTtlMs: 1_000 });

    expect(fetch).toHaveBeenCalledTimes(3);
  });

  it('does not cache failed GET responses', async () => {
    authStoreMock.getAccessToken.mockResolvedValue('desktop-session-token');
    vi.stubGlobal(
      'fetch',
      vi
        .fn()
        .mockResolvedValueOnce(
          new Response(JSON.stringify({ error: { code: 'boom', message: 'Boom' } }), {
            status: 500,
          }),
        )
        .mockResolvedValueOnce(
          new Response(JSON.stringify({ data: ['file-1'] }), { status: 200 }),
        ),
    );

    await expect(cloudFetch('/api/cloud/files', { cacheTtlMs: 1_000 })).rejects.toMatchObject({
      code: 'boom',
    });
    await expect(cloudFetch('/api/cloud/files', { cacheTtlMs: 1_000 })).resolves.toEqual({
      data: ['file-1'],
    });

    expect(fetch).toHaveBeenCalledTimes(2);
  });

  it('keeps GET caches isolated by access token', async () => {
    authStoreMock.getAccessToken
      .mockResolvedValueOnce('desktop-session-token-a')
      .mockResolvedValueOnce('desktop-session-token-b');
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response(JSON.stringify({ data: ['file-1'] }), { status: 200 })),
    );

    await cloudFetch('/api/cloud/files', { cacheTtlMs: 1_000 });
    await cloudFetch('/api/cloud/files', { cacheTtlMs: 1_000 });

    expect(fetch).toHaveBeenCalledTimes(2);
  });
});
