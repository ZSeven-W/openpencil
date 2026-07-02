/**
 * Shared helpers for resolving local CLI binaries across the builtin agent
 * providers. Electron on macOS does NOT inherit the user's login-shell PATH,
 * so a bare `which <cli>` from Nitro often fails even when the CLI works
 * in the user's terminal. Common install locations like ~/.bun/bin,
 * ~/.nvm/versions/node/<ver>/bin, ~/Library/pnpm, ~/.volta/bin,
 * ~/.asdf/shims, ~/.local/share/mise/shims are invisible to the server.
 *
 * Two primitives here:
 *   - posixUserBinDirs(): enumerate the standard user-local / package-manager
 *     bin directories, so resolvers can scan them as concrete candidates.
 *   - probeViaLoginShell(binary): ask the user's configured shell for the
 *     resolved path of a binary, picking up nvm / pnpm / bun / mise / asdf
 *     shims without having to enumerate every possible install layout.
 *
 * Everything logs through serverLog so the server-YYYY-MM-DD.log file gets
 * the full resolution trace for remote diagnosis.
 */

import { execSync } from 'node:child_process';
import { existsSync, readdirSync } from 'node:fs';
import { homedir, platform } from 'node:os';
import { join } from 'node:path';
import { serverLog } from './server-logger';

const isWindows = platform() === 'win32';

/** Enumerate the standard macOS/Linux user-local install directories. */
export function posixUserBinDirs(): string[] {
  const home = homedir();
  const dirs = [
    join(home, '.bun', 'bin'),
    join(home, '.volta', 'bin'),
    join(home, '.local', 'bin'),
    join(home, '.local', 'share', 'mise', 'shims'),
    join(home, '.asdf', 'shims'),
    join(home, 'Library', 'pnpm'),
    join(home, '.pnpm-global', 'bin'),
    join(home, '.cargo', 'bin'),
    '/usr/local/bin',
    '/opt/homebrew/bin',
  ];

  // nvm: enumerate installed node versions best-effort (just readdir)
  try {
    const nvmNodeRoot = join(home, '.nvm', 'versions', 'node');
    if (existsSync(nvmNodeRoot)) {
      for (const ver of readdirSync(nvmNodeRoot)) {
        dirs.push(join(nvmNodeRoot, ver, 'bin'));
      }
    }
  } catch {
    /* best effort */
  }

  // fnm
  try {
    const fnmRoot = join(home, '.fnm', 'node-versions');
    if (existsSync(fnmRoot)) {
      for (const ver of readdirSync(fnmRoot)) {
        dirs.push(join(fnmRoot, ver, 'installation', 'bin'));
      }
    }
  } catch {
    /* best effort */
  }

  return dirs;
}

/**
 * Standard locations where fish is typically installed on macOS/Linux.
 * Homebrew Apple Silicon uses /opt/homebrew, Homebrew Intel + most distros
 * ship to /usr/local, MacPorts uses /opt/local.
 */
const FISH_FALLBACK_PATHS = [
  '/opt/homebrew/bin/fish',
  '/usr/local/bin/fish',
  '/opt/local/bin/fish',
  '/usr/bin/fish',
];

/**
 * Probe the user's login shell for the resolved path of a binary.
 * Runs `<shell> -ilc 'command -v <binary>'` to source the user's rc + profile
 * so nvm / pnpm / bun / mise / asdf shims wired there are visible. `prefix`
 * is used only for the log lines to match the caller's existing namespace.
 *
 * Supports zsh, bash, and fish — fish users configure their PATH in
 * `~/.config/fish/config.fish` (often via nvm.fish / mise / bass), which
 * neither zsh nor bash will source. `command -v` and `2>/dev/null` both
 * work in fish 3.x, so the invocation is identical across shells.
 */
export function probeViaLoginShell(binary: string, prefix: string): string | undefined {
  if (isWindows) return undefined;

  const userShell = process.env.SHELL;
  const shells: string[] = [];

  // User's declared shell wins — the one that sources their real rc/profile.
  if (userShell && existsSync(userShell)) shells.push(userShell);

  // Fish fallback: many users run fish but Electron may launch with SHELL
  // unset (e.g. Dock-launched apps inherit a scrubbed environment). Without
  // this, fish-only PATH entries (nvm.fish / mise / bass) stay invisible.
  if (!shells.some((s) => s.endsWith('/fish'))) {
    for (const p of FISH_FALLBACK_PATHS) {
      if (existsSync(p)) {
        shells.push(p);
        break;
      }
    }
  }

  // zsh / bash catch-all — most POSIX CLIs wire their shims through at least
  // one of these even on fish-primary systems.
  if (!shells.some((s) => s.endsWith('/zsh')) && existsSync('/bin/zsh')) {
    shells.push('/bin/zsh');
  }
  if (!shells.some((s) => s.endsWith('/bash')) && existsSync('/bin/bash')) {
    shells.push('/bin/bash');
  }

  for (const shell of shells) {
    try {
      const cmd = `${shell} -ilc 'command -v ${binary} 2>/dev/null' 2>/dev/null`;
      serverLog.info(`[${prefix}] login-shell probe: ${cmd}`);
      const raw = execSync(cmd, {
        encoding: 'utf-8',
        timeout: 6000,
        // Start from a minimal env — inheriting Electron's env can suppress
        // the login-shell side effects we rely on (nvm's __NVM_DIR etc).
        env: { HOME: homedir(), USER: process.env.USER ?? '' },
      }).trim();
      const path = raw.split(/\r?\n/).filter(Boolean).pop();
      if (path && existsSync(path)) {
        serverLog.info(`[${prefix}] login-shell probe hit via ${shell}: "${path}"`);
        return path;
      }
      if (path) {
        serverLog.info(
          `[${prefix}] login-shell (${shell}) returned "${path}" but file does not exist`,
        );
      }
    } catch (err) {
      serverLog.info(
        `[${prefix}] login-shell probe via ${shell} failed: ${err instanceof Error ? err.message : err}`,
      );
    }
  }
  return undefined;
}
