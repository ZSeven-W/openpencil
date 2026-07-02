// apps/desktop/git/ssh-keys.ts
//
// SSH key management. We generate ed25519 keypairs via node:crypto and
// format the public key as OpenSSH using sshpk. Private keys are stored
// as PEM PKCS#8 with file mode 0600. The metadata index lives at
// <sshDir>/index.json and is rewritten atomically on every mutation.
//
// This module is factory-based so tests can inject a temp directory.

import { promises as fsp } from 'node:fs';
import { generateKeyPair, randomUUID } from 'node:crypto';
import { promisify } from 'node:util';
import { join, basename } from 'node:path';
// eslint-disable-next-line @typescript-eslint/no-require-imports
const sshpk = require('sshpk') as typeof import('sshpk');

const generateKeyPairAsync = promisify(generateKeyPair);

export interface SshKeyInfo {
  id: string;
  host: string;
  publicKey: string; // OpenSSH single-line format
  fingerprint: string; // SHA256 fingerprint with `SHA256:` prefix
  comment: string;
  /** Absolute path to the PEM private key file. Not exposed via IPC — used
   * internally by git-sys when constructing GIT_SSH_COMMAND. */
  privateKeyPath: string;
}

export interface SshKeyManager {
  generate(opts: { host: string; comment: string }): Promise<SshKeyInfo>;
  import(opts: { privateKeyPath: string; host: string }): Promise<SshKeyInfo>;
  list(): Promise<SshKeyInfo[]>;
  delete(keyId: string): Promise<void>;
  /** Resolve a keyId to its private key path. Throws if missing. Used by
   * git-sys when invoking SSH transport. Not part of the IPC surface. */
  getPrivateKeyPath(keyId: string): Promise<string>;
}

export interface SshKeyManagerOpts {
  /** Directory where private keys + index.json live. Created on first use. */
  sshDir: string;
}

const INDEX_FILE = 'index.json';

export function createSshKeyManager(opts: SshKeyManagerOpts): SshKeyManager {
  const { sshDir } = opts;

  async function ensureDir(): Promise<void> {
    await fsp.mkdir(sshDir, { recursive: true, mode: 0o700 });
  }

  async function loadIndex(): Promise<SshKeyInfo[]> {
    try {
      const bytes = await fsp.readFile(join(sshDir, INDEX_FILE), 'utf-8');
      return JSON.parse(bytes) as SshKeyInfo[];
    } catch (err) {
      if ((err as NodeJS.ErrnoException).code === 'ENOENT') return [];
      throw err;
    }
  }

  async function saveIndex(keys: SshKeyInfo[]): Promise<void> {
    await ensureDir();
    const tmp = join(sshDir, `${INDEX_FILE}.tmp`);
    const dest = join(sshDir, INDEX_FILE);
    await fsp.writeFile(tmp, JSON.stringify(keys, null, 2), { mode: 0o600 });
    await fsp.rename(tmp, dest);
  }

  function computeFingerprint(opensshPublicKey: string): string {
    const key = sshpk.parseKey(opensshPublicKey, 'ssh');
    return key.fingerprint('sha256').toString();
  }

  function formatPublicKeyOpenSsh(pemPublic: string, comment: string): string {
    const key = sshpk.parseKey(pemPublic, 'pem');
    const ssh = key.toString('ssh');
    // sshpk's ssh format already includes "ssh-ed25519 ..."; append comment.
    return `${ssh.trim()} ${comment}`.trim();
  }

  return {
    async generate({ host, comment }) {
      await ensureDir();
      const { publicKey, privateKey } = await generateKeyPairAsync('ed25519', {
        publicKeyEncoding: { type: 'spki', format: 'pem' },
        privateKeyEncoding: { type: 'pkcs8', format: 'pem' },
      });

      const id = randomUUID();
      const privateKeyPath = join(sshDir, `${id}.pem`);
      await fsp.writeFile(privateKeyPath, privateKey, { mode: 0o600 });

      const sshPublic = formatPublicKeyOpenSsh(publicKey, comment);
      const fingerprint = computeFingerprint(sshPublic);

      const info: SshKeyInfo = {
        id,
        host,
        publicKey: sshPublic,
        fingerprint,
        comment,
        privateKeyPath,
      };
      const all = await loadIndex();
      all.push(info);
      await saveIndex(all);
      return info;
    },

    async import({ privateKeyPath, host }) {
      // Read the existing private key, derive the public key via sshpk.
      const pemBytes = await fsp.readFile(privateKeyPath);
      const privateKeyObj = sshpk.parsePrivateKey(pemBytes, 'auto');
      const publicKeyObj = privateKeyObj.toPublic();
      const sshPublic = publicKeyObj.toString('ssh').trim();
      const fingerprint = publicKeyObj.fingerprint('sha256').toString();
      const comment = `imported-${basename(privateKeyPath)}`;

      // Copy the file into our sshDir so the user's original location can
      // move freely without breaking us.
      const id = randomUUID();
      const destPath = join(sshDir, `${id}.pem`);
      await ensureDir();
      // Re-export as PKCS#8 PEM via sshpk so we always store a uniform format.
      const pkcs8Pem = privateKeyObj.toString('pkcs8');
      await fsp.writeFile(destPath, pkcs8Pem, { mode: 0o600 });

      const info: SshKeyInfo = {
        id,
        host,
        publicKey: `${sshPublic} ${comment}`,
        fingerprint,
        comment,
        privateKeyPath: destPath,
      };
      const all = await loadIndex();
      all.push(info);
      await saveIndex(all);
      return info;
    },

    async list() {
      return loadIndex();
    },

    async delete(keyId) {
      const all = await loadIndex();
      const idx = all.findIndex((k) => k.id === keyId);
      if (idx === -1) return; // already gone — idempotent
      const info = all[idx];
      try {
        await fsp.unlink(info.privateKeyPath);
      } catch (err) {
        if ((err as NodeJS.ErrnoException).code !== 'ENOENT') throw err;
      }
      all.splice(idx, 1);
      await saveIndex(all);
    },

    async getPrivateKeyPath(keyId) {
      const all = await loadIndex();
      const found = all.find((k) => k.id === keyId);
      if (!found) {
        throw new Error(`SSH key ${keyId} not found`);
      }
      return found.privateKeyPath;
    },
  };
}

/**
 * Default factory: builds a manager pointed at <userData>/ssh/. Lazy-imports
 * Electron so tests don't pull it in.
 */
export function createDefaultSshKeyManager(): SshKeyManager {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const electron = require('electron');
  const userDataDir: string = electron.app.getPath('userData');
  return createSshKeyManager({ sshDir: join(userDataDir, 'ssh') });
}
