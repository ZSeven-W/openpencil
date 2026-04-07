// apps/desktop/git/auth-store.ts
//
// Encrypted credential store backed by Electron safeStorage. The whole
// credential map is encrypted as a single blob and persisted to disk on
// every mutation. We don't shard per-host because the map is tiny (a
// handful of hosts at most) and atomic single-file writes are simpler.
//
// Tests inject a fake EncryptionBackend so they don't need a live Electron
// process.

import { promises as fsp } from 'node:fs';
import { join } from 'node:path';

export type AuthCreds =
  | { kind: 'token'; username: string; token: string }
  | { kind: 'ssh'; keyId: string };

export interface EncryptionBackend {
  isAvailable(): boolean;
  encrypt(plain: string): Buffer | string;
  decrypt(cipher: Buffer | string): string;
}

export interface AuthStore {
  set(host: string, creds: AuthCreds): Promise<void>;
  get(host: string): Promise<AuthCreds | null>;
  clear(host: string): Promise<void>;
  list(): Promise<string[]>;
}

interface AuthStoreOpts {
  filePath: string;
  backend: EncryptionBackend;
}

const PLAINTEXT_HEADER = '__OPENPENCIL_AUTH_PLAINTEXT_V1__';

/**
 * Build an AuthStore around a file path and an encryption backend. The
 * default factory at the bottom of this file uses Electron's safeStorage;
 * tests use createInMemoryBackend() instead.
 */
export function createAuthStore(opts: AuthStoreOpts): AuthStore {
  const { filePath, backend } = opts;
  let cache: Map<string, AuthCreds> | null = null;
  let warnedNoEncryption = false;
  // Set when we detect an encrypted blob on disk but the encryption backend
  // is unavailable. While locked, all reads return empty AND all writes throw
  // — we refuse to overwrite the encrypted file with plaintext (which would
  // destroy the user's stored credentials).
  let lockedOut = false;

  async function load(): Promise<Map<string, AuthCreds>> {
    if (cache) return cache;
    try {
      const bytes = await fsp.readFile(filePath);
      let json: string;
      // Plaintext file (from a previous run without encryption available)?
      // Detect via the header marker.
      const head = bytes
        .slice(0, Math.min(PLAINTEXT_HEADER.length, bytes.length))
        .toString('utf-8');
      if (head === PLAINTEXT_HEADER) {
        json = bytes.slice(PLAINTEXT_HEADER.length).toString('utf-8');
      } else if (backend.isAvailable()) {
        json = backend.decrypt(bytes);
      } else {
        // Encrypted blob exists but no key. Lock the store: subsequent
        // writes will throw rather than silently destroying the encrypted
        // file by overwriting it with plaintext.
        if (!warnedNoEncryption) {
          console.warn(
            '[git/auth-store] Encrypted credential file exists but safeStorage is unavailable. ' +
              'Refusing to read or modify until encryption is restored (e.g. install libsecret on Linux).',
          );
          warnedNoEncryption = true;
        }
        lockedOut = true;
        cache = new Map();
        return cache;
      }
      const obj = JSON.parse(json) as Record<string, AuthCreds>;
      cache = new Map(Object.entries(obj));
    } catch (err) {
      if ((err as NodeJS.ErrnoException).code === 'ENOENT') {
        cache = new Map();
      } else {
        throw err;
      }
    }
    return cache;
  }

  async function save(map: Map<string, AuthCreds>): Promise<void> {
    if (lockedOut) {
      throw new Error(
        'auth-store is locked: encrypted credential file exists but safeStorage is unavailable. ' +
          'Restore the encryption backend before modifying credentials to avoid data loss.',
      );
    }

    const obj: Record<string, AuthCreds> = {};
    for (const [host, creds] of map) obj[host] = creds;
    const json = JSON.stringify(obj);

    if (backend.isAvailable()) {
      const encrypted = backend.encrypt(json);
      const buf = Buffer.isBuffer(encrypted) ? encrypted : Buffer.from(encrypted);
      await fsp.writeFile(filePath, buf, { mode: 0o600 });
    } else {
      if (!warnedNoEncryption) {
        console.warn(
          '[git/auth-store] safeStorage unavailable; persisting credentials in plaintext (file mode 0600). Install libsecret for encryption.',
        );
        warnedNoEncryption = true;
      }
      await fsp.writeFile(filePath, PLAINTEXT_HEADER + json, { mode: 0o600 });
    }
  }

  return {
    async set(host, creds) {
      const map = await load();
      map.set(host, creds);
      await save(map);
    },
    async get(host) {
      const map = await load();
      return map.get(host) ?? null;
    },
    async clear(host) {
      const map = await load();
      map.delete(host);
      await save(map);
    },
    async list() {
      const map = await load();
      return [...map.keys()];
    },
  };
}

/**
 * In-memory backend used by tests. Encrypt/decrypt are no-ops that wrap
 * the input in a marker so we can verify the round-trip happened.
 */
export function createInMemoryBackend(): EncryptionBackend {
  return {
    isAvailable: () => true,
    encrypt: (plain) => Buffer.from('MEMENC:' + plain, 'utf-8'),
    decrypt: (cipher) => {
      const s = Buffer.isBuffer(cipher) ? cipher.toString('utf-8') : cipher;
      if (!s.startsWith('MEMENC:')) throw new Error('not memenc');
      return s.slice('MEMENC:'.length);
    },
  };
}

/**
 * Test-only helper: build an unavailable backend that always returns false
 * for isAvailable() so tests can exercise the plaintext fallback.
 */
export function createUnavailableBackend(): EncryptionBackend {
  return {
    isAvailable: () => false,
    encrypt: () => {
      throw new Error('not available');
    },
    decrypt: () => {
      throw new Error('not available');
    },
  };
}

/**
 * Default factory: build an AuthStore that uses the real Electron safeStorage
 * and the standard userData git-auth.bin location. Imported by ipc-handlers.ts.
 *
 * NOTE: This factory must NOT be called at module load time — Electron's
 * safeStorage is only available after `app.whenReady()`. ipc-handlers.ts
 * calls this lazily inside setupGitIPC().
 */
export function createDefaultAuthStore(): AuthStore {
  // Lazy require so tests don't pull in Electron.
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const electron = require('electron');
  const userDataDir: string = electron.app.getPath('userData');
  const filePath = join(userDataDir, 'git-auth.bin');
  const backend: EncryptionBackend = {
    isAvailable: () => electron.safeStorage.isEncryptionAvailable(),
    encrypt: (plain) => electron.safeStorage.encryptString(plain),
    decrypt: (cipher) => {
      const buf = Buffer.isBuffer(cipher) ? cipher : Buffer.from(cipher);
      return electron.safeStorage.decryptString(buf);
    },
  };
  return createAuthStore({ filePath, backend });
}
