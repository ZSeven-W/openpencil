// apps/desktop/git/__tests__/git-sys.test.ts
import { describe, it, expect, beforeEach } from 'vitest';
import { isSystemGitAvailable, __resetSystemGitCache, getSystemAuthor } from '../git-sys';

describe('git-sys', () => {
  beforeEach(() => {
    __resetSystemGitCache();
  });

  it('isSystemGitAvailable returns a boolean and caches the result', async () => {
    const first = await isSystemGitAvailable();
    expect(typeof first).toBe('boolean');
    // Second call should hit the cache and return the same value.
    const second = await isSystemGitAvailable();
    expect(second).toBe(first);
  });
});

describe('getSystemAuthor (injected exec)', () => {
  // These tests use the injected-exec seam to stay deterministic and avoid
  // depending on whatever user.name/user.email happen to be configured on the
  // host running the suite. The seam short-circuits isSystemGitAvailable and
  // runGit entirely, so we exercise only the parse/validate/catch logic.

  it('returns parsed name/email on success', async () => {
    const calls: string[][] = [];
    const fakeExec = async (args: string[]) => {
      calls.push(args);
      if (args[2] === 'user.name') return { stdout: 'Alice\n', stderr: '' };
      if (args[2] === 'user.email') return { stdout: 'alice@example.com\n', stderr: '' };
      return { stdout: '', stderr: '' };
    };

    const result = await getSystemAuthor(fakeExec);

    expect(result).toEqual({ name: 'Alice', email: 'alice@example.com' });
    expect(calls).toHaveLength(2);
    expect(calls[0]).toEqual(['config', '--get', 'user.name']);
    expect(calls[1]).toEqual(['config', '--get', 'user.email']);
  });

  it('returns null when git throws (e.g. key not set)', async () => {
    const calls: string[][] = [];
    const fakeExec = async (args: string[]) => {
      calls.push(args);
      // Simulate `git config --get user.name` exiting non-zero when unset.
      throw new Error('git config --get user.name failed: exit code 1');
    };

    const result = await getSystemAuthor(fakeExec);

    expect(result).toBeNull();
    // First call throws, so the second call never happens.
    expect(calls).toHaveLength(1);
  });

  it('returns null when either value is empty/whitespace', async () => {
    const calls: string[][] = [];
    const fakeExec = async (args: string[]) => {
      calls.push(args);
      if (args[2] === 'user.name') return { stdout: 'Bob\n', stderr: '' };
      if (args[2] === 'user.email') return { stdout: '   \n', stderr: '' };
      return { stdout: '', stderr: '' };
    };

    const result = await getSystemAuthor(fakeExec);

    expect(result).toBeNull();
    // Both calls happen because validation is post-fetch.
    expect(calls).toHaveLength(2);
  });
});
