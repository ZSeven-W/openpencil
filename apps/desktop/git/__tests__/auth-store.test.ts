// apps/desktop/git/__tests__/auth-store.test.ts
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { promises as fsp } from 'node:fs';
import { join } from 'node:path';
import {
  createAuthStore,
  createInMemoryBackend,
  createUnavailableBackend,
  type AuthStore,
} from '../auth-store';
import { mkTempDir } from './test-helpers';

describe('auth-store (in-memory backend)', () => {
  let temp: { dir: string; dispose: () => Promise<void> };
  let store: AuthStore;
  let filePath: string;

  beforeEach(async () => {
    temp = await mkTempDir();
    filePath = join(temp.dir, 'git-auth.bin');
    store = createAuthStore({ filePath, backend: createInMemoryBackend() });
  });

  afterEach(async () => {
    await temp.dispose();
  });

  it('round-trips a token credential through set + get', async () => {
    await store.set('github.com', {
      kind: 'token',
      username: 'kay',
      token: 'ghp_abc123',
    });
    const got = await store.get('github.com');
    expect(got).toEqual({ kind: 'token', username: 'kay', token: 'ghp_abc123' });
  });

  it('round-trips an SSH credential', async () => {
    await store.set('git@github.com', { kind: 'ssh', keyId: 'key-1' });
    const got = await store.get('git@github.com');
    expect(got).toEqual({ kind: 'ssh', keyId: 'key-1' });
  });

  it('returns null for an unknown host', async () => {
    const got = await store.get('not-stored.example.com');
    expect(got).toBeNull();
  });

  it('list returns all stored hosts', async () => {
    await store.set('github.com', { kind: 'token', username: 'a', token: 't1' });
    await store.set('gitlab.com', { kind: 'token', username: 'b', token: 't2' });
    const hosts = (await store.list()).sort();
    expect(hosts).toEqual(['github.com', 'gitlab.com']);
  });

  it('clear removes one host without affecting others', async () => {
    await store.set('github.com', { kind: 'token', username: 'a', token: 't1' });
    await store.set('gitlab.com', { kind: 'token', username: 'b', token: 't2' });
    await store.clear('github.com');
    expect(await store.get('github.com')).toBeNull();
    expect(await store.get('gitlab.com')).not.toBeNull();
  });

  it('persists across new store instances backed by the same file', async () => {
    await store.set('github.com', { kind: 'token', username: 'kay', token: 't' });
    // New instance, same file + same backend type.
    const store2 = createAuthStore({ filePath, backend: createInMemoryBackend() });
    const got = await store2.get('github.com');
    expect(got?.kind).toBe('token');
  });
});

describe('auth-store (unavailable backend → plaintext fallback)', () => {
  let temp: { dir: string; dispose: () => Promise<void> };

  beforeEach(async () => {
    temp = await mkTempDir();
  });

  afterEach(async () => {
    await temp.dispose();
  });

  it('writes a plaintext file with the marker header when safeStorage is unavailable from the start', async () => {
    const filePath = join(temp.dir, 'git-auth.bin');
    const store = createAuthStore({ filePath, backend: createUnavailableBackend() });
    await store.set('github.com', { kind: 'token', username: 'kay', token: 't' });

    const bytes = await fsp.readFile(filePath, 'utf-8');
    expect(bytes.startsWith('__OPENPENCIL_AUTH_PLAINTEXT_V1__')).toBe(true);
    const body = bytes.slice('__OPENPENCIL_AUTH_PLAINTEXT_V1__'.length);
    const obj = JSON.parse(body);
    expect(obj['github.com'].kind).toBe('token');
  });

  it('locks the store when an existing encrypted file is read with no decryption key — refuses writes to avoid data loss', async () => {
    const filePath = join(temp.dir, 'git-auth.bin');
    // Step 1: write an encrypted file via the in-memory backend.
    const enc = createAuthStore({ filePath, backend: createInMemoryBackend() });
    await enc.set('github.com', { kind: 'token', username: 'kay', token: 'precious-pat' });
    await enc.set('gitlab.com', { kind: 'token', username: 'kay', token: 'also-precious' });

    // Step 2: open a fresh store pointing at the same file but with an
    // unavailable backend. The encrypted bytes are NOT plaintext-marked, so
    // the store should detect them and lock.
    const locked = createAuthStore({ filePath, backend: createUnavailableBackend() });

    // Reads return empty (locked) but do not throw.
    expect(await locked.get('github.com')).toBeNull();
    expect(await locked.list()).toEqual([]);

    // Writes throw the lock error.
    await expect(
      locked.set('newhost.com', { kind: 'token', username: 'a', token: 'b' }),
    ).rejects.toThrow(/locked/);
    await expect(locked.clear('github.com')).rejects.toThrow(/locked/);

    // Critical: the original encrypted file is unchanged.
    const bytesAfter = await fsp.readFile(filePath, 'utf-8');
    expect(bytesAfter.startsWith('__OPENPENCIL_AUTH_PLAINTEXT_V1__')).toBe(false);
    // And a new instance with the in-memory backend can still read both
    // original credentials.
    const recovered = createAuthStore({ filePath, backend: createInMemoryBackend() });
    expect((await recovered.get('github.com'))?.kind).toBe('token');
    expect((await recovered.get('gitlab.com'))?.kind).toBe('token');
  });
});
