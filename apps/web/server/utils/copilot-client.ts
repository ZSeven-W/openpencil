import { execSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { serverLog } from './server-logger';
import { posixUserBinDirs, probeViaLoginShell } from './cli-resolver-helpers';

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

/** Resolve the standalone copilot CLI binary path to avoid Bun's node:sqlite issue */
export function resolveCopilotCli(): string | undefined {
  serverLog.info(
    `[resolve-copilot] platform=${process.platform}, isWindows=${isWindows}, SHELL=${process.env.SHELL ?? 'unset'}`,
  );

  // 1. Try PATH lookup
  try {
    const cmd = isWindows ? 'where copilot 2>nul' : 'which copilot 2>/dev/null';
    serverLog.info(`[resolve-copilot] PATH lookup: ${cmd}`);
    const result = execSync(cmd, { encoding: 'utf-8', timeout: 5000 }).trim();
    // `where` on Windows may return multiple lines
    const path = result.split(/\r?\n/)[0]?.trim();
    serverLog.info(
      `[resolve-copilot] PATH result: "${path}" (exists=${path ? existsSync(path) : false})`,
    );
    if (path && existsSync(path)) return resolveWinExtension(path);
  } catch (err) {
    serverLog.info(
      `[resolve-copilot] PATH lookup failed: ${err instanceof Error ? err.message : err}`,
    );
  }

  // 2. macOS/Linux login-shell probe
  if (!isWindows) {
    const viaShell = probeViaLoginShell('copilot', 'resolve-copilot');
    if (viaShell) return viaShell;
  }

  // 3. Try `npm prefix -g` on Windows (npm install -g creates .cmd wrappers)
  if (isWindows) {
    try {
      serverLog.info('[resolve-copilot] trying npm.cmd prefix -g');
      const prefix = execSync('npm.cmd prefix -g', {
        encoding: 'utf-8',
        timeout: 5000,
      }).trim();
      serverLog.info(`[resolve-copilot] npm global prefix: "${prefix}"`);
      if (prefix) {
        for (const bin of winNpmCandidates(prefix, 'copilot')) {
          serverLog.info(`[resolve-copilot] npm global bin: "${bin}" (exists=${existsSync(bin)})`);
          if (existsSync(bin)) return bin;
        }
      }
    } catch (err) {
      serverLog.info(
        `[resolve-copilot] npm prefix -g failed: ${err instanceof Error ? err.message : err}`,
      );
    }
  }

  // 4. Common install locations
  const candidates = isWindows
    ? [
        // npm global (.cmd + .ps1)
        ...winNpmCandidates(join(process.env.APPDATA || '', 'npm'), 'copilot'),
        // nvm-windows / fnm
        ...winNpmCandidates(join(process.env.NVM_SYMLINK || ''), 'copilot'),
        ...winNpmCandidates(join(process.env.FNM_MULTISHELL_PATH || ''), 'copilot'),
        // winget / native
        join(process.env.LOCALAPPDATA || '', 'Microsoft', 'WinGet', 'Links', 'copilot.exe'),
      ]
    : posixUserBinDirs().map((dir) => join(dir, 'copilot'));
  for (const c of candidates) {
    const exists = c ? existsSync(c) : false;
    serverLog.info(`[resolve-copilot] candidate: "${c}" (exists=${exists})`);
    if (c && exists) return c;
  }

  serverLog.warn(
    '[resolve-copilot] no copilot binary found after PATH, login-shell probe, and candidate scan',
  );
  return undefined;
}

/**
 * On Windows, `.cmd` wrappers cannot be spawned directly by the copilot-sdk
 * (it calls `spawn()` without `shell: true`, causing EINVAL).
 * Resolve the `.cmd` to the actual `.js` entry point so the SDK can run it via node.
 */
export function resolveCliPathForSdk(cliPath: string): string {
  if (!isWindows || !cliPath.endsWith('.cmd')) return cliPath;

  // npm .cmd wrappers live alongside node_modules — the JS entry is at:
  // <dir>/node_modules/@github/copilot/index.js
  const dir = dirname(cliPath);
  const jsEntry = join(dir, 'node_modules', '@github', 'copilot', 'index.js');
  if (existsSync(jsEntry)) {
    serverLog.info(`[resolve-copilot] resolved .cmd → .js: ${jsEntry}`);
    return jsEntry;
  }

  // Fallback: parse the .cmd file to extract the JS path
  // npm .cmd wrappers use %dp0% or %~dp0 to reference their own directory
  try {
    const content = readFileSync(cliPath, 'utf-8');
    const match = content.match(/"([^"]+\.js)"/g);
    if (match) {
      for (const m of match) {
        const jsPath = m.replace(/"/g, '').replace(/%~?dp0%?\\/g, dir + '\\');
        if (jsPath.includes('copilot') && existsSync(jsPath)) {
          serverLog.info(`[resolve-copilot] parsed .cmd → .js: ${jsPath}`);
          return jsPath;
        }
      }
    }
  } catch (err) {
    serverLog.warn(
      `[resolve-copilot] failed to parse .cmd: ${err instanceof Error ? err.message : err}`,
    );
  }

  // Last resort: return as-is (will likely fail with EINVAL)
  serverLog.warn(`[resolve-copilot] could not resolve .cmd to .js, using as-is: ${cliPath}`);
  return cliPath;
}
