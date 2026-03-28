import { execSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { homedir } from 'node:os';
import { serverLog } from './server-logger';

const isWindows = process.platform === 'win32';

/** Windows npm global installs may create .cmd or .ps1 wrappers — try both */
function winNpmCandidates(dir: string, name: string): string[] {
  return [join(dir, `${name}.cmd`), join(dir, `${name}.ps1`)];
}

/** On Windows, `where` may return an extensionless shell script — prefer .cmd/.ps1 */
function resolveWinExtension(binPath: string): string {
  if (!isWindows) return binPath;
  if (/\.(cmd|ps1|exe)$/i.test(binPath)) return binPath;
  for (const ext of ['.cmd', '.ps1']) {
    if (existsSync(binPath + ext)) return binPath + ext;
  }
  return binPath;
}

/** Resolve the Gemini CLI binary path across macOS, Linux, and Windows. */
export function resolveGeminiCli(): string | undefined {
  serverLog.info(`[resolve-gemini] platform=${process.platform}, isWindows=${isWindows}`);

  // 1. Try PATH lookup
  try {
    const cmd = isWindows ? 'where gemini 2>nul' : 'which gemini 2>/dev/null';
    serverLog.info(`[resolve-gemini] PATH lookup: ${cmd}`);
    const result = execSync(cmd, { encoding: 'utf-8', timeout: 5000 }).trim();
    // `where` on Windows may return multiple lines
    const path = result.split(/\r?\n/)[0]?.trim();
    serverLog.info(
      `[resolve-gemini] PATH result: "${path}" (exists=${path ? existsSync(path) : false})`,
    );
    if (path && existsSync(path)) return resolveWinExtension(path);
  } catch (err) {
    serverLog.info(
      `[resolve-gemini] PATH lookup failed: ${err instanceof Error ? err.message : err}`,
    );
  }

  // 2. Try `npm prefix -g` (Windows uses npm.cmd; Unix uses npm)
  try {
    const npmCmd = isWindows ? 'npm.cmd prefix -g' : 'npm prefix -g';
    serverLog.info(`[resolve-gemini] npm prefix lookup: ${npmCmd}`);
    const prefix = execSync(npmCmd, { encoding: 'utf-8', timeout: 5000 }).trim();
    serverLog.info(`[resolve-gemini] npm global prefix: "${prefix}"`);
    if (prefix) {
      if (isWindows) {
        for (const bin of winNpmCandidates(prefix, 'gemini')) {
          serverLog.info(`[resolve-gemini] npm global bin: "${bin}" (exists=${existsSync(bin)})`);
          if (existsSync(bin)) return bin;
        }
      } else {
        const bin = join(prefix, 'bin', 'gemini');
        serverLog.info(`[resolve-gemini] npm global bin: "${bin}" (exists=${existsSync(bin)})`);
        if (existsSync(bin)) return bin;
      }
    }
  } catch (err) {
    serverLog.info(
      `[resolve-gemini] npm prefix -g failed: ${err instanceof Error ? err.message : err}`,
    );
  }

  // 3. Common install locations
  const home = homedir();
  const candidates = isWindows
    ? [
        // npm global (.cmd + .ps1)
        ...winNpmCandidates(join(process.env.APPDATA || '', 'npm'), 'gemini'),
        // nvm-windows / fnm
        ...winNpmCandidates(join(process.env.NVM_SYMLINK || ''), 'gemini'),
        ...winNpmCandidates(join(process.env.FNM_MULTISHELL_PATH || ''), 'gemini'),
        // winget / native
        join(process.env.LOCALAPPDATA || '', 'Microsoft', 'WinGet', 'Links', 'gemini.exe'),
      ]
    : [
        // npm global
        '/usr/local/bin/gemini',
        // Homebrew (macOS)
        '/opt/homebrew/bin/gemini',
        // User-local
        join(home, '.local', 'bin', 'gemini'),
        join(home, '.npm-global', 'bin', 'gemini'),
      ];

  for (const c of candidates) {
    const exists = c ? existsSync(c) : false;
    serverLog.info(`[resolve-gemini] candidate: "${c}" (exists=${exists})`);
    if (c && exists) return c;
  }

  serverLog.warn('[resolve-gemini] no gemini binary found');
  return undefined;
}
