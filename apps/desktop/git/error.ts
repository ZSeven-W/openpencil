// apps/desktop/git/error.ts
//
// Unified error type for the desktop git layer. Phase 1b only emits a subset
// of these codes; Phase 2 will throw the rest (auth, network, merge, etc.)
// without modifying this file.

/**
 * The complete error code union. New codes should land here, not in callsites,
 * so the renderer's error matrix in the spec stays in sync with the reality.
 */
export type GitErrorCode =
  // Phase 1b emits these:
  | 'init-failed'
  | 'open-failed'
  | 'not-a-repo'
  | 'commit-empty'
  | 'branch-exists'
  | 'branch-current'
  | 'branch-unmerged'
  | 'engine-crash'
  // Phase 2 will emit these (declared here for forward-compat):
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
 * Type guard for catching GitError specifically (since `instanceof` across
 * realms can be flaky in tests, this is a defensive backup).
 */
export function isGitError(err: unknown): err is GitError {
  return (
    err instanceof GitError ||
    (typeof err === 'object' && err !== null && (err as { name?: string }).name === 'GitError')
  );
}
