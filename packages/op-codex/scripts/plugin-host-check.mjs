import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { codexCommand } from './codex-command.mjs';

// Exercise Codex's actual plugin loader, not a manually spawned bridge.
// This opens no chat and never calls a design mutation.
export async function verifyPluginHost({ env = process.env, timeout = 45_000, command = codexCommand({ env }) } = {}) {
  const { binary, prefix } = command;
  const child = spawn(binary, [...prefix, 'app-server'], { env, stdio: ['pipe', 'pipe', 'pipe'] });
  let id = 0;
  const pending = new Map();
  const lines = createInterface({ input: child.stdout });
  const closed = new Promise(resolve => child.once('close', resolve));
  const fail = error => { for (const request of pending.values()) request.reject(error); pending.clear(); };
  child.on('error', fail);
  child.on('exit', () => fail(new Error('Codex plugin host exited before verification.')));
  child.stdin.on('error', fail);
  child.stderr.resume();
  lines.on('line', line => {
    let value;
    try { value = JSON.parse(line); } catch { return; }
    const request = pending.get(value.id);
    if (!request) return;
    pending.delete(value.id);
    value.error ? request.reject(new Error(value.error.message)) : request.resolve(value.result);
  });
  function request(method, params) {
    return new Promise((resolve, reject) => {
      const current = ++id;
      const timer = setTimeout(() => { pending.delete(current); reject(new Error(`Codex ${method} timed out.`)); }, timeout);
      pending.set(current, {
        resolve: value => { clearTimeout(timer); resolve(value); },
        reject: error => { clearTimeout(timer); reject(error); },
      });
      child.stdin.write(`${JSON.stringify({ id: current, method, params })}\n`);
    });
  }
  try {
    await request('initialize', { clientInfo: { name: 'openpencil_setup_check', version: '1' }, capabilities: { experimentalApi: true } });
    child.stdin.write('{"method":"initialized"}\n');
    const status = await request('mcpServerStatus/list', { serverName: 'codex_openpencil', detail: 'toolsAndAuthOnly' });
    const server = status.data?.find(server => server.name === 'codex_openpencil');
    if (!server) throw new Error('Codex did not load the OpenPencil plugin server. Check plugin enablement and update Codex.');
    if (server.pluginId !== 'openpencil@zseven-openpencil') throw new Error('Codex resolved codex_openpencil from another source. Inspect that MCP configuration before changing it.');
    if (server.toolsError) throw new Error(`Codex could not start OpenPencil: ${server.toolsError}`);
    const controls = ['open_canvas', 'canvas_open_browser', 'canvas_prepare_browser', 'canvas_launch_status', 'canvas_status', 'canvas_selection', 'canvas_import_image', 'canvas_save', 'canvas_codegen_status', 'canvas_codegen_complete'];
    if (controls.some(name => !server.tools?.[name])) throw new Error('Codex loaded an incomplete OpenPencil tool catalog. Reinstall the plugin and reconnect.');
    return { ok: true, server: server.name, pluginId: server.pluginId, controls, toolCount: Object.keys(server.tools).length };
  } finally {
    lines.close(); child.stdin.end(); child.kill('SIGTERM');
    const killTimer = setTimeout(() => child.kill('SIGKILL'), 2000);
    await closed; clearTimeout(killTimer);
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { console.log(JSON.stringify(await verifyPluginHost(), null, 2)); }
  catch (error) { console.error(`OpenPencil host check failed: ${error.message}`); process.exitCode = 1; }
}
