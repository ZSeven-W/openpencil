import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { CANVAS_URI } from '../plugins/openpencil/scripts/canvas-controller.mjs';
import { inspectQoderRegistration, launcherPathForPlugin } from './host-config.mjs';

const defaultPluginRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../plugins/openpencil');

/** Spawn the launcher under a minimal GUI-like PATH and verify MCP handshake, Qoder entrypoints, and UI resource. */
export async function probeLauncher({
  command,
  args = [],
  cwd,
  env = process.env,
  timeoutMs = 15_000,
} = {}) {
  const minimalPath = process.platform === 'win32'
    ? (env.SystemRoot ? `${env.SystemRoot}\\System32;${env.SystemRoot}` : env.PATH || '')
    : '/usr/bin:/bin';
  const isWindowsCmd = process.platform === 'win32' && /\.cmd$/i.test(command);
  const spawnCmd = isWindowsCmd ? (env.COMSPEC || 'cmd.exe') : command;
  const spawnArgs = isWindowsCmd ? ['/d', '/c', command, ...args] : args;
  const child = spawn(spawnCmd, spawnArgs, {
    cwd,
    env: {
      HOME: env.HOME,
      USERPROFILE: env.USERPROFILE,
      SystemRoot: env.SystemRoot,
      SystemDrive: env.SystemDrive,
      COMSPEC: env.COMSPEC,
      PATHEXT: env.PATHEXT,
      TMPDIR: env.TMPDIR,
      TEMP: env.TEMP,
      OPENPENCIL_QODER_STATE_DIR: env.OPENPENCIL_QODER_STATE_DIR,
      OPENPENCIL_EDITOR_RUNTIME_DIR: env.OPENPENCIL_EDITOR_RUNTIME_DIR,
      PATH: minimalPath,
    },
    stdio: ['pipe', 'pipe', 'pipe'],
  });
  let stderr = '';
  child.stderr.setEncoding('utf8');
  child.stderr.on('data', chunk => { stderr = (stderr + chunk).slice(-8192); });

  const replies = new Map();
  let buffer = '';
  child.stdout.setEncoding('utf8');
  child.stdout.on('data', chunk => {
    buffer += chunk;
    let newline;
    while ((newline = buffer.indexOf('\n')) !== -1) {
      const line = buffer.slice(0, newline).trim();
      buffer = buffer.slice(newline + 1);
      if (!line) continue;
      try {
        const message = JSON.parse(line);
        const waiter = replies.get(message.id);
        if (waiter) {
          replies.delete(message.id);
          clearTimeout(waiter.timer);
          waiter.resolve(message);
        }
      } catch { /* Handled by timeout if malformed. */ }
    }
  });

  const rpc = (id, method, params = {}) => new Promise((resolvePromise, rejectPromise) => {
    const timer = setTimeout(() => {
      replies.delete(id);
      rejectPromise(new Error(`Launcher timed out waiting for ${method}${stderr ? `: ${stderr.trim()}` : ''}`));
    }, timeoutMs);
    replies.set(id, { resolve: resolvePromise, reject: rejectPromise, timer });
    child.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', id, method, params })}\n`);
  });

  try {
    const init = await rpc(1, 'initialize', { protocolVersion: '2025-11-25', clientInfo: { name: 'qoder-host-check', version: '1.0.0' }, capabilities: {} });
    if (init.error || !init.result?.capabilities?.tools) {
      throw new Error(`MCP initialize failed: ${init.error?.message || 'missing capabilities'}`);
    }
    const toolsReply = await rpc(2, 'tools/list', {});
    const tools = toolsReply.result?.tools || [];
    const toolNames = tools.map(tool => tool.name);
    for (const required of ['open_canvas', 'canvas_open_browser', 'canvas_status', 'canvas_selection', 'canvas_save', 'canvas_import_image']) {
      if (!toolNames.includes(required)) throw new Error(`Missing required canvas tool: ${required}`);
    }
    const openTool = tools.find(tool => tool.name === 'open_canvas');
    const entrypoints = openTool?._meta?.['qoder/ui']?.entrypoints?.map(entry => entry.type) || [];
    if (!entrypoints.includes('global') || !entrypoints.includes('thread')) {
      throw new Error('open_canvas is missing required qoder/ui global and thread entrypoints');
    }
    const resourcesReply = await rpc(3, 'resources/list', {});
    const resource = (resourcesReply.result?.resources || []).find(item => item.uri === CANVAS_URI);
    if (!resource || resource.mimeType !== 'text/html;profile=mcp-app') {
      throw new Error(`Missing MCP App UI resource ${CANVAS_URI}`);
    }
    return {
      ok: true,
      serverInfo: init.result.serverInfo,
      canvasTools: toolNames.filter(name => name === 'open_canvas' || name.startsWith('canvas_')),
      qoderEntrypoints: entrypoints,
      resourceUri: resource.uri,
      mimeType: resource.mimeType,
    };
  } finally {
    child.stdin.end();
    child.kill();
  }
}

export async function verifyPluginHost({ configDir, pluginRoot = defaultPluginRoot, env = process.env, home } = {}) {
  const registration = await inspectQoderRegistration({ configDir, env, home });
  const installedRoot = registration.pluginRecord?.installPath;
  const command = installedRoot
    ? launcherPathForPlugin(installedRoot)
    : (registration.mcpEntry?.command || launcherPathForPlugin(pluginRoot));
  const args = installedRoot
    ? []
    : (Array.isArray(registration.mcpEntry?.args) ? registration.mcpEntry.args : []);
  const cwd = installedRoot || registration.mcpEntry?.cwd || resolve(pluginRoot);
  if (!existsSync(command)) {
    throw new Error(`Registered launcher does not exist: ${command}`);
  }
  const probe = await probeLauncher({ command, args, cwd, env });
  return {
    ok: true,
    pluginRegistered: registration.pluginRegistered,
    pluginEnabled: registration.pluginEnabled,
    mcpSettingsRegistered: registration.mcpSettingsRegistered,
    configDir: registration.configDir,
    command,
    skills: registration.skills,
    probe,
  };
}

export async function runHostCheck(args = process.argv.slice(2)) {
  let configDir;
  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--config-dir' && args[i + 1]) configDir = args[++i];
    else throw new Error('Usage: qoder-openpencil host-check [--config-dir PATH]');
  }
  try {
    console.log(JSON.stringify(await verifyPluginHost({ configDir }), null, 2));
  } catch (error) {
    console.error(`OpenPencil Qoder host check failed: ${error.message}`);
    process.exitCode = 1;
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await runHostCheck(process.argv.slice(2));
}
