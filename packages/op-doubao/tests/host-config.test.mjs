import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync } from 'node:fs';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  SKILL_NAMES,
  exportDoubaoConnectorConfig,
  inspectDoubaoRegistration,
  installDoubaoMcpConfig,
  installDoubaoSkills,
  renderDoubaoConnectorConfig,
  renderDoubaoConnectorForm,
  uninstallDoubaoMcpConfig,
  uninstallDoubaoSkills,
} from '../scripts/host-config.mjs';

test('renderDoubaoConnectorConfig and renderDoubaoConnectorForm output valid stdio connector metadata', () => {
  const config = renderDoubaoConnectorConfig({ platform: 'darwin' });
  assert.equal(config.mcpServers.openpencil.type, 'stdio');
  assert.equal(config.mcpServers.openpencil.disabled, false);
  assert.ok(config.mcpServers.openpencil.command.endsWith(join('scripts', 'launch-mcp')));

  const winConfig = renderDoubaoConnectorConfig({ platform: 'win32' });
  assert.ok(winConfig.mcpServers.openpencil.command.endsWith(join('scripts', 'launch-mcp.cmd')));

  const form = renderDoubaoConnectorForm({ platform: 'darwin' });
  assert.equal(form.fields.name, 'openpencil');
  assert.equal(form.fields.transport, 'stdio');
  assert.match(form.text, /豆包电脑版/);
  assert.match(form.text, /launch-mcp/);
});

test('exportDoubaoConnectorConfig writes atomic ready-to-paste connector JSON in state exports', async (t) => {
  const stateDir = await mkdtemp(join(tmpdir(), 'op-doubao-export-'));
  t.after(() => rm(stateDir, { recursive: true, force: true }));

  const exported = await exportDoubaoConnectorConfig({ stateDir });
  assert.ok(existsSync(exported.outputPath));
  const saved = JSON.parse(await readFile(exported.outputPath, 'utf8'));
  assert.equal(saved.mcpServers.openpencil.type, 'stdio');
});

test('installDoubaoMcpConfig merges openpencil while preserving unrelated servers and uninstall removes only openpencil', async (t) => {
  const dir = await mkdtemp(join(tmpdir(), 'op-doubao-mcp-'));
  t.after(() => rm(dir, { recursive: true, force: true }));
  const mcpConfigPath = join(dir, 'mcp.json');
  await writeFile(
    mcpConfigPath,
    JSON.stringify({
      customOption: 1,
      mcpServers: {
        existing: { type: 'stdio', command: 'node', args: ['/existing.js'] },
      },
    }, null, 2),
  );

  const installed = await installDoubaoMcpConfig({ mcpConfigPath });
  assert.equal(installed.mcpPath, mcpConfigPath);
  const saved = JSON.parse(await readFile(mcpConfigPath, 'utf8'));
  assert.equal(saved.customOption, 1);
  assert.deepEqual(saved.mcpServers.existing, { type: 'stdio', command: 'node', args: ['/existing.js'] });
  assert.equal(saved.mcpServers.openpencil.type, 'stdio');

  const inspected = await inspectDoubaoRegistration({ mcpConfigPath });
  assert.equal(inspected.registered, true);

  const removed = await uninstallDoubaoMcpConfig({ mcpConfigPath });
  assert.equal(removed.removed, true);
  const after = JSON.parse(await readFile(mcpConfigPath, 'utf8'));
  assert.equal(after.customOption, 1);
  assert.deepEqual(after.mcpServers.existing, { type: 'stdio', command: 'node', args: ['/existing.js'] });
  assert.equal(after.mcpServers.openpencil, undefined);
});

test('installDoubaoSkills installs managed skills, refuses unmanaged overwrite, and cleanly uninstalls only managed skills', async (t) => {
  const dir = await mkdtemp(join(tmpdir(), 'op-doubao-skills-'));
  t.after(() => rm(dir, { recursive: true, force: true }));
  const skillsDir = join(dir, 'skills');

  const first = await installDoubaoSkills({ skillsDir });
  assert.equal(first.installed.length, SKILL_NAMES.length);
  for (const name of SKILL_NAMES) {
    assert.ok(existsSync(join(skillsDir, name, 'SKILL.md')));
  }

  const removed = await uninstallDoubaoSkills({ skillsDir });
  assert.deepEqual(removed.removed, [...SKILL_NAMES]);

  const userSkill = join(skillsDir, 'openpencil-design');
  await mkdir(userSkill, { recursive: true });
  await writeFile(join(userSkill, 'SKILL.md'), '# user owned skill\n');
  await assert.rejects(() => installDoubaoSkills({ skillsDir }), /Refusing to overwrite existing unmanaged skill/);
  const noop = await uninstallDoubaoSkills({ skillsDir });
  assert.deepEqual(noop.removed, []);
  assert.ok(existsSync(join(userSkill, 'SKILL.md')));
});
