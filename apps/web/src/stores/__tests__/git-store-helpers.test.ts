// apps/web/src/stores/__tests__/git-store-helpers.test.ts
//
// Unit 测试 git-store-helpers.ts 中的纯助手。 These 固定
// 分类合同独立于商店，因此未来会发生变化
// （例如，助手停止导入 REMOTE_AUTH_ERROR_CODES）被捕获
// 单元级别而不是泄漏到集成测试中。

import { describe, it, expect } from 'vitest';
import { GitError } from '@/services/git-error';
import { classifyRemoteAuthError } from '@/stores/git-store-helpers';
import { REMOTE_AUTH_ERROR_CODES } from '@/stores/git-store-types';

describe('classifyRemoteAuthError', () => {
  // The 合约：REMOTE_AUTH_ERROR_CODES 中的身份验证代码必须分类为 { kind: 'auth' }，以便
  // pull/push 按钮知道打开共享身份验证表单。 Everything else 必须落到 { kind: 'other' }
  // 以便组件的通用错误处理运行。 Spread 到可变数组中，因此 it.each 的元组与只读数组类型不会与我们对抗。
  // REMOTE_AUTH_ERROR_CODES 是 `readonly [...]`。
  it.each([...REMOTE_AUTH_ERROR_CODES])(
    'classifies GitError(%s) as { kind: "auth" } for both pull and push',
    (code) => {
      const err = new GitError(code, `HTTP error for ${code}`);
      const pull = classifyRemoteAuthError(err, 'pull');
      const push = classifyRemoteAuthError(err, 'push');
      expect(pull).toEqual({ kind: 'auth', code, message: `HTTP error for ${code}` });
      expect(push).toEqual({ kind: 'auth', code, message: `HTTP error for ${code}` });
    },
  );

  it('classifies non-auth GitError codes as { kind: "other" }', () => {
    const crash = new GitError('engine-crash', 'boom');
    const save = new GitError('save-required', 'dirty');
    expect(classifyRemoteAuthError(crash, 'pull')).toEqual({ kind: 'other' });
    expect(classifyRemoteAuthError(crash, 'push')).toEqual({ kind: 'other' });
    expect(classifyRemoteAuthError(save, 'pull')).toEqual({ kind: 'other' });
    expect(classifyRemoteAuthError(save, 'push')).toEqual({ kind: 'other' });
  });

  it('classifies non-GitError values as { kind: "other" }', () => {
    expect(classifyRemoteAuthError(new Error('plain error'), 'pull')).toEqual({ kind: 'other' });
    expect(classifyRemoteAuthError('string error', 'push')).toEqual({ kind: 'other' });
    expect(classifyRemoteAuthError(null, 'pull')).toEqual({ kind: 'other' });
    expect(classifyRemoteAuthError(undefined, 'push')).toEqual({ kind: 'other' });
  });
});
