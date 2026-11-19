import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync } from 'node:fs';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import {
  SKILL_NAMES,
  inspectQwenWorkRegistration,
  installQwenWorkMcpConfig,
  installQwenWorkSkills,
  renderQwenWorkConnectorConfig,
  resolveQwenWorkConfigDir,
  uninstallQwenWorkMcpConfig,
  uninstallQwenWorkSkills,
} from '../scripts/host-config.mjs';

test('resolveQwenWorkConfigDir honors explicit arg, env overrides, and auto-detects QwenWork/QoderWork dirs', async (t) => {
  const home = await mkdtemp(join(tmpdir(), 'op-qw-home-'));
  t.after(() => rm(home, { recursive: true, force: true }));

  // Default falls back to .qwenworkcn when none exist.
  assert.equal(resolveQwenWorkConfigDir({ home, env: {} }), join(home, '.qwenworkcn'));

  // Auto-detects .qoderwork when it is the only existing directory.
  await mkdir(join(home, '.qoderwork'), { recursive: true });
  assert.equal(resolveQwenWorkConfigDir({ home, env: {} }), join(home, '.qoderwork'));

  // Prefers .qwenwork over .qoderwork, and .qwenworkcn over .qwenwork.
  await mkdir(join(home, '.qwenwork'), { recursive: true });
  assert.equal(resolveQwenWorkConfigDir({ home, env: {} }), join(home, '.qwenwork'));
  await mkdir(join(home, '.qwenworkcn'), { recursive: true });
  assert.equal(resolveQwenWorkConfigDir({ home, env: {} }), join(home, '.qwenworkcn'));

  // Explicit env vars and configDir take precedence.
  const customHome = resolve('custom-qw-home');
  const customCfg = resolve('custom-qw-cfg');
  const explicitDir = resolve('explicit-qw');
  assert.equal(resolveQwenWorkConfigDir({ home, env: { QWENWORK_HOME: customHome } }), customHome);
  assert.equal(
    resolveQwenWorkConfigDir({ home, env: { QWENWORK_CONFIG_DIR: customCfg, QWENWORK_HOME: customHome } }),
    customCfg,
  );
  assert.equal(
    resolveQwenWorkConfigDir({ configDir: explicitDir, home, env: { QWENWORK_CONFIG_DIR: customCfg } }),
    explicitDir,
  );
});

test('installQwenWorkMcpConfig merges openpencil while preserving cliConnectors and unrelated servers', async (t) => {
  const home = await mkdtemp(join(tmpdir(), 'op-qw-config-'));
  t.after(() => rm(home, { recursive: true, force: true }));
  const configDir = join(home, '.qwenworkcn');
  await mkdir(configDir, { recursive: true });
  await writeFile(
    join(configDir, 'mcp.json'),
    JSON.stringify({
      cliConnectors: { sample: { enabled: true } },
      mcpServers: {
        existing: { type: 'stdio', command: 'node', args: ['/existing.js'] },
      },
    }, null, 2),
  );

  const installed = await installQwenWorkMcpConfig({ home, env: {} });
  assert.equal(installed.mcpPath, join(configDir, 'mcp.json'));
  const saved = JSON.parse(await readFile(installed.mcpPath, 'utf8'));
  assert.deepEqual(saved.cliConnectors, { sample: { enabled: true } });
  assert.deepEqual(saved.mcpServers.existing, { type: 'stdio', command: 'node', args: ['/existing.js'] });
  assert.equal(saved.mcpServers.openpencil.type, 'stdio');
  assert.ok(saved.mcpServers.openpencil.command.endsWith(process.platform === 'win32' ? 'launch-mcp.cmd' : 'launch-mcp'));

  const inspected = await inspectQwenWorkRegistration({ home, env: {} });
  assert.equal(inspected.registered, true);

  const uninstalled = await uninstallQwenWorkMcpConfig({ home, env: {} });
  assert.equal(uninstalled.removed, true);
  const after = JSON.parse(await readFile(installed.mcpPath, 'utf8'));
  assert.deepEqual(after.cliConnectors, { sample: { enabled: true } });
  assert.deepEqual(after.mcpServers.existing, { type: 'stdio', command: 'node', args: ['/existing.js'] });
  assert.equal(after.mcpServers.openpencil, undefined);
});

test('installQwenWorkSkills installs managed skills, refuses unmanaged overwrite, and cleanly uninstalls only managed skills', async (t) => {
  const home = await mkdtemp(join(tmpdir(), 'op-qw-skills-'));
  t.after(() => rm(home, { recursive: true, force: true }));

  const first = await installQwenWorkSkills({ home, env: {} });
  assert.equal(first.installed.length, SKILL_NAMES.length);
  for (const name of SKILL_NAMES) {
    assert.ok(existsSync(join(first.skillsDir, name, 'SKILL.md')));
  }

  const second = await installQwenWorkSkills({ home, env: {}, version: '0.1.1' });
  assert.equal(second.installed.length, SKILL_NAMES.length);

  const removed = await uninstallQwenWorkSkills({ home, env: {} });
  assert.deepEqual(removed.removed, [...SKILL_NAMES]);
  for (const name of SKILL_NAMES) {
    assert.equal(existsSync(join(first.skillsDir, name)), false);
  }

  const userSkill = join(first.skillsDir, 'openpencil-design');
  await mkdir(userSkill, { recursive: true });
  await writeFile(join(userSkill, 'SKILL.md'), '# custom user skill\n');
  await assert.rejects(() => installQwenWorkSkills({ home, env: {} }), /Refusing to overwrite existing unmanaged skill/);
  const noopRemove = await uninstallQwenWorkSkills({ home, env: {} });
  assert.deepEqual(noopRemove.removed, []);
  assert.ok(existsSync(join(userSkill, 'SKILL.md')));
});

test('renderQwenWorkConnectorConfig outputs valid stdio mcpServers snippet for UI paste', () => {
  const config = renderQwenWorkConnectorConfig({ platform: 'darwin' });
  assert.equal(config.mcpServers.openpencil.type, 'stdio');
  assert.ok(config.mcpServers.openpencil.command.endsWith(join('scripts', 'launch-mcp')));
  const winConfig = renderQwenWorkConnectorConfig({ platform: 'win32' });
  assert.ok(winConfig.mcpServers.openpencil.command.endsWith(join('scripts', 'launch-mcp.cmd')));
});
