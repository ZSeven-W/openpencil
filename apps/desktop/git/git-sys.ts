// apps/desktop/git/git-sys.ts
//
// System git wrapper. Phase 2b makes this real: clone/fetch/push/pull-FF
// run via execFile, returning typed results or throwing GitError on
// recognized failure modes.
//
// All ops accept an optional env map so SSH callers can set
// GIT_SSH_COMMAND='ssh -i <keyPath> -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new'
// without affecting the parent process env.

import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { GitError, type GitErrorCode } from './error';

const execFileAsync = promisify(execFile);

let cached: boolean | undefined;

const DEFAULT_TIMEOUT_MS = 60_000;

export async function isSystemGitAvailable(): Promise<boolean> {
  if (cached !== undefined) return cached;
  try {
    await execFileAsync('git', ['--version'], { timeout: 5000 });
    cached = true;
  } catch {
    cached = false;
  }
  return cached;
}

export function __resetSystemGitCache(): void {
  cached = undefined;
}

interface RunOpts {
  cwd?: string;
  env?: Record<string, string>;
  timeoutMs?: number;
}

interface RunResult {
  stdout: string;
  stderr: string;
}

/**
 * Run `git <args>` with the given env and cwd. On failure, maps stderr to a
 * GitError code and throws. Used by all the higher-level fns below.
 */
async function runGit(args: string[], opts: RunOpts = {}): Promise<RunResult> {
  const env = { ...process.env, ...opts.env };
  try {
    const { stdout, stderr } = await execFileAsync('git', args, {
      cwd: opts.cwd,
      env,
      timeout: opts.timeoutMs ?? DEFAULT_TIMEOUT_MS,
      maxBuffer: 32 * 1024 * 1024,
    });
    return { stdout, stderr };
  } catch (err) {
    const e = err as NodeJS.ErrnoException & { stderr?: string; stdout?: string };
    const stderr = e.stderr ?? '';
    const code = mapSysError(stderr, e);
    throw new GitError(code, `git ${args.join(' ')} failed: ${stderr.trim() || e.message}`, {
      cause: err,
      detail: { args, stderr },
    });
  }
}

/**
 * Map stderr text from a failed `git` invocation to a GitError code.
 * Patterns are intentionally simple substring checks — git's error messages
 * are not localized and have been stable for many years.
 */
export function mapSysError(stderr: string, err?: { message?: string }): GitErrorCode {
  const s = stderr.toLowerCase();
  const msg = (err?.message ?? '').toLowerCase();

  // Auth failures
  if (
    s.includes('authentication failed') ||
    s.includes('could not read username') ||
    s.includes('permission denied (publickey)') ||
    s.includes('access denied')
  ) {
    return 'auth-failed';
  }

  // Repository missing
  if (s.includes('repository not found') || s.includes('does not exist')) {
    return 'clone-failed';
  }

  // Clone target already exists
  if (s.includes('already exists and is not an empty directory')) {
    return 'clone-target-exists';
  }

  // Network failures
  if (
    s.includes("couldn't resolve host") ||
    s.includes('could not resolve hostname') ||
    s.includes('connection refused') ||
    s.includes('no route to host')
  ) {
    return 'network';
  }

  // Timeouts (both git's own and execFile's)
  if (
    s.includes('connection timed out') ||
    s.includes('operation timed out') ||
    msg.includes('etimedout')
  ) {
    return 'timeout';
  }

  // Push rejected (non-FF)
  if (s.includes('updates were rejected') || s.includes('non-fast-forward')) {
    return 'push-rejected';
  }

  // Pull non-FF (we run --ff-only so any divergence yields this)
  if (s.includes('not possible to fast-forward')) {
    return 'pull-non-fast-forward';
  }

  // Not a repo
  if (s.includes('not a git repository') || s.includes('does not appear to be a git repository')) {
    return 'not-a-repo';
  }

  return 'engine-crash';
}

// ---------------------------------------------------------------------------
// Public ops
// ---------------------------------------------------------------------------

export async function sysClone(opts: {
  url: string;
  dest: string;
  env?: Record<string, string>;
  timeoutMs?: number;
}): Promise<void> {
  await runGit(['clone', opts.url, opts.dest], {
    env: opts.env,
    timeoutMs: opts.timeoutMs,
  });
}

export async function sysFetch(opts: {
  cwd: string;
  remote?: string;
  env?: Record<string, string>;
  timeoutMs?: number;
}): Promise<void> {
  const remote = opts.remote ?? 'origin';
  await runGit(['fetch', remote], {
    cwd: opts.cwd,
    env: opts.env,
    timeoutMs: opts.timeoutMs,
  });
}

