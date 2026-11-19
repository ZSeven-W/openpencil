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
    `codex-openpencil v${pkg.version}\n\n` +
    'Usage: codex-openpencil <command> [options]\n\n' +
    'Commands:\n' +
    '  setup [--runtime-only] [--repair-runtime]  Verify runtime, register Codex plugin and check host\n' +
    '  canvas [--file /abs/path.op]               Open the persistent terminal canvas\n' +
    '  host-check                                 Verify Codex plugin loader and canvas controls\n' +
    '  doctor [--port PORT]                       Check connectivity to an existing desktop OpenPencil MCP server\n' +
    '  configure-runtime /abs/runtime/root        Select a custom complete Rust editor runtime\n' +
    '  --version, -v                              Show package version\n' +
    '  --help, -h                                 Show this help message',
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
} else if (command === 'canvas') {
  const { runCanvas } = await import('../scripts/canvas.mjs');
  await runCanvas(args);
} else if (command === 'host-check') {
  const { verifyPluginHost } = await import('../scripts/plugin-host-check.mjs');
  try {
    console.log(JSON.stringify(await verifyPluginHost(), null, 2));
  } catch (error) {
    console.error(`OpenPencil host check failed: ${error.message}`);
    process.exitCode = 1;
  }
} else if (command === 'configure-runtime') {
  if (args.length !== 1) {
    console.error('Usage: codex-openpencil configure-runtime /absolute/runtime/root');
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
  console.error(`Unknown command: ${command}\nRun "codex-openpencil --help" for usage.`);
  process.exitCode = 2;
}
