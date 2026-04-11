import { describe, expect, it } from 'vitest';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';

import { resolveServableLocalImagePath } from './local-asset';

describe('resolveServableLocalImagePath', () => {
  it('resolves an extensionless png file by sniffing its bytes', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'openpencil-local-asset-'));
    try {
      const filePath = join(dir, 'hero');
      writeFileSync(
        filePath,
        Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d]),
      );

      await expect(resolveServableLocalImagePath(filePath)).resolves.toEqual({
        resolvedPath: filePath,
        mimeType: 'image/png',
      });
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  it('resolves a sibling image file when the requested path is missing an extension', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'openpencil-local-asset-'));
    try {
      const filePath = join(dir, 'hero.jpg');
      writeFileSync(filePath, Buffer.from([0xff, 0xd8, 0xff, 0xe0]));

      await expect(resolveServableLocalImagePath(join(dir, 'hero'))).resolves.toEqual({
        resolvedPath: filePath,
        mimeType: 'image/jpeg',
      });
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  it('returns null for explicit unsupported extensions', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'openpencil-local-asset-'));
    try {
      const filePath = join(dir, 'hero.pdf');
      writeFileSync(filePath, Buffer.from('%PDF-1.7'));

      await expect(resolveServableLocalImagePath(filePath)).resolves.toBeNull();
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});
