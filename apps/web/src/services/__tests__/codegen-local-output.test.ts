import { afterEach, describe, expect, it, vi } from 'vitest';

import {
  commitCodegenOutput,
  getCodegenOutputGitStatus,
  isCodegenLocalOutputAvailable,
  pushCodegenOutput,
  revealCodegenOutputPath,
  selectCodegenOutputDirectory,
  writeCodegenOutputFiles,
} from '@/services/codegen-local-output';

function stubCodegenApi(overrides: Partial<CodegenAPI> = {}) {
  const codegen: CodegenAPI = {
    selectOutputDirectory: vi.fn(),
    writeFiles: vi.fn(),
    revealPath: vi.fn(),
    gitStatus: vi.fn(),
    gitCommit: vi.fn(),
    gitPush: vi.fn(),
    ...overrides,
  };
  vi.stubGlobal('window', {
    electronAPI: {
      isElectron: true,
      codegen,
    },
  });
  return codegen;
}

describe('codegen-local-output', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('reports unavailable outside Electron codegen bridge', () => {
    vi.stubGlobal('window', {});

    expect(isCodegenLocalOutputAvailable()).toBe(false);
  });

  it('selects an output directory through the Electron bridge', async () => {
    const selectOutputDirectory = vi.fn(async () => '/tmp/out');
    stubCodegenApi({ selectOutputDirectory });

    await expect(selectCodegenOutputDirectory()).resolves.toBe('/tmp/out');
    expect(selectOutputDirectory).toHaveBeenCalledTimes(1);
  });

  it('writes generated files through the Electron bridge', async () => {
    const writeFiles = vi.fn(async () => ({
      rootDir: '/tmp/out',
      writtenFiles: [{ path: 'src/App.tsx', absolutePath: '/tmp/out/src/App.tsx', bytes: 12 }],
    }));
    stubCodegenApi({ writeFiles });

    const files = [{ path: 'src/App.tsx', language: 'tsx', role: 'entry' as const, content: 'x' }];
    await expect(writeCodegenOutputFiles({ rootDir: '/tmp/out', files })).resolves.toMatchObject({
      writtenFiles: [{ path: 'src/App.tsx' }],
    });
    expect(writeFiles).toHaveBeenCalledWith({
      rootDir: '/tmp/out',
      files: [{ path: 'src/App.tsx', content: 'x' }],
    });
  });

  it('forwards reveal and git actions', async () => {
    const revealPath = vi.fn(async () => {});
    const gitStatus = vi.fn(async () => ({ mode: 'none' as const, rootDir: '/tmp/out' }));
    const gitCommit = vi.fn(async () => ({ hash: 'abc', changedFiles: ['src/App.tsx'] }));
    const gitPush = vi.fn(async () => ({ result: 'ok' as const }));
    stubCodegenApi({ revealPath, gitStatus, gitCommit, gitPush });
    const files = [{ path: 'src/App.tsx', language: 'tsx', role: 'entry' as const, content: 'x' }];

    await revealCodegenOutputPath('/tmp/out');
    await getCodegenOutputGitStatus({ rootDir: '/tmp/out', files });
    await commitCodegenOutput({
      rootDir: '/tmp/out',
      files,
      message: 'add generated code',
      author: { name: 'OpenPencil', email: 'op@example.test' },
    });
    await pushCodegenOutput('/tmp/out');

    expect(revealPath).toHaveBeenCalledWith('/tmp/out');
    expect(gitStatus).toHaveBeenCalledWith({
      rootDir: '/tmp/out',
      files: [{ path: 'src/App.tsx', content: 'x' }],
    });
    expect(gitCommit).toHaveBeenCalledWith({
      rootDir: '/tmp/out',
      files: [{ path: 'src/App.tsx', content: 'x' }],
      message: 'add generated code',
      author: { name: 'OpenPencil', email: 'op@example.test' },
    });
    expect(gitPush).toHaveBeenCalledWith({ rootDir: '/tmp/out' });
  });

  it('throws a clear error when called outside Electron', async () => {
    vi.stubGlobal('window', {});

    expect(() => selectCodegenOutputDirectory()).toThrow(/not available/i);
  });
});
