import { promises as fsp } from 'node:fs';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { mkTempDir } from '../../git/__tests__/test-helpers';
import {
  createCloudSessionStore,
  createInMemoryCloudSessionBackend,
  createUnavailableCloudSessionBackend,
  type CloudSessionStore,
} from '../session-store';

describe('cloud session store', () => {
  let temp: { dir: string; dispose: () => Promise<void> };
  let filePath: string;
  let store: CloudSessionStore;

  beforeEach(async () => {
    temp = await mkTempDir('op-cloud-session-test-');
    filePath = join(temp.dir, 'cloud-session.bin');
    store = createCloudSessionStore({
      filePath,
      backend: createInMemoryCloudSessionBackend(),
    });
  });

  afterEach(async () => {
    await temp.dispose();
  });

  it('round-trips Supabase auth storage values by key', async () => {
    const value = JSON.stringify({
      access_token: 'access-token',
      refresh_token: 'refresh-token',
      expires_at: 123,
    });

    await store.setItem('sb-openpencil-auth-token', value);

    await expect(store.getItem('sb-openpencil-auth-token')).resolves.toBe(value);
    await expect(store.getItem('missing-key')).resolves.toBeNull();
  });

  it('persists values across store instances backed by the same file', async () => {
    await store.setItem('auth-key', '{"access_token":"a"}');
    const next = createCloudSessionStore({
      filePath,
      backend: createInMemoryCloudSessionBackend(),
    });

    await expect(next.getItem('auth-key')).resolves.toBe('{"access_token":"a"}');
  });

  it('removes keys without affecting other session values', async () => {
    await store.setItem('auth-key', 'auth');
    await store.setItem('auth-key-code-verifier', 'verifier');

    await store.removeItem('auth-key');

    await expect(store.getItem('auth-key')).resolves.toBeNull();
    await expect(store.getItem('auth-key-code-verifier')).resolves.toBe('verifier');
  });

  it('writes plaintext with a marker only when encryption is unavailable from the start', async () => {
    const plaintextStore = createCloudSessionStore({
      filePath,
      backend: createUnavailableCloudSessionBackend(),
    });

    await plaintextStore.setItem('auth-key', 'auth');

    const raw = await fsp.readFile(filePath, 'utf-8');
    expect(raw.startsWith('__OPENPENCIL_CLOUD_SESSION_PLAINTEXT_V1__')).toBe(true);
    expect(raw).toContain('"auth-key":"auth"');
  });

  it('locks writes when an encrypted file exists but safeStorage is unavailable', async () => {
    await store.setItem('auth-key', 'precious-session');
    const locked = createCloudSessionStore({
      filePath,
      backend: createUnavailableCloudSessionBackend(),
    });

    await expect(locked.getItem('auth-key')).resolves.toBeNull();
    await expect(locked.setItem('new-key', 'new-session')).rejects.toThrow(/locked/);
    await expect(locked.removeItem('auth-key')).rejects.toThrow(/locked/);

    const recovered = createCloudSessionStore({
      filePath,
      backend: createInMemoryCloudSessionBackend(),
    });
    await expect(recovered.getItem('auth-key')).resolves.toBe('precious-session');
  });
});
