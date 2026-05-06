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
    // Second 调用应该命中缓存并返回相同的值。
    const second = await isSystemGitAvailable();
    expect(second).toBe(first);
  });
});

describe('getSystemAuthor (injected exec)', () => {
  // These 测试使用注入执行接缝来保持确定性，并避免依赖于运行套件的主机上碰巧配置的任何 user.name/user.email。 The
  // 接缝使 isSystemGitAvailable 和 runGit 完全短路，因此我们仅练习 parse/validate/catch 逻辑。

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
      // Simulate `git config --get user.name` 未设置时退出非零。
      throw new Error('git config --get user.name failed: exit code 1');
    };

    const result = await getSystemAuthor(fakeExec);

    expect(result).toBeNull();
    // First 调用会抛出异常，因此第二个调用永远不会发生。
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
    // Both 调用发生是因为验证是提取后的。
    expect(calls).toHaveLength(2);
  });
});
