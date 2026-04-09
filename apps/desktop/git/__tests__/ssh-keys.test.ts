// apps/desktop/git/__tests__/ssh-keys.test.ts
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { promises as fsp } from 'node:fs';
import { join } from 'node:path';
import { createSshKeyManager, type SshKeyManager } from '../ssh-keys';
import { mkTempDir } from './test-helpers';

describe('ssh-keys', () => {
  let temp: { dir: string; dispose: () => Promise<void> };
  let manager: SshKeyManager;

  beforeEach(async () => {
    temp = await mkTempDir();
    manager = createSshKeyManager({ sshDir: join(temp.dir, 'ssh') });
  });

  afterEach(async () => {
    await temp.dispose();
  });

  it('generate produces a valid OpenSSH ed25519 public key with a SHA256 fingerprint', async () => {
    const info = await manager.generate({ host: 'github.com', comment: 'kay@laptop' });
    expect(info.publicKey.startsWith('ssh-ed25519 ')).toBe(true);
    expect(info.publicKey.endsWith('kay@laptop')).toBe(true);
    expect(info.fingerprint.startsWith('SHA256:')).toBe(true);
    expect(info.privateKeyPath).toMatch(/\.pem$/);

    // The private key file exists with mode 0600.
    const stat = await fsp.stat(info.privateKeyPath);
    // mode bits include the file type, mask for permissions only.
    expect(stat.mode & 0o777).toBe(0o600);
  });

  it('list returns all generated keys', async () => {
    const a = await manager.generate({ host: 'github.com', comment: 'a' });
    const b = await manager.generate({ host: 'gitlab.com', comment: 'b' });
    const all = await manager.list();
    const ids = all.map((k) => k.id).sort();
    expect(ids).toEqual([a.id, b.id].sort());
  });

  it('delete removes the key from the index and unlinks the private file', async () => {
    const info = await manager.generate({ host: 'github.com', comment: 'k' });
    await manager.delete(info.id);
    const all = await manager.list();
    expect(all).toHaveLength(0);
    await expect(fsp.access(info.privateKeyPath)).rejects.toThrow();
  });

  it('delete is idempotent: deleting an unknown key is a no-op', async () => {
    await expect(manager.delete('not-a-real-id')).resolves.toBeUndefined();
  });

  it('getPrivateKeyPath returns the absolute path for an existing key and throws for unknown', async () => {
    const info = await manager.generate({ host: 'github.com', comment: 'k' });
    expect(await manager.getPrivateKeyPath(info.id)).toBe(info.privateKeyPath);
    await expect(manager.getPrivateKeyPath('not-real')).rejects.toThrow(/SSH key/);
  });

  it('import takes an existing PEM private key file and stores a copy', async () => {
    // First generate one to get a real PEM file we can use as the "external" source.
    const seeded = await manager.generate({ host: 'github.com', comment: 'seed' });
    // Now create a fresh manager (different sshDir) and import the seeded private key.
    const otherDir = join(temp.dir, 'other-ssh');
    const other = createSshKeyManager({ sshDir: otherDir });
    const imported = await other.import({
      privateKeyPath: seeded.privateKeyPath,
      host: 'gitlab.com',
    });
    expect(imported.publicKey.startsWith('ssh-ed25519 ')).toBe(true);
    expect(imported.fingerprint).toBe(seeded.fingerprint); // same key → same fingerprint
    expect(imported.privateKeyPath).not.toBe(seeded.privateKeyPath); // copied to new location
    expect(imported.privateKeyPath.startsWith(otherDir)).toBe(true);
  });
});
