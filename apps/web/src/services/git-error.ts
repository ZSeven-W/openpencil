// apps/web/src/services/git-error.ts
//
// Renderer-side GitError class. Mirrors apps/desktop/git/error.ts in shape
// so the store + panel can `instanceof GitError` and pattern-match on .code
// uniformly.
//
// Electron IPC drops custom Error subclasses across the bridge. Phase 2a's
// ipc-handlers.ts works around this by serializing GitError into a plain
// Error whose message starts with the GIT_ERROR_MARKER followed by JSON.
// This file provides the inverse operation — rehydrateGitError parses the
// marker-encoded message back into a GitError instance.

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
 * Defensive type guard. `instanceof GitError` works in same-realm code, but
 * if the GitError was reconstructed from an IPC payload it may be a plain
 * object rather than a class instance. This guard handles both.
 */
export function isGitError(err: unknown): err is GitError {
  if (err instanceof GitError) return true;
  if (typeof err !== 'object' || err === null) return false;
  const e = err as { name?: string; code?: unknown };
  return e.name === 'GitError' && typeof e.code === 'string';
}

/**
 * Parse an IPC-delivered Error whose message starts with GIT_ERROR_MARKER
 * back into a GitError instance. Returns null for any input that doesn't
 * match the marker format — the caller should fall back to re-throwing the
 * original error.
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
