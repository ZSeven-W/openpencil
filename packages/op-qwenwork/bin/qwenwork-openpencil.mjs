#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

if (Number(process.versions.node.split('.')[0]) < 22) {
  console.error('OpenPencil requires Node.js 22 or later.');
  process.exit(1);
}

const packageRoot = fileURLToPath(new URL('../', import.meta.url));
const pkg = JSON.parse(await readFile(new URL('../package.json', import.meta.url), 'utf8'));

function printHelp() {
  console.log(
    `qwenwork-openpencil v${pkg.version}\n\n`
    + 'Usage: qwenwork-openpencil <command> [options]\n\n'
    + 'Commands:\n'
    + '  setup [options]                       Verify runtime, register QwenWork MCP connector and install skills\n'
    + '    --runtime-only                      Verify runtime without editing QwenWork config or skills\n'
    + '    --repair-runtime                    Re-activate the atomic platform runtime package\n'
    + '    --config-dir <path>                 Override QwenWork config dir (default: auto-detect ~/.qwenworkcn)\n'
    + '    --skip-skills                       Register MCP connector without installing user skills\n'
    + '    --no-mcp                            Install skills without modifying mcp.json\n'
    + '    --pack-plugin [path]                Also generate a QwenWork .zip plugin bundle for UI import\n'
    + '  uninstall [--config-dir <path>]       Remove the openpencil connector and managed skills from QwenWork\n'
    + '  print-config                          Print copy-pasteable mcpServers JSON for QwenWork Connector UI\n'
    + '  pack-plugin [--output <path.zip>]     Build a ready-to-import QwenWork plugin .zip bundle\n'
    + '  canvas [--file /abs/path.op]          Open the persistent terminal canvas\n'
    + '  host-check [--config-dir <path>]      Verify QwenWork MCP launcher, tools, UI resource, and registration\n'
    + '  doctor [--port PORT]                  Check connectivity to an existing desktop OpenPencil MCP server\n'
    + '  configure-runtime /abs/runtime/root   Select a custom complete Rust editor runtime\n'
    + '  --version, -v                         Show package version\n'
    + '  --help, -h                            Show this help message',
  );
}

const [command, ...args] = process.argv.slice(2);

if (!command || command === '--help' || command === '-h' || command === 'help') {
  printHelp();
} else if (command === '--version' || command === '-v') {
  console.log(pkg.version);
} else if (command === 'setup') {
  const { runSetup } = await import('../scripts/setup.mjs');
  await runSetup(args);
} else if (command === 'uninstall') {
  const { runSetup } = await import('../scripts/setup.mjs');
  await runSetup(['--uninstall', ...args]);
} else if (command === 'print-config') {
  const { runSetup } = await import('../scripts/setup.mjs');
  await runSetup(['--print-config', ...args]);
} else if (command === 'pack-plugin') {
  const { runPackPlugin } = await import('../scripts/pack-plugin.mjs');
  await runPackPlugin(args);
} else if (command === 'canvas') {
  const { runCanvas } = await import('../scripts/canvas.mjs');
  await runCanvas(args);
} else if (command === 'host-check') {
  const { runHostCheck } = await import('../scripts/plugin-host-check.mjs');
  await runHostCheck(args);
} else if (command === 'configure-runtime') {
  if (args.length !== 1) {
    console.error('Usage: qwenwork-openpencil configure-runtime /absolute/runtime/root');
    process.exitCode = 2;
  } else {
    const { configureRuntime } = await import('../plugins/openpencil/scripts/runtime-install.mjs');
    const runtime = await configureRuntime(resolve(args[0]), undefined, { source: 'custom' });
    console.log(`Verified and configured Rust OpenPencil ${runtime.version} (${process.platform}-${process.arch}).`);
  }
} else if (command === 'doctor') {
  const bridge = resolve(packageRoot, 'plugins/openpencil/scripts/mcp-bridge.mjs');
  const result = spawnSync(process.execPath, [bridge, '--doctor', ...args], { stdio: 'inherit' });
  process.exitCode = result.status ?? 1;
} else {
  console.error(`Unknown command: ${command}\nRun "qwenwork-openpencil --help" for usage.`);
  process.exitCode = 2;
}