export async function sysPullFastForward(opts: {
  cwd: string;
  remote?: string;
  branch: string;
  env?: Record<string, string>;
  timeoutMs?: number;
}): Promise<{ result: 'fast-forward' | 'up-to-date' }> {
  const remote = opts.remote ?? 'origin';
  // --ff-only refuses to merge; if the remote diverged, throws pull-non-fast-forward.
  const { stdout } = await runGit(['pull', '--ff-only', remote, opts.branch], {
    cwd: opts.cwd,
    env: opts.env,
    timeoutMs: opts.timeoutMs,
  });
  if (stdout.includes('Already up to date')) return { result: 'up-to-date' };
  return { result: 'fast-forward' };
}

export async function sysPush(opts: {
  cwd: string;
  remote?: string;
  branch: string;
  env?: Record<string, string>;
  timeoutMs?: number;
}): Promise<void> {
  const remote = opts.remote ?? 'origin';
  await runGit(['push', remote, opts.branch], {
    cwd: opts.cwd,
    env: opts.env,
    timeoutMs: opts.timeoutMs,
  });
}

/**
 * Compute ahead/behind counts for the current branch vs `<remote>/<branch>`.
 * Returns 0/0 if no remote-tracking ref exists for the branch.
 */
export async function sysAheadBehind(opts: {
  cwd: string;
  branch: string;
  remote?: string;
  env?: Record<string, string>;
}): Promise<{ ahead: number; behind: number }> {
  const remote = opts.remote ?? 'origin';
  try {
    const { stdout } = await runGit(
      ['rev-list', '--left-right', '--count', `${opts.branch}...${remote}/${opts.branch}`],
      { cwd: opts.cwd, env: opts.env },
    );
    // Output format: "<ahead>\t<behind>\n"
    const [aheadStr, behindStr] = stdout.trim().split(/\s+/);
    return {
      ahead: parseInt(aheadStr, 10) || 0,
      behind: parseInt(behindStr, 10) || 0,
    };
  } catch (err) {
    // No remote tracking ref → not an error, just zeros.
    if (err instanceof GitError && err.code === 'engine-crash') {
      return { ahead: 0, behind: 0 };
    }
    throw err;
  }
}

/**
 * Executor shape used by getSystemAuthor's test seam. Mirrors runGit's return
 * type so tests can inject a fake without pulling in node:child_process.
 */
type RunGitExec = (args: string[]) => Promise<{ stdout: string; stderr: string }>;

/**
 * Read `user.name` / `user.email` from the system git config. Used by Phase 4a
 * as step 2 of the author identity lookup chain (prefs → sysGit → form).
 *
 * The optional `injectedExec` parameter is a TEST SEAM: production callers pass
 * no args and get the real `runGit`-backed path, while tests inject a fake to
 * deterministically exercise both the success and null branches without
 * depending on the host machine having a configured global git identity.
 *
 * Returns null on:
 *   - system git not available (when not using injected exec)
 *   - either config value missing / empty / whitespace-only
 *   - git exec throwing (e.g. key not set, which git reports as exit 1)
 * The catch block intentionally swallows errors: "no identity" is a normal
 * state, not an operational failure, and the caller treats null as "fall
 * through to the next step in the chain".
 */
export async function getSystemAuthor(
  injectedExec?: RunGitExec,
): Promise<{ name: string; email: string } | null> {
  if (!injectedExec && !(await isSystemGitAvailable())) return null;

  const exec: RunGitExec = injectedExec ?? ((args) => runGit(args, { timeoutMs: 5000 }));

  try {
    const nameResult = await exec(['config', '--get', 'user.name']);
    const emailResult = await exec(['config', '--get', 'user.email']);
    const name = nameResult.stdout.trim();
    const email = emailResult.stdout.trim();
    if (!name || !email) return null;
    return { name, email };
  } catch {
    return null;
  }
}

/**
 * Build the GIT_SSH_COMMAND env value for an SSH key file. Used by the
 * engine before invoking sysClone/Fetch/Pull/Push with auth.kind === 'ssh'.
 */
export function buildSshCommand(privateKeyPath: string): string {
  // -i: identity file
  // -o IdentitiesOnly=yes: don't try ssh-agent identities (avoid prompting)
  // -o StrictHostKeyChecking=accept-new: trust on first use, verify thereafter
  // -o BatchMode=yes: never prompt; fail fast on missing creds
  return [
    'ssh',
    '-i',
    JSON.stringify(privateKeyPath),
    '-o',
    'IdentitiesOnly=yes',
    '-o',
    'StrictHostKeyChecking=accept-new',
    '-o',
    'BatchMode=yes',
  ].join(' ');
}
