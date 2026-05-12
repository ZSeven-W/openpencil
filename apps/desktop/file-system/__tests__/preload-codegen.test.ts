import { beforeEach, describe, expect, it, vi } from 'vitest';

const exposeInMainWorld = vi.fn();
const invoke = vi.fn();

vi.mock('electron', () => ({
  clipboard: { writeText: vi.fn() },
  contextBridge: { exposeInMainWorld },
  ipcRenderer: {
    invoke,
    on: vi.fn(),
    removeListener: vi.fn(),
    send: vi.fn(),
  },
  webUtils: { getPathForFile: vi.fn() },
}));

describe('preload codegen bridge', () => {
  beforeEach(() => {
    exposeInMainWorld.mockClear();
    invoke.mockClear();
    vi.resetModules();
  });

  it('exposes the desktop codegen IPC surface', async () => {
    await import('../../preload');

    expect(exposeInMainWorld).toHaveBeenCalledWith(
      'electronAPI',
      expect.objectContaining({
        codegen: expect.objectContaining({
          selectOutputDirectory: expect.any(Function),
          writeFiles: expect.any(Function),
          revealPath: expect.any(Function),
          gitStatus: expect.any(Function),
          gitCommit: expect.any(Function),
          gitPush: expect.any(Function),
        }),
      }),
    );
  });

  it('forwards codegen calls to named IPC channels', async () => {
    await import('../../preload');
    const api = exposeInMainWorld.mock.calls[0]?.[1] as ElectronAPI;

    await api.codegen.selectOutputDirectory();
    await api.codegen.writeFiles({ rootDir: '/tmp/out', files: [{ path: 'a.ts', content: 'x' }] });
    await api.codegen.revealPath('/tmp/out');
    await api.codegen.gitStatus({ rootDir: '/tmp/out', files: [] });
    await api.codegen.gitCommit({
      rootDir: '/tmp/out',
      files: [],
      message: 'add generated code',
      author: { name: 'OpenPencil', email: 'op@example.test' },
    });
    await api.codegen.gitPush({ rootDir: '/tmp/out' });

    expect(invoke.mock.calls.map((call) => call[0])).toEqual([
      'codegen:selectOutputDirectory',
      'codegen:writeFiles',
      'codegen:revealPath',
      'codegen:gitStatus',
      'codegen:gitCommit',
      'codegen:gitPush',
    ]);
  });
});
