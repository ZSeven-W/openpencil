import { beforeEach, describe, expect, it, vi } from 'vitest';

const authStoreMock = vi.hoisted(() => ({
  getAccessToken: vi.fn(),
}));

vi.mock('@/stores/cloud-auth-store', () => ({
  useCloudAuthStore: {
    getState: () => authStoreMock,
  },
}));

import { CloudApiError, cloudFetchRaw } from '../cloud-fetch';

describe('cloudFetchRaw', () => {
  beforeEach(() => {
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
});
