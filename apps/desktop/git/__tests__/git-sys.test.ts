// apps/desktop/git/__tests__/git-sys.test.ts
import { describe, it, expect, beforeEach } from 'vitest';
import { isSystemGitAvailable, __resetSystemGitCache } from '../git-sys';

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
