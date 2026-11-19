import { existsSync } from 'node:fs';
import { mkdir, mkdtemp, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { activatePackageRuntime, ensureRuntime } from '../plugins/openpencil/scripts/runtime-install.mjs';
import { CanvasController } from '../plugins/openpencil/scripts/canvas-controller.mjs';
import { EditorSession } from '../plugins/openpencil/scripts/editor-session.mjs';
import { runCodex } from './codex-command.mjs';
import { registerNode } from './node-registration.mjs';
import { verifyPluginHost } from './plugin-host-check.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));

export async function verifyRuntime(runtimeRoot) {
  const dir = await mkdtemp(join(tmpdir(), 'openpencil-install-check-'));
  const filePath = join(dir, 'setup-check.op');
  const controller = new CanvasController({ runtimeRoot });
  try {
    await writeFile(filePath, JSON.stringify({ version: '0.8.5', children: [
      { type: 'frame', id: 'setup-check', name: 'Installation check', width: 400, height: 300, children: [] },
    ] }));
    const opened = await controller.open({ filePath });
    if (!(await fetch(opened.structuredContent.panelUrl)).ok) throw new Error('Canvas panel did not load.');
    const reply = await controller.dispatch({ jsonrpc: '2.0', id: 1, method: 'tools/call', params: {
      name: 'update_node', arguments: { nodeId: 'setup-check', data: { name: 'Verified installation' } },
    } });
    if (reply.error || reply.result?.isError) throw new Error('Native editor modification failed.');
    await controller.call('canvas_save', { filePath });
    if (!(await readFile(filePath, 'utf8')).includes('Verified installation')) throw new Error('Saved edit was missing.');
    await controller.close();
    const reopened = await EditorSession.start({ filePath, allowOrigin: 'http://127.0.0.1:1', runtimeRoot });
    try {
      if (!JSON.stringify((await reopened.document()).document).includes('Verified installation')) throw new Error('Saved document did not reopen.');
    } finally { await reopened.close(); }
  } finally { await controller.close(); await rm(dir, { recursive: true, force: true }); }
}

export async function runSetup(args = process.argv.slice(2)) {
  if (args.some(arg => !['--runtime-only', '--repair-runtime'].includes(arg))) {
    throw new Error('Usage: codex-openpencil setup [--runtime-only] [--repair-runtime]');
  }
  if (Number(process.versions.node.split('.')[0]) < 22) throw new Error('OpenPencil requires Node.js 22 or later.');
  try {
    if (!args.includes('--runtime-only')) {
      if (process.env.CODEX_HOME) await mkdir(resolve(process.env.CODEX_HOME), { recursive: true, mode: 0o700 });
      runCodex(['plugin', 'add', '--help'], { cwd: root, quiet: true });
    }
    // Verify the atomic platform runtime before registering the plugin; a
    // missing or tampered runtime must never leave an apparently ready plugin.
    const runtime = args.includes('--repair-runtime') ? await activatePackageRuntime() : await ensureRuntime();
    await verifyRuntime(runtime.root);
    let pluginHost;
    if (!args.includes('--runtime-only')) {
      await registerNode();
      const repoMarketplace = resolve(root, '../../.agents/plugins/marketplace.json');
      const marketplaceRoot = existsSync(repoMarketplace) ? resolve(root, '../..') : root;
      runCodex(['plugin', 'marketplace', 'add', marketplaceRoot], { cwd: marketplaceRoot });
      runCodex(['plugin', 'add', 'openpencil@zseven-openpencil'], { cwd: marketplaceRoot });
      pluginHost = await verifyPluginHost();
    }
    console.log(JSON.stringify({
      ok: true,
      runtime: runtime.version,
      source: runtime.source ?? 'custom',
      platform: `${process.platform}-${process.arch}`,
      verified: ['panel', 'native-edit', 'save', 'reopen'],
      pluginInstalled: !args.includes('--runtime-only'),
      pluginHost,
      next: 'Call canvas_open_browser and show its panelUrl in the Codex browser panel. Otherwise run codex-openpencil canvas (or npm run canvas in a checkout) in a persistent terminal to use the canvas now; reconnect the plugin or start a new chat to load its tools.',
    }, null, 2));
  } catch (error) {
    console.error(`OpenPencil setup failed: ${error.message}`);
    process.exitCode = 1;
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await runSetup(process.argv.slice(2));
}
