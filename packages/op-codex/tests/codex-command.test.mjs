import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { codexCommand } from '../scripts/codex-command.mjs';
import { assertSupportedLibc } from '../plugins/openpencil/scripts/runtime-install.mjs';

test('Windows npm Codex entry points preserve paths with spaces and shell characters as literal arguments', async t => {
  const root = await mkdtemp(join(tmpdir(), 'openpencil & codex '));
  t.after(() => rm(root, { recursive: true, force: true }));
  const entry = join(root, 'node_modules', '@openai', 'codex', 'bin', 'codex.js');
  await mkdir(join(root, 'node_modules', '@openai', 'codex', 'bin'), { recursive: true });
  await writeFile(entry, '// test fixture');
  assert.deepEqual(codexCommand({ platform: 'win32', env: { OPENPENCIL_CODEX_BIN: join(root, 'codex.cmd') } }),
    { binary: process.execPath, prefix: [entry] });
  const native = join(root, 'codex.exe'); await writeFile(native, 'fixture');
  assert.deepEqual(codexCommand({ platform: 'win32', env: { OPENPENCIL_CODEX_BIN: native } }), { binary: native, prefix: [] });
  assert.deepEqual(codexCommand({ platform: 'darwin', env: { OPENPENCIL_CODEX_BIN: '/custom/codex' } }), { binary: '/custom/codex', prefix: [] });
});

test('Published Linux runtime compatibility fails early for musl and old glibc', () => {
  assertSupportedLibc('linux-x64', '2.35'); assertSupportedLibc('linux-arm64', '2.39');
  assertSupportedLibc('linux-arm64', '3.0'); assertSupportedLibc('darwin-arm64', undefined);
  assert.throws(() => assertSupportedLibc('linux-x64', undefined), /musl/);
  assert.throws(() => assertSupportedLibc('linux-x64', '2.31'), /2\.35/);
  assert.throws(() => assertSupportedLibc('linux-arm64', '2.35'), /2\.39/);
});
