import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync } from 'node:fs';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import {
  PLUGIN_ID,
  SKILL_NAMES,
  inspectQoderRegistration,
  installQoderMcpSettings,
  installQoderPlugin,
  installQoderSkills,
  renderQoderConnectorConfig,
  resolveQoderConfigDir,
  uninstallQoderMcpSettings,
  uninstallQoderPlugin,
  uninstallQoderSkills,
} from '../scripts/host-config.mjs';

test('resolveQoderConfigDir honors Qoder env precedence and auto-detects ~/.qoder and ~/.qoder-cn', async (t) => {
  const home = await mkdtemp(join(tmpdir(), 'op-qoder-home-'));
  t.after(() => rm(home, { recursive: true, force: true }));

  assert.equal(resolveQoderConfigDir({ home, env: {} }), join(home, '.qoder'));

  await mkdir(join(home, '.qoder-cn'), { recursive: true });
  assert.equal(resolveQoderConfigDir({ home, env: {} }), join(home, '.qoder-cn'));

  await mkdir(join(home, '.qoder'), { recursive: true });
  assert.equal(resolveQoderConfigDir({ home, env: {} }), join(home, '.qoder'));

  const cliHome = resolve('qoder-cli-home');
  const cnCfg = resolve('qoder-cn-cfg');
  const globalCfg = resolve('qoder-global-cfg');
  const explicitCfg = resolve('qoder-explicit-cfg');
  assert.equal(
    resolveQoderConfigDir({ home, env: { QODER_CLI_HOME: cliHome, QODER_CONFIG_DIR_NAME: '.qoder-dev' } }),
    join(cliHome, '.qoder-dev'),
  );
  assert.equal(resolveQoderConfigDir({ home, env: { QODERCN_CONFIG_DIR: cnCfg } }), cnCfg);
  assert.equal(resolveQoderConfigDir({ home, env: { QODER_CONFIG_DIR: globalCfg, QODERCN_CONFIG_DIR: cnCfg } }), globalCfg);
  assert.equal(resolveQoderConfigDir({ configDir: explicitCfg, home, env: { QODER_CONFIG_DIR: globalCfg } }), explicitCfg);
});

test('installQoderPlugin registers openpencil@local in installed_plugins_v2.json and enabledPlugins while preserving unrelated entries', async (t) => {
  const home = await mkdtemp(join(tmpdir(), 'op-qoder-plugin-'));
  t.after(() => rm(home, { recursive: true, force: true }));
  const configDir = join(home, '.qoder');
  await mkdir(join(configDir, 'plugins'), { recursive: true });
  await writeFile(
    join(configDir, 'plugins', 'installed_plugins_v2.json'),
    JSON.stringify({
      version: 2,
      plugins: {
        'other-plugin@local': [{ scope: 'user', installPath: '/tmp/other', version: '1.0.0' }],
      },
    }, null, 2),
  );
  await writeFile(
    join(configDir, 'settings.json'),
    JSON.stringify({
      theme: 'dark',
      enabledPlugins: { 'other-plugin@local': true },
    }, null, 2),
  );

  const installed = await installQoderPlugin({ home, env: {}, version: '0.1.0' });
  assert.equal(installed.pluginId, PLUGIN_ID);
  assert.ok(existsSync(join(installed.installPath, '.qoder-plugin', 'plugin.json')));
  assert.ok(existsSync(join(installed.installPath, '.mcp.json')));

  const registry = JSON.parse(await readFile(installed.pluginsRegistryPath, 'utf8'));
  assert.equal(registry.version, 2);
  assert.deepEqual(registry.plugins['other-plugin@local'], [{ scope: 'user', installPath: '/tmp/other', version: '1.0.0' }]);
  assert.equal(registry.plugins[PLUGIN_ID][0].installPath, installed.installPath);

  const settings = JSON.parse(await readFile(installed.settingsPath, 'utf8'));
  assert.equal(settings.theme, 'dark');
  assert.equal(settings.enabledPlugins['other-plugin@local'], true);
  assert.equal(settings.enabledPlugins[PLUGIN_ID], true);

  const inspected = await inspectQoderRegistration({ home, env: {} });
  assert.equal(inspected.pluginRegistered, true);
  assert.equal(inspected.pluginEnabled, true);

  const removed = await uninstallQoderPlugin({ home, env: {} });
  assert.equal(removed.registryRemoved, true);
  assert.equal(removed.settingRemoved, true);
  assert.equal(existsSync(installed.installPath), false);

  const afterRegistry = JSON.parse(await readFile(installed.pluginsRegistryPath, 'utf8'));
  assert.equal(afterRegistry.plugins[PLUGIN_ID], undefined);
  assert.ok(afterRegistry.plugins['other-plugin@local']);

  const afterSettings = JSON.parse(await readFile(installed.settingsPath, 'utf8'));
  assert.equal(afterSettings.enabledPlugins[PLUGIN_ID], undefined);
  assert.equal(afterSettings.enabledPlugins['other-plugin@local'], true);

  // Refuses overwriting an unmanaged plugin cache directory.
  await mkdir(installed.installPath, { recursive: true });
  await writeFile(join(installed.installPath, 'custom.txt'), 'keep');
  await assert.rejects(() => installQoderPlugin({ home, env: {}, version: '0.1.0' }), /Refusing to overwrite existing unmanaged plugin/);
});

test('installQoderMcpSettings and installQoderSkills safely merge and uninstall without clobbering user entries', async (t) => {
  const home = await mkdtemp(join(tmpdir(), 'op-qoder-settings-'));
  t.after(() => rm(home, { recursive: true, force: true }));
  const configDir = join(home, '.qoder');
  await mkdir(configDir, { recursive: true });
  await writeFile(
    join(configDir, 'settings.json'),
    JSON.stringify({ mcpServers: { existing: { command: 'node', args: ['a.js'] } } }, null, 2),
  );

  const mcp = await installQoderMcpSettings({ home, env: {} });
  const saved = JSON.parse(await readFile(mcp.settingsPath, 'utf8'));
  assert.deepEqual(saved.mcpServers.existing, { command: 'node', args: ['a.js'] });
  assert.equal(saved.mcpServers.openpencil.type, 'stdio');

  const skills = await installQoderSkills({ home, env: {} });
  assert.equal(skills.installed.length, SKILL_NAMES.length);

  const removedMcp = await uninstallQoderMcpSettings({ home, env: {} });
  assert.equal(removedMcp.removed, true);
  const removedSkills = await uninstallQoderSkills({ home, env: {} });
  assert.deepEqual(removedSkills.removed, [...SKILL_NAMES]);
});

test('renderQoderConnectorConfig outputs valid stdio mcpServers snippet', () => {
  const config = renderQoderConnectorConfig({ platform: 'darwin' });
  assert.equal(config.mcpServers.openpencil.type, 'stdio');
  assert.ok(config.mcpServers.openpencil.command.endsWith(join('scripts', 'launch-mcp')));
  const winConfig = renderQoderConnectorConfig({ platform: 'win32' });
  assert.ok(winConfig.mcpServers.openpencil.command.endsWith(join('scripts', 'launch-mcp.cmd')));
});
