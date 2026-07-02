// apps/web/src/services/__tests__/git-client.test.ts
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { gitClient, isGitApiAvailable } from '@/services/git-client';
import { GitError, GIT_ERROR_MARKER } from '@/services/git-error';

// A minimal stub of window.electronAPI.git — we only implement the methods
// the tests exercise.
function makeFakeApi(overrides: Record<string, unknown> = {}) {
  return {
    isElectron: true as const,
    git: {
      detect: vi.fn(),
      init: vi.fn(),
      status: vi.fn(),
      commit: vi.fn(),
      ...overrides,
    },
  };
}

describe('git-client', () => {
  afterEach(() => {
    // Clean up the stub so tests don't leak.
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (globalThis as any).window = undefined;
  });

  describe('isGitApiAvailable', () => {
    it('returns false when window.electronAPI is undefined', () => {
      vi.stubGlobal('window', {});
      expect(isGitApiAvailable()).toBe(false);
      vi.unstubAllGlobals();
    });

    it('returns true when window.electronAPI.git is present', () => {
      vi.stubGlobal('window', { electronAPI: makeFakeApi() });
      expect(isGitApiAvailable()).toBe(true);
      vi.unstubAllGlobals();
    });
  });

  describe('method passthrough', () => {
    beforeEach(() => {
      // Reset any stubs from previous tests.
      vi.unstubAllGlobals();
    });

    afterEach(() => {
      vi.unstubAllGlobals();
    });

    it('forwards a successful call to window.electronAPI.git', async () => {
      const detect = vi.fn().mockResolvedValue({ mode: 'none' });
      vi.stubGlobal('window', { electronAPI: makeFakeApi({ detect }) });
      const result = await gitClient.detect('/tmp/file.op');
      expect(detect).toHaveBeenCalledWith('/tmp/file.op');
      expect(result).toEqual({ mode: 'none' });
    });

    it('throws GitError(engine-crash) when window.electronAPI.git is missing', async () => {
      vi.stubGlobal('window', {});
      await expect(gitClient.detect('/tmp/file.op')).rejects.toMatchObject({
        name: 'GitError',
        code: 'engine-crash',
      });
    });

    it('rehydrates a marker Error thrown by the IPC layer into a GitError', async () => {
      const payload = { code: 'commit-empty', message: 'no changes', recoverable: true };
      const wire = new Error(`${GIT_ERROR_MARKER}${JSON.stringify(payload)}`);
      const commit = vi.fn().mockRejectedValue(wire);
      vi.stubGlobal('window', { electronAPI: makeFakeApi({ commit }) });

      await expect(
        gitClient.commit('repo-1', {
          kind: 'milestone',
          message: 'test',
          author: { name: 't', email: 't@e.com' },
        }),
      ).rejects.toBeInstanceOf(GitError);
      try {
        await gitClient.commit('repo-1', {
          kind: 'milestone',
          message: 'test',
          author: { name: 't', email: 't@e.com' },
        });
      } catch (err) {
        expect((err as GitError).code).toBe('commit-empty');
        expect((err as GitError).message).toBe('no changes');
      }
    });

    it('re-throws non-GitError exceptions unchanged', async () => {
      const weird = new TypeError('something else');
      const status = vi.fn().mockRejectedValue(weird);
      vi.stubGlobal('window', { electronAPI: makeFakeApi({ status }) });
      await expect(gitClient.status('repo-1')).rejects.toBe(weird);
    });
  });
});
