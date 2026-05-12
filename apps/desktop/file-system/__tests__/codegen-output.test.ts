import { mkdir, readFile, rm } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { mkdtemp } from 'node:fs/promises';
import { describe, expect, it } from 'vitest';

import {
  resolveOutputFilePath,
  validateRelativeOutputPath,
  writeCodegenFilesToDirectory,
} from '../codegen-output';

async function mkTempDir(prefix = 'op-codegen-output-test-'): Promise<{
  dir: string;
  dispose: () => Promise<void>;
}> {
  const dir = await mkdtemp(join(tmpdir(), prefix));
  return {
    dir,
    dispose: async () => {
      await rm(dir, { recursive: true, force: true });
    },
  };
}

describe('codegen-output', () => {
  it('writes nested generated files into the selected directory', async () => {
    const { dir, dispose } = await mkTempDir();
    try {
      const result = await writeCodegenFilesToDirectory({
        rootDir: dir,
        files: [
          { path: 'src/App.tsx', content: 'export function App() { return null; }' },
          { path: 'styles/app.css', content: '.app { color: red; }' },
        ],
      });

      expect(result.rootDir).toBe(dir);
      expect(result.writtenFiles).toHaveLength(2);
      expect(result.writtenFiles[0]).toMatchObject({
        path: 'src/App.tsx',
        bytes: Buffer.byteLength('export function App() { return null; }', 'utf-8'),
      });
      await expect(readFile(join(dir, 'src', 'App.tsx'), 'utf-8')).resolves.toContain(
        'function App',
      );
      await expect(readFile(join(dir, 'styles', 'app.css'), 'utf-8')).resolves.toContain(
        'color: red',
      );
    } finally {
      await dispose();
    }
  });

  it('normalizes windows separators before writing files', async () => {
    const { dir, dispose } = await mkTempDir();
    try {
      const result = await writeCodegenFilesToDirectory({
        rootDir: dir,
        files: [{ path: 'pages\\index\\index.vue', content: '<template />' }],
      });

      expect(result.writtenFiles[0]?.path).toBe('pages/index/index.vue');
      await expect(readFile(join(dir, 'pages', 'index', 'index.vue'), 'utf-8')).resolves.toBe(
        '<template />',
      );
    } finally {
      await dispose();
    }
  });

  it.each(['../evil.ts', '/abs.ts', 'pages/../../evil.ts', 'src/\0evil.ts'])(
    'rejects unsafe output path %s',
    async (path) => {
      const { dir, dispose } = await mkTempDir();
      try {
        expect(validateRelativeOutputPath(path)).toBeNull();
        await expect(
          writeCodegenFilesToDirectory({
            rootDir: dir,
            files: [{ path, content: 'x' }],
          }),
        ).rejects.toThrow(/Invalid output file path/);
      } finally {
        await dispose();
      }
    },
  );

  it('refuses resolved paths outside the selected directory', async () => {
    const { dir, dispose } = await mkTempDir();
    try {
      await mkdir(join(dir, 'nested'), { recursive: true });
      expect(() => resolveOutputFilePath(join(dir, 'nested'), '../escape.ts')).toThrow(
        /Invalid output file path/,
      );
    } finally {
      await dispose();
    }
  });
});
