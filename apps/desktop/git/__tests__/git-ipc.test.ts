// apps/desktop/git/__tests__/git-ipc.test.ts
import { describe, it, expect } from 'vitest';
import { GitError } from '../error';
import { serializeGitError, GIT_ERROR_MARKER } from '../ipc-handlers';

describe('serializeGitError', () => {
  it('produces an Error whose message starts with the marker and round-trips fields', () => {
    const original = new GitError('commit-empty', 'No changes', { recoverable: true });
    const serialized = serializeGitError(original);
    expect(serialized.message.startsWith(GIT_ERROR_MARKER)).toBe(true);
    const payload = JSON.parse(serialized.message.slice(GIT_ERROR_MARKER.length));
    expect(payload.code).toBe('commit-empty');
    expect(payload.message).toBe('No changes');
    expect(payload.recoverable).toBe(true);
  });

  it('preserves recoverable=false from the GitError', () => {
    const original = new GitError('engine-crash', 'something blew up', { recoverable: false });
    const serialized = serializeGitError(original);
    const payload = JSON.parse(serialized.message.slice(GIT_ERROR_MARKER.length));
    expect(payload.recoverable).toBe(false);
  });

  it('the marker is a stable, unique string the renderer can pattern-match', () => {
    expect(GIT_ERROR_MARKER).toBe('__GIT_ERROR__');
  });

  it('serializes auth-related and network error codes', () => {
    const codes = [
      'auth-failed',
      'auth-required',
      'clone-failed',
      'network',
      'pull-non-fast-forward',
    ] as const;
    for (const code of codes) {
      const original = new GitError(code, `${code} test`);
      const serialized = serializeGitError(original);
      const payload = JSON.parse(serialized.message.slice(GIT_ERROR_MARKER.length));
      expect(payload.code).toBe(code);
    }
  });

  it('serializes merge-related error codes', () => {
    const codes = ['merge-conflict', 'merge-still-conflicted', 'merge-abort-failed'] as const;
    for (const code of codes) {
      const original = new GitError(code, `${code} test`);
      const serialized = serializeGitError(original);
      const payload = JSON.parse(serialized.message.slice(GIT_ERROR_MARKER.length));
      expect(payload.code).toBe(code);
    }
  });
});
