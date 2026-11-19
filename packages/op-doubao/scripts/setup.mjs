import { mkdtemp, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { activatePackageRuntime, ensureRuntime } from '../plugins/openpencil/scripts/runtime-install.mjs';
import { stateRoot } from '../plugins/openpencil/scripts/editor-runtime.mjs';
import { CanvasController } from '../plugins/openpencil/scripts/canvas-controller.mjs';
import { EditorSession } from '../plugins/openpencil/scripts/editor-session.mjs';
import { registerNode } from './node-registration.mjs';
import { verifyPluginHost } from './plugin-host-check.mjs';
import { packDoubaoSkill } from './pack-skill.mjs';
import {
  exportDoubaoConnectorConfig,
  installDoubaoMcpConfig,
  installDoubaoSkills,
  renderDoubaoConnectorConfig,
  renderDoubaoConnectorForm,
  resolveDoubaoMcpConfigPath,
  resolveDoubaoSkillsDir,
  uninstallDoubaoMcpConfig,
  uninstallDoubaoSkills,
} from './host-config.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const pkg = JSON.parse(await readFile(new URL('../package.json', import.meta.url), 'utf8'));

export async function verifyRuntime(runtimeRoot) {
  const dir = await mkdtemp(join(tmpdir(), 'openpencil-doubao-check-'));
  const filePath = join(dir, 'setup-check.op');
  const controller = new CanvasController({ runtimeRoot });
  try {
    await writeFile(filePath, JSON.stringify({
      version: '0.8.6',
      children: [{ type: 'frame', id: 'setup-check', name: 'Installation check', width: 400, height: 300, children: [] }],
    }));
    const opened = await controller.open({ filePath });
    if (!(await fetch(opened.structuredContent.panelUrl)).ok) throw new Error('Canvas panel did not load.');
    const reply = await controller.dispatch({
      jsonrpc: '2.0',
      id: 1,
      method: 'tools/call',
      params: { name: 'update_node', arguments: { nodeId: 'setup-check', data: { name: 'Verified installation' } } },
    });
    if (reply.error || reply.result?.isError) throw new Error('Native editor modification failed.');
    await controller.call('canvas_save', { filePath });
    if (!(await readFile(filePath, 'utf8')).includes('Verified installation')) throw new Error('Saved edit was missing.');
    await controller.close();
    const reopened = await EditorSession.start({ filePath, allowOrigin: 'http://127.0.0.1:1', runtimeRoot });
    try {
      if (!JSON.stringify((await reopened.document()).document).includes('Verified installation')) {
        throw new Error('Saved document did not reopen.');
      }
    } finally {
      await reopened.close();
    }
  } finally {
    await controller.close();
    await rm(dir, { recursive: true, force: true });
  }
}

function parseArgs(args) {
  const options = {
    runtimeOnly: false,
    repairRuntime: false,
    printConfig: false,
    printForm: false,
    uninstall: false,
    mcpConfigPath: undefined,
    skillsDir: undefined,
    skillOutput: undefined,
  };
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === '--runtime-only') options.runtimeOnly = true;
    else if (arg === '--repair-runtime') options.repairRuntime = true;
    else if (arg === '--print-config') options.printConfig = true;
    else if (arg === '--print-form') options.printForm = true;
    else if (arg === '--uninstall') options.uninstall = true;
    else if (arg === '--mcp-config' && args[i + 1]) options.mcpConfigPath = args[++i];
    else if (arg === '--skills-dir' && args[i + 1]) options.skillsDir = args[++i];
    else if (arg === '--skill-output' && args[i + 1]) options.skillOutput = args[++i];
    else {
      throw new Error('Usage: doubao-openpencil setup [--runtime-only] [--repair-runtime] [--mcp-config PATH] [--skills-dir PATH] [--skill-output PATH] [--print-config] [--print-form] [--uninstall]');
    }
  }
  return options;
}

export async function runSetup(args = process.argv.slice(2)) {
  if (Number(process.versions.node.split('.')[0]) < 22) throw new Error('OpenPencil requires Node.js 22 or later.');
  const options = parseArgs(args);
  const pluginRoot = join(root, 'plugins', 'openpencil');

  try {
    if (options.printConfig) {
      console.log(JSON.stringify(renderDoubaoConnectorConfig({ pluginRoot }), null, 2));
      return;
    }
    if (options.printForm) {
      console.log(renderDoubaoConnectorForm({ pluginRoot }).text);
      return;
    }
    if (options.uninstall) {
      const mcp = await uninstallDoubaoMcpConfig({ mcpConfigPath: options.mcpConfigPath });
      const skills = await uninstallDoubaoSkills({ skillsDir: options.skillsDir });
      console.log(JSON.stringify({ ok: true, uninstalled: true, mcp, skills }, null, 2));
      return;
    }
    const runtime = options.repairRuntime ? await activatePackageRuntime() : await ensureRuntime();
    await verifyRuntime(runtime.root);
    let exportedConnector = null;
    let exportedSkill = null;
    let mcp = null;
    let skills = null;
    let pluginHost = null;
    if (!options.runtimeOnly) {
      await registerNode();
      exportedConnector = await exportDoubaoConnectorConfig({ pluginRoot });
      const defaultSkillOutput = join(stateRoot(), 'exports', 'openpencil-design-doubao-skill.zip');
      exportedSkill = await packDoubaoSkill({
        pluginRoot,
        skillName: 'openpencil-design',
        outputPath: options.skillOutput || defaultSkillOutput,
      });
      if (resolveDoubaoMcpConfigPath({ mcpConfigPath: options.mcpConfigPath })) {
        mcp = await installDoubaoMcpConfig({ mcpConfigPath: options.mcpConfigPath, pluginRoot });
      }
      if (resolveDoubaoSkillsDir({ skillsDir: options.skillsDir })) {
        skills = await installDoubaoSkills({ skillsDir: options.skillsDir, pluginRoot, version: pkg.version });
      }
      pluginHost = await verifyPluginHost({ mcpConfigPath: options.mcpConfigPath, skillsDir: options.skillsDir, pluginRoot });
    }
    console.log(JSON.stringify({
      ok: true,
      runtime: runtime.version,
      source: runtime.source ?? 'custom',
      platform: `${process.platform}-${process.arch}`,
      verified: ['panel', 'native-edit', 'save', 'reopen'],
      exportedConnector: exportedConnector?.outputPath ?? null,
      exportedSkill,
      mcp,
      skills,
      pluginHost,
      next: 'In Doubao Desktop (豆包电脑版), open 技能·连接器·伙伴 -> 连接器 -> 手动添加 (stdio) using "doubao-openpencil print-config" or "doubao-openpencil print-form", then upload the exported skill ZIP under 我的技能 -> 新建 -> 上传技能.',
    }, null, 2));
  } catch (error) {
    console.error(`OpenPencil Doubao setup failed: ${error.message}`);
    process.exitCode = 1;
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await runSetup(process.argv.slice(2));
}
