// apps/web/src/services/git-error.ts
//
// Renderer 端 GitError 类。 Mirrors apps/desktop/git/error.ts 形状
// 所以商店+面板可以在.code 上进行 `instanceof GitError` 和模式匹配
// 均匀地。
//
// Electron IPC 通过桥删除自定义 Error 子类。 Phase 2a 的
// ipc-handlers.ts 通过将 GitError 序列化为普通格式来解决此问题
// Error，其消息以 GIT_ERROR_MARKER 开头，后跟 JSON。
// This 文件提供逆操作 — rehydrateGitError 解析
// 标记编码消息返回到 GitError 实例。

import type { GitErrorCode } from './git-types';

export const GIT_ERROR_MARKER = '__GIT_ERROR__';

export class GitError extends Error {
  readonly code: GitErrorCode;
  readonly recoverable: boolean;
  readonly detail?: unknown;

  constructor(
    code: GitErrorCode,
    message: string,
    opts: { recoverable?: boolean; detail?: unknown } = {},
  ) {
    super(message);
    this.name = 'GitError';
    this.code = code;
    this.recoverable = opts.recoverable ?? true;
    if (opts.detail !== undefined) this.detail = opts.detail;
  }
}

/**
 * Defensive 型防护罩。 `instanceof GitError` 在同一领域代码中工作，但是
 * if the GitError was reconstructed from an IPC payload it may be a plain
 * 对象而不是类实例。 This 守卫处理两者。
 */
export function isGitError(err: unknown): err is GitError {
  if (err instanceof GitError) return true;
  if (typeof err !== 'object' || err === null) return false;
  const e = err as { name?: string; code?: unknown };
  return e.name === 'GitError' && typeof e.code === 'string';
}

/**
 * Parse 是 IPC
 * 传递的 Error，其消息以 GIT_ERROR_MARKER 开头返回到 GitError 实例。 Returns null
 * 对于任何与标记格式不匹配的输入 - 调用者应该重新抛出原始错误。
 *
 */
export function rehydrateGitError(err: unknown): GitError | null {
  if (!(err instanceof Error)) return null;
  if (typeof err.message !== 'string') return null;
  if (!err.message.startsWith(GIT_ERROR_MARKER)) return null;

  try {
    const raw = err.message.slice(GIT_ERROR_MARKER.length);
    const payload = JSON.parse(raw) as {
      code: GitErrorCode;
      message: string;
      recoverable: boolean;
    };
    return new GitError(payload.code, payload.message, {
      recoverable: payload.recoverable,
    });
  } catch {
    return null;
  }
}
