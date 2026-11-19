import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync } from 'node:fs';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import {
  SKILL_NAMES,
  inspectWorkBuddyRegistration,
  installWorkBuddyMcpConfig,
  installWorkBuddySkills,
  renderWorkBuddyConnectorConfig,
  resolveWorkBuddyConfigDir,
  uninstallWorkBuddyMcpConfig,
  uninstallWorkBuddySkills,
} from '../scripts/host-config.mjs';

test('resolveWorkBuddyConfigDir honors explicit arg, WORKBUDDY_CONFIG_DIR, and WORKBUDDY_DATA_FOLDER_NAME', () => {
  const fakeHome = resolve('fake-home');
  const customDir = resolve('custom-wb');
  const explicitDir = resolve('explicit-wb');
  assert.equal(resolveWorkBuddyConfigDir({ home: fakeHome, env: {} }), join(fakeHome, '.workbuddy'));
  assert.equal(
    resolveWorkBuddyConfigDir({ home: fakeHome, env: { WORKBUDDY_DATA_FOLDER_NAME: '.workbuddy-beta' } }),
    join(fakeHome, '.workbuddy-beta'),
  );
  assert.equal(
    resolveWorkBuddyConfigDir({ home: fakeHome, env: { WORKBUDDY_CONFIG_DIR: customDir } }),
    customDir,
  );
  assert.equal(
    resolveWorkBuddyConfigDir({ configDir: explicitDir, home: fakeHome, env: { WORKBUDDY_CONFIG_DIR: customDir } }),
    explicitDir,
  );
});

test('installWorkBuddyMcpConfig merges openpencil while preserving unrelated servers and keys, and uninstall removes only openpencil', async (t) => {
  const home = await mkdtemp(join(tmpdir(), 'op-wb-config-'));
  t.after(() => rm(home, { recursive: true, force: true }));
  const configDir = join(home, '.workbuddy');
  await mkdir(configDir, { recursive: true });
  await writeFile(
    join(configDir, 'mcp.json'),
    JSON.stringify({
      customFlag: true,
      mcpServers: {
        existing: { type: 'stdio', command: 'node', args: ['/existing.js'] },
      },
    }, null, 2),
  );

  const installed = await installWorkBuddyMcpConfig({ home, env: {} });
  assert.equal(installed.mcpPath, join(configDir, 'mcp.json'));
  const saved = JSON.parse(await readFile(installed.mcpPath, 'utf8'));
  assert.equal(saved.customFlag, true);
  assert.deepEqual(saved.mcpServers.existing, { type: 'stdio', command: 'node', args: ['/existing.js'] });
  assert.equal(saved.mcpServers.openpencil.type, 'stdio');
  assert.ok(saved.mcpServers.openpencil.command.endsWith(process.platform === 'win32' ? 'launch-mcp.cmd' : 'launch-mcp'));

  const inspected = await inspectWorkBuddyRegistration({ home, env: {} });
  assert.equal(inspected.registered, true);

  const uninstalled = await uninstallWorkBuddyMcpConfig({ home, env: {} });
  assert.equal(uninstalled.removed, true);
  const after = JSON.parse(await readFile(installed.mcpPath, 'utf8'));
  assert.equal(after.customFlag, true);
  assert.deepEqual(after.mcpServers.existing, { type: 'stdio', command: 'node', args: ['/existing.js'] });
  assert.equal(after.mcpServers.openpencil, undefined);
});

test('installWorkBuddySkills installs managed skills, refuses unmanaged overwrite, and cleanly uninstalls only managed skills', async (t) => {
  const home = await mkdtemp(join(tmpdir(), 'op-wb-skills-'));
  t.after(() => rm(home, { recursive: true, force: true }));

  const first = await installWorkBuddySkills({ home, env: {} });
  assert.equal(first.installed.length, SKILL_NAMES.length);
  for (const name of SKILL_NAMES) {
    assert.ok(existsSync(join(first.skillsDir, name, 'SKILL.md')));
  }

  // Re-installing managed skills succeeds idempotently.
  const second = await installWorkBuddySkills({ home, env: {}, version: '0.1.1' });
  assert.equal(second.installed.length, SKILL_NAMES.length);

  // Uninstalling removes managed skills.
  const removed = await uninstallWorkBuddySkills({ home, env: {} });
  assert.deepEqual(removed.removed, [...SKILL_NAMES]);
  for (const name of SKILL_NAMES) {
    assert.equal(existsSync(join(first.skillsDir, name)), false);
  }

  // An unmanaged user skill directory must never be overwritten or deleted.
  const userSkill = join(first.skillsDir, 'openpencil-design');
  await mkdir(userSkill, { recursive: true });
  await writeFile(join(userSkill, 'SKILL.md'), '# custom user skill\n');
  await assert.rejects(() => installWorkBuddySkills({ home, env: {} }), /Refusing to overwrite existing unmanaged skill/);
  const noopRemove = await uninstallWorkBuddySkills({ home, env: {} });
  assert.deepEqual(noopRemove.removed, []);
  assert.ok(existsSync(join(userSkill, 'SKILL.md')));
});

test('renderWorkBuddyConnectorConfig outputs valid stdio mcpServers snippet', () => {
  const config = renderWorkBuddyConnectorConfig({ platform: 'darwin' });
  assert.equal(config.mcpServers.openpencil.type, 'stdio');
  assert.ok(config.mcpServers.openpencil.command.endsWith(join('scripts', 'launch-mcp')));
  const winConfig = renderWorkBuddyConnectorConfig({ platform: 'win32' });
  assert.ok(winConfig.mcpServers.openpencil.command.endsWith(join('scripts', 'launch-mcp.cmd')));
});
