// apps/web/src/services/__tests__/git-error.test.ts
import { describe, it, expect } from 'vitest';
import { GitError, isGitError, rehydrateGitError, GIT_ERROR_MARKER } from '@/services/git-error';

describe('GitError', () => {
  it('constructs with code + message + recoverable default=true', () => {
    const err = new GitError('commit-empty', 'nothing to commit');
    expect(err.code).toBe('commit-empty');
    expect(err.message).toBe('nothing to commit');
    expect(err.recoverable).toBe(true);
    expect(err.name).toBe('GitError');
    expect(err).toBeInstanceOf(Error);
  });

  it('respects explicit recoverable=false', () => {
    const err = new GitError('engine-crash', 'boom', { recoverable: false });
    expect(err.recoverable).toBe(false);
  });

  it('isGitError matches both class instances and duck-typed objects', () => {
    expect(isGitError(new GitError('network', 'down'))).toBe(true);
    expect(isGitError({ name: 'GitError', code: 'network' })).toBe(true);
    expect(isGitError(new Error('not a git error'))).toBe(false);
    expect(isGitError(null)).toBe(false);
    expect(isGitError('string')).toBe(false);
  });
});

describe('rehydrateGitError', () => {
  function encode(code: string, message: string, recoverable = true): Error {
    return new Error(`${GIT_ERROR_MARKER}${JSON.stringify({ code, message, recoverable })}`);
  }

  it('rehydrates a valid marker Error into a GitError', () => {
    const wire = encode('auth-failed', 'bad token', false);
    const rehydrated = rehydrateGitError(wire);
    expect(rehydrated).toBeInstanceOf(GitError);
    expect(rehydrated?.code).toBe('auth-failed');
    expect(rehydrated?.message).toBe('bad token');
    expect(rehydrated?.recoverable).toBe(false);
  });

  it('returns null for a plain Error without the marker', () => {
    expect(rehydrateGitError(new Error('network broke'))).toBeNull();
  });

  it('returns null for non-Error inputs', () => {
    expect(rehydrateGitError('string')).toBeNull();
    expect(rehydrateGitError({ message: 'fake' })).toBeNull();
    expect(rehydrateGitError(null)).toBeNull();
  });

  it('returns null for a marker Error with malformed JSON', () => {
    const bad = new Error(`${GIT_ERROR_MARKER}{not json`);
    expect(rehydrateGitError(bad)).toBeNull();
  });

  it('round-trips all common error codes', () => {
    const codes = [
      'init-failed',
      'commit-empty',
      'branch-exists',
      'merge-still-conflicted',
      'save-required',
    ] as const;
    for (const code of codes) {
      const rehydrated = rehydrateGitError(encode(code, `${code} test`));
      expect(rehydrated?.code).toBe(code);
    }
  });
});
