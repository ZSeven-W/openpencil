import { mkdtemp, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { activatePackageRuntime, ensureRuntime } from '../plugins/openpencil/scripts/runtime-install.mjs';
import { CanvasController } from '../plugins/openpencil/scripts/canvas-controller.mjs';
import { EditorSession } from '../plugins/openpencil/scripts/editor-session.mjs';
import { registerNode } from './node-registration.mjs';
import { verifyPluginHost } from './plugin-host-check.mjs';
import {
  installWorkBuddyMcpConfig,
  installWorkBuddySkills,
  renderWorkBuddyConnectorConfig,
  uninstallWorkBuddyMcpConfig,
  uninstallWorkBuddySkills,
} from './host-config.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const pkg = JSON.parse(await readFile(new URL('../package.json', import.meta.url), 'utf8'));

export async function verifyRuntime(runtimeRoot) {
  const dir = await mkdtemp(join(tmpdir(), 'openpencil-workbuddy-check-'));
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
    skipSkills: false,
    noMcp: false,
    printConfig: false,
    uninstall: false,
    configDir: undefined,
  };
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === '--runtime-only') options.runtimeOnly = true;
    else if (arg === '--repair-runtime') options.repairRuntime = true;
    else if (arg === '--skip-skills') options.skipSkills = true;
    else if (arg === '--no-mcp') options.noMcp = true;
    else if (arg === '--print-config') options.printConfig = true;
    else if (arg === '--uninstall') options.uninstall = true;
    else if (arg === '--config-dir' && args[i + 1]) options.configDir = args[++i];
    else {
      throw new Error('Usage: workbuddy-openpencil setup [--runtime-only] [--repair-runtime] [--config-dir PATH] [--skip-skills] [--no-mcp] [--print-config] [--uninstall]');
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
      console.log(JSON.stringify(renderWorkBuddyConnectorConfig({ pluginRoot }), null, 2));
      return;
    }
    if (options.uninstall) {
      const mcp = await uninstallWorkBuddyMcpConfig({ configDir: options.configDir });
      const skills = await uninstallWorkBuddySkills({ configDir: options.configDir });
      console.log(JSON.stringify({ ok: true, uninstalled: true, mcp, skills }, null, 2));
      return;
    }
    // Verify the atomic platform runtime before modifying WorkBuddy config;
    // a missing or tampered runtime must never leave an apparently ready connector.
    const runtime = options.repairRuntime ? await activatePackageRuntime() : await ensureRuntime();
    await verifyRuntime(runtime.root);
    let mcp = null;
    let skills = null;
    let pluginHost = null;
    if (!options.runtimeOnly) {
      await registerNode();
      if (!options.noMcp) mcp = await installWorkBuddyMcpConfig({ configDir: options.configDir, pluginRoot });
      if (!options.skipSkills) {
        skills = await installWorkBuddySkills({ configDir: options.configDir, pluginRoot, version: pkg.version });
      }
      pluginHost = await verifyPluginHost({ configDir: options.configDir, pluginRoot });
    }
    console.log(JSON.stringify({
      ok: true,
      runtime: runtime.version,
      source: runtime.source ?? 'custom',
      platform: `${process.platform}-${process.arch}`,
      verified: ['panel', 'native-edit', 'save', 'reopen'],
      connectorInstalled: !options.runtimeOnly && !options.noMcp,
      mcp,
      skills,
      pluginHost,
      next: 'Open WorkBuddy -> 连接器 (Connectors), approve Trust/Enable on "openpencil", then start a new chat and call open_canvas.',
    }, null, 2));
  } catch (error) {
    console.error(`OpenPencil WorkBuddy setup failed: ${error.message}`);
    process.exitCode = 1;
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await runSetup(process.argv.slice(2));
}
