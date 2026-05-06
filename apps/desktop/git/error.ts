// apps/desktop/git/error.ts
//
// Unified 桌面 git 层的错误类型。 Phase 1b 仅发出一个子集
// 这些代码； Phase 2 将抛出其余部分（身份验证、网络、合并等）
// 无需修改此文件。

/**
 * The 完整错误代码联合
 * 。 New 代码应该放在这里，而不是在调用站点中，因此规范中渲染器的错误矩阵与现实保持同步。
 */
export type GitErrorCode =
  // Phase 1b 发出这些：
  | 'init-failed'
  | 'open-failed'
  | 'not-a-repo'
  | 'commit-empty'
  | 'branch-exists'
  | 'branch-current'
  | 'branch-unmerged'
  | 'engine-crash'
  // Phase 2 将发出这些（此处声明是为了向前兼容）：
  | 'no-file'
  | 'clone-failed'
  | 'clone-target-exists'
  | 'clone-network'
  | 'auth-required'
  | 'auth-failed'
  | 'auth-token-invalid'
  | 'network'
  | 'timeout'
  | 'commit-author-missing'
  | 'pull-non-fast-forward'
  | 'push-rejected'
  | 'push-no-remote'
  | 'branch-switch-dirty'
  | 'merge-conflict'
  | 'merge-conflict-non-op'
  | 'merge-still-conflicted'
  | 'merge-abort-failed'
  | 'restore-dirty'
  | 'ssh-not-supported-iso'
  | 'ssh-key-missing'
  | 'concurrent-busy'
  | 'external-modified'
  | 'save-required';

export class GitError extends Error {
  readonly code: GitErrorCode;
  readonly recoverable: boolean;
  readonly detail?: unknown;

  constructor(
    code: GitErrorCode,
    message: string,
    opts: { recoverable?: boolean; detail?: unknown; cause?: unknown } = {},
  ) {
    super(message, opts.cause !== undefined ? { cause: opts.cause } : undefined);
    this.name = 'GitError';
    this.code = code;
    this.recoverable = opts.recoverable ?? true;
    if (opts.detail !== undefined) this.detail = opts.detail;
  }
}

/**
 * Type 专门用于捕捉
 * GitError 的守卫（因为跨领域的 `instanceof` 在测试中可能会不稳定，所以这是一个防御性备份）。
 */
export function isGitError(err: unknown): err is GitError {
  return (
    err instanceof GitError ||
    (typeof err === 'object' && err !== null && (err as { name?: string }).name === 'GitError')
  );
}
