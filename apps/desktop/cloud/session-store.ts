import { promises as fsp } from 'node:fs';
import { join } from 'node:path';

export interface CloudSessionEncryptionBackend {
  isAvailable(): boolean;
  encrypt(plain: string): Buffer | string;
  decrypt(cipher: Buffer | string): string;
}

export interface CloudSessionStore {
  getItem(key: string): Promise<string | null>;
  setItem(key: string, value: string): Promise<void>;
  removeItem(key: string): Promise<void>;
}

interface CloudSessionStoreOptions {
  filePath: string;
  backend: CloudSessionEncryptionBackend;
}

const PLAINTEXT_HEADER = '__OPENPENCIL_CLOUD_SESSION_PLAINTEXT_V1__';

function validateStorageKey(key: string): void {
  if (!key || key.includes('\0')) {
    throw new Error('Invalid cloud auth storage key');
  }
}

export function createCloudSessionStore(opts: CloudSessionStoreOptions): CloudSessionStore {
  const { filePath, backend } = opts;
  let cache: Map<string, string> | null = null;
  let lockedOut = false;
  let warnedNoEncryption = false;

  async function load(): Promise<Map<string, string>> {
    if (cache) return cache;

    try {
      const bytes = await fsp.readFile(filePath);
      let json: string;
      const head = bytes
        .slice(0, Math.min(PLAINTEXT_HEADER.length, bytes.length))
        .toString('utf-8');

      if (head === PLAINTEXT_HEADER) {
        json = bytes.slice(PLAINTEXT_HEADER.length).toString('utf-8');
      } else if (backend.isAvailable()) {
        json = backend.decrypt(bytes);
      } else {
        if (!warnedNoEncryption) {
          console.warn(
            '[cloud/session-store] Encrypted session file exists but safeStorage is unavailable. Refusing to modify it.',
          );
          warnedNoEncryption = true;
        }
        lockedOut = true;
        cache = new Map();
        return cache;
      }

      const parsed = JSON.parse(json) as Record<string, string>;
      cache = new Map(Object.entries(parsed));
    } catch (err) {
      if ((err as NodeJS.ErrnoException).code === 'ENOENT') {
        cache = new Map();
      } else {
        throw err;
      }
    }

    return cache;
  }

  async function save(map: Map<string, string>): Promise<void> {
    if (lockedOut) {
      throw new Error(
        'cloud session store is locked: encrypted session file exists but safeStorage is unavailable.',
      );
    }

    const json = JSON.stringify(Object.fromEntries(map));
    if (backend.isAvailable()) {
      const encrypted = backend.encrypt(json);
      const bytes = Buffer.isBuffer(encrypted) ? encrypted : Buffer.from(encrypted);
      await fsp.writeFile(filePath, bytes, { mode: 0o600 });
      return;
    }

    if (!warnedNoEncryption) {
      console.warn(
        '[cloud/session-store] safeStorage unavailable; persisting cloud session in plaintext with file mode 0600.',
      );
      warnedNoEncryption = true;
    }
    await fsp.writeFile(filePath, PLAINTEXT_HEADER + json, { mode: 0o600 });
  }

  return {
    async getItem(key) {
      validateStorageKey(key);
      const map = await load();
      return map.get(key) ?? null;
    },
    async setItem(key, value) {
      validateStorageKey(key);
      const map = await load();
      map.set(key, value);
      await save(map);
    },
    async removeItem(key) {
      validateStorageKey(key);
      const map = await load();
      map.delete(key);
      await save(map);
    },
  };
}

export function createInMemoryCloudSessionBackend(): CloudSessionEncryptionBackend {
  return {
    isAvailable: () => true,
    encrypt: (plain) => Buffer.from('MEMENC:' + plain, 'utf-8'),
    decrypt: (cipher) => {
      const value = Buffer.isBuffer(cipher) ? cipher.toString('utf-8') : cipher;
      if (!value.startsWith('MEMENC:')) throw new Error('not memenc');
      return value.slice('MEMENC:'.length);
    },
  };
}

export function createUnavailableCloudSessionBackend(): CloudSessionEncryptionBackend {
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

export function createDefaultCloudSessionStore(): CloudSessionStore {
  // Lazy require so tests do not need a live Electron app.
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const electron = require('electron');
  const userDataDir: string = electron.app.getPath('userData');
  const filePath = join(userDataDir, 'cloud-session.bin');
  const backend: CloudSessionEncryptionBackend = {
    isAvailable: () => electron.safeStorage.isEncryptionAvailable(),
    encrypt: (plain) => electron.safeStorage.encryptString(plain),
    decrypt: (cipher) => {
      const bytes = Buffer.isBuffer(cipher) ? cipher : Buffer.from(cipher);
      return electron.safeStorage.decryptString(bytes);
    },
  };
  return createCloudSessionStore({ filePath, backend });
}
