import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';

export function codexCommand({ platform = process.platform, env = process.env } = {}) {
  const override = env.OPENPENCIL_CODEX_BIN;
  if (platform !== 'win32') return { binary: override || 'codex', prefix: [] };
  const found = override ? [override] : (spawnSync('where.exe', ['codex'], { encoding: 'utf8', env }).stdout || '').trim().split(/\r?\n/);
  const native = found.find(path => /\.exe$/i.test(path) && existsSync(path));
  if (native) return { binary: native, prefix: [] };
  // npm's Windows .cmd shim cannot be executed without cmd.exe. Invoke its
  // Node entry point directly so paths containing spaces or shell characters
  // remain literal arguments, without shell interpolation.
  for (const path of found.filter(Boolean)) {
    for (const entry of [join(dirname(path), 'node_modules', '@openai', 'codex', 'bin', 'codex.js'),
      join(dirname(path), '..', '@openai', 'codex', 'bin', 'codex.js')]) {
      if (existsSync(entry)) return { binary: process.execPath, prefix: [entry] };
    }
  }
  throw new Error('Codex CLI is missing. Install @openai/codex or set OPENPENCIL_CODEX_BIN to its native codex.exe.');
}

export function runCodex(command, { cwd, quiet = false } = {}) {
  const { binary, prefix } = codexCommand();
  const result = spawnSync(binary, [...prefix, ...command], { cwd, stdio: quiet ? 'pipe' : 'inherit', timeout: 180_000 });
  if (result.error?.code === 'ENOENT') throw new Error('Codex CLI is missing from PATH. Install the current official @openai/codex CLI, then run setup again.');
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`Codex ${command.slice(0, 3).join(' ')} failed. A recent Codex CLI with plugin support is required.`);
}
