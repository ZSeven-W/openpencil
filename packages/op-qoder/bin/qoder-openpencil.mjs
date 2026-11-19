#!/usr/bin/env node
import { readFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const pkg = JSON.parse(await readFile(join(packageRoot, 'package.json'), 'utf8'));

function printHelp() {
  console.log(`qoder-openpencil v${pkg.version}

Usage: qoder-openpencil <command> [options]

Commands:
  setup [options]                       Verify runtime and install OpenPencil plugin into Qoder
    --runtime-only                      Verify runtime without editing Qoder plugin cache or settings
    --repair-runtime                    Re-activate the atomic platform runtime package
    --config-dir <path>                 Override Qoder config dir (default: auto-detect ~/.qoder or ~/.qoder-cn)
    --mcp-settings                      Register via settings.json mcpServers + skills instead of plugin cache
    --pack-plugin [path]                Also generate a Qoder local plugin .zip (root .qoder-plugin/plugin.json)
  uninstall [--config-dir <path>]       Remove the OpenPencil plugin and managed skills from Qoder
  print-config                          Print copy-pasteable mcpServers JSON for Qoder IDE/CLI settings.json
  pack-plugin [--output <path.zip>]     Build a Qoder-compatible local plugin .zip bundle
  canvas [--file /abs/path.op]          Open the persistent terminal canvas
  host-check [--config-dir <path>]      Verify Qoder MCP launcher, qoder/ui entrypoints, and registration
  doctor [--port PORT]                  Check connectivity to an existing desktop OpenPencil MCP server
  configure-runtime /abs/runtime/root   Select a custom complete Rust editor runtime
  --version, -v                         Show package version
  --help, -h                            Show this help message`);
}

export async function runCli(args = process.argv.slice(2)) {
  const [command, ...rest] = args;
  if (!command || command === '--help' || command === '-h' || command === 'help') {
    printHelp();
    return;
  }
  if (command === '--version' || command === '-v') {
    console.log(pkg.version);
    return;
  }
  if (command === 'setup') {
    const { runSetup } = await import(pathToFileURL(join(packageRoot, 'scripts/setup.mjs')).href);
    await runSetup(rest);
    return;
  }
  if (command === 'uninstall') {
    const { runSetup } = await import(pathToFileURL(join(packageRoot, 'scripts/setup.mjs')).href);
    await runSetup(['--uninstall', ...rest]);
    return;
  }
  if (command === 'print-config') {
    const { runSetup } = await import(pathToFileURL(join(packageRoot, 'scripts/setup.mjs')).href);
    await runSetup(['--print-config', ...rest]);
    return;
  }
  if (command === 'pack-plugin') {
    const { runPackPlugin } = await import(pathToFileURL(join(packageRoot, 'scripts/pack-plugin.mjs')).href);
    await runPackPlugin(rest);
    return;
  }
  if (command === 'host-check') {
    const { runHostCheck } = await import(pathToFileURL(join(packageRoot, 'scripts/plugin-host-check.mjs')).href);
    await runHostCheck(rest);
    return;
  }
  if (command === 'canvas') {
    const { runCanvas } = await import(pathToFileURL(join(packageRoot, 'scripts/canvas.mjs')).href);
    await runCanvas(rest);
    return;
  }
  if (command === 'configure-runtime') {
    const { runConfigureRuntime } = await import(pathToFileURL(join(packageRoot, 'scripts/configure-runtime.mjs')).href);
    await runConfigureRuntime(rest);
    return;
  }
  if (command === 'doctor') {
    const { spawnSync } = await import('node:child_process');
    const bridge = join(packageRoot, 'plugins/openpencil/scripts/mcp-bridge.mjs');
    const result = spawnSync(process.execPath, [bridge, '--doctor', ...rest], { stdio: 'inherit' });
    process.exitCode = result.status ?? 1;
    return;
  }
  console.error(`Unknown command: ${command}\nRun "qoder-openpencil --help" for usage.`);
  process.exitCode = 1;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await runCli(process.argv.slice(2));
}
