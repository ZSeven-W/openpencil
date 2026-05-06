// apps/desktop/git/auth-store.ts
//
// Encrypted 凭证存储由 Electron safeStorage 支持。 The 整体
// 凭证映射被加密为单个 blob 并持久保存到磁盘上
// 每一次突变。 We 不会对每个主机进行分片，因为地图很小（
// 最多几个主机）并且原子单文件写入更简单。
//
// Tests 注入一个假的 EncryptionBackend 所以他们不需要实时的 Electron
// 过程。

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
 * Build 和 AuthStore 围绕文件路径和加密后端。 The
 * default factory at the bottom of this file uses Electron's safeStorage;
 * 测试使用 createInMemoryBackend() 代替。
 */
export function createAuthStore(opts: AuthStoreOpts): AuthStore {
  const { filePath, backend } = opts;
  let cache: Map<string, AuthCreds> | null = null;
  let warnedNoEncryption = false;
  // Set 当我们在磁盘上检测到加密的 blob 但加密后端不可用时。 While 已锁定，所有读取都返回空 AND 所有写入都会抛出异常 -
  // 我们拒绝用明文覆盖加密文件（这会破坏用户存储的凭据）。
  let lockedOut = false;

  async function load(): Promise<Map<string, AuthCreds>> {
    if (cache) return cache;
    try {
      const bytes = await fsp.readFile(filePath);
      let json: string;
      // Plaintext 文件（来自先前运行且没有加密可用）？ Detect 通过标题标记。
      const head = bytes
        .slice(0, Math.min(PLAINTEXT_HEADER.length, bytes.length))
        .toString('utf-8');
      if (head === PLAINTEXT_HEADER) {
        json = bytes.slice(PLAINTEXT_HEADER.length).toString('utf-8');
      } else if (backend.isAvailable()) {
        json = backend.decrypt(bytes);
      } else {
        // Encrypted blob 存在，但没有密钥。 Lock 存储：后续写入将抛出异常，而不是通过用明文覆盖加密文件来默默
        // 地销毁它。
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
 * In-测试使用的内存后端
 * 。 Encrypt/decrypt 是无操作，将输入包装在标记中，以便我们可以验证是否发生了往返。
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
 * Test-only helper：构建一个始终返回 false 的不可用后端
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
 * Default
 * 工厂：构建一个 AuthStore，它使用真正的 Electron safeStorage 和标准 userData git-auth.bin 位置。
 *
 * Imported 作者：ipc-handlers.ts。 NOTE：This 工厂必须在模块加载时调用 NOT — Electron 的
 * safeStorage
 * 仅在 `app.whenReady()` 之后可用。 ipc-handlers.ts 在 setupGitIPC() 中懒惰地调用它。
 */
export function createDefaultAuthStore(): AuthStore {
  // Lazy 需要，因此测试不会引入 Electron。
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
