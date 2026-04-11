// apps/web/src/stores/__tests__/git-store-helpers.test.ts
//
// Unit tests for the pure helpers in git-store-helpers.ts. These pin the
// classification contract independently of the store so a future drift
// (e.g. the helper stops importing REMOTE_AUTH_ERROR_CODES) is caught at
// the unit level instead of leaking into integration tests.

import { describe, it, expect } from 'vitest';
import { GitError } from '@/services/git-error';
import { classifyRemoteAuthError } from '@/stores/git-store-helpers';
import { REMOTE_AUTH_ERROR_CODES } from '@/stores/git-store-types';

describe('classifyRemoteAuthError', () => {
  // The contract: an auth code in REMOTE_AUTH_ERROR_CODES must classify
  // as { kind: 'auth' } so the pull/push buttons know to open the shared
  // auth form. Everything else must fall through to { kind: 'other' } so
  // the component's generic error handling runs.

  // Spread into a mutable array so it.each's tuple-vs-readonly-array typing
  // doesn't fight us. REMOTE_AUTH_ERROR_CODES is `readonly [...]`.
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
