import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const cli = fileURLToPath(new URL('../bin/codex-openpencil.mjs', import.meta.url));

test('codex-openpencil CLI reports version, help and rejects unknown commands', async () => {
  const pkg = JSON.parse(await readFile(new URL('../package.json', import.meta.url), 'utf8'));

  const version = spawnSync(process.execPath, [cli, '--version'], { cwd: root, encoding: 'utf8' });
  assert.equal(version.status, 0, version.stderr);
  assert.equal(version.stdout.trim(), pkg.version);

  const help = spawnSync(process.execPath, [cli, '--help'], { cwd: root, encoding: 'utf8' });
  assert.equal(help.status, 0, help.stderr);
  assert.match(help.stdout, /Usage: codex-openpencil/);
  assert.match(help.stdout, /\bsetup\b/);
  assert.match(help.stdout, /\bcanvas\b/);
  assert.match(help.stdout, /\bhost-check\b/);

  const unknown = spawnSync(process.execPath, [cli, 'nonexistent-subcommand'], { cwd: root, encoding: 'utf8' });
  assert.equal(unknown.status, 2);
  assert.match(unknown.stderr, /Unknown command/);
});
