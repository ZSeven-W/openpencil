#!/usr/bin/env node
import { readFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const pkg = JSON.parse(await readFile(join(packageRoot, 'package.json'), 'utf8'));

function printHelp() {
  console.log(`doubao-openpencil v${pkg.version}

Usage: doubao-openpencil <command> [options]

Commands:
  setup [options]                       Verify runtime, record Node 22+, run host-check, and export Doubao assets
    --runtime-only                      Verify runtime without exporting connector JSON or skill ZIP
    --repair-runtime                    Re-activate the atomic platform runtime package
    --mcp-config <path>                 Also merge mcpServers.openpencil into a local MCP JSON file
    --skills-dir <path>                 Also install managed OpenPencil skills into a local directory
    --skill-output <path.zip>           Override output path for the exported Doubao skill ZIP
  uninstall [options]                   Remove managed OpenPencil entries from --mcp-config / --skills-dir
  print-config                          Print copy-pasteable mcpServers JSON for Doubao Custom Connector
  print-form                            Print field-by-field values for Doubao Desktop manual stdio connector form
  pack-skill [options]                  Build an uploadable Doubao Desktop Skill .zip (SKILL.md + references)
    --skill <name>                      Skill name: openpencil-design (default) or openpencil-setup
    --flat                              Place SKILL.md directly at the ZIP root instead of <skill>/SKILL.md
    --output, -o <path.zip>             Output ZIP file path
  canvas [--file /abs/path.op]          Open the persistent terminal canvas
  host-check [--mcp-config <path>]      Verify Doubao MCP launcher, tools, and UI resource under stripped GUI PATH
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
  if (command === 'print-form') {
    const { runSetup } = await import(pathToFileURL(join(packageRoot, 'scripts/setup.mjs')).href);
    await runSetup(['--print-form', ...rest]);
    return;
  }
  if (command === 'pack-skill') {
    const { runPackSkill } = await import(pathToFileURL(join(packageRoot, 'scripts/pack-skill.mjs')).href);
    await runPackSkill(rest);
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
  console.error(`Unknown command: ${command}\nRun "doubao-openpencil --help" for usage.`);
  process.exitCode = 1;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await runCli(process.argv.slice(2));
}
