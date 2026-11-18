import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { once } from 'node:events';
import { mkdir, readFile, writeFile, stat } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { discover } from '../plugins/openpencil/scripts/transport.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const args = process.argv.slice(2);
let binary, output;
for (let i = 0; i < args.length; i++) {
  if (args[i] === '--binary' && args[i + 1]) binary = resolve(args[++i]);
  else if (args[i] === '--output' && args[i + 1]) output = resolve(args[++i]);
  else throw new Error('Usage: npm run smoke -- --binary PATH [--output DIR]');
}
if (!binary) throw new Error('Pass --binary pointing to the Rust openpencil-desktop or op-host-web-server binary.');
output ??= join(root, 'artifacts', `smoke-${Date.now()}`);
await mkdir(output, { recursive: true });
const document = join(output, 'fixture.op');
// Exclusive creation avoids overwriting a user's existing document.
await writeFile(document, JSON.stringify({ version: '0.8.5', name: 'Codex Plugin Smoke', children: [] }), { flag: 'wx' });
const socket = createServer();
socket.listen(0, '127.0.0.1');
await once(socket, 'listening');
const port = socket.address().port;
await new Promise((resolve) => socket.close(resolve));
const host = spawn(binary, ['--mcp-http', String(port), document], { stdio: ['ignore', 'ignore', 'pipe'] });
let hostLog = '', hostFailed = false;
host.stderr.on('data', (chunk) => { hostLog = (hostLog + chunk).slice(-8000); });
host.on('error', () => { hostFailed = true; });
let bridge;
const pending = new Map();
let sequence = 0;
function rpc(method, params) {
  const id = ++sequence;
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(`Smoke RPC timed out: ${method}`)); }, 30_000);
    pending.set(id, { resolve, reject, timer });
    bridge.stdin.write(JSON.stringify({ jsonrpc: '2.0', id, method, params }) + '\n');
  });
}
function tool(name, args = {}) {
  return rpc('tools/call', { name, arguments: args }).then((result) => {
    if (result.isError) throw new Error(`OpenPencil tool failed: ${name}: ${JSON.stringify(result)}`);
    return result;
  });
}
function content(result) {
  const text = result.content?.find((c) => c.type === 'text')?.text;
  return text ? JSON.parse(text) : {};
}
try {
  let ready = false;
  for (let i = 0; i < 80; i++) {
    if (hostFailed || host.exitCode !== null) throw new Error('Headless OpenPencil failed to start. Check the binary.');
    try { await discover({ port }); ready = true; break; } catch {}
    await new Promise((r) => setTimeout(r, 100));
  }
  if (!ready) throw new Error('Headless OpenPencil did not become ready within 8 seconds.');
  bridge = spawn(process.execPath, [join(root, 'plugins/openpencil/scripts/mcp-bridge.mjs'), '--port', String(port)], { stdio: ['pipe', 'pipe', 'pipe'] });
  let buffer = '', bridgeLog = '';
  bridge.stderr.on('data', (c) => { bridgeLog = (bridgeLog + c).slice(-8000); });
  bridge.stdout.on('data', (c) => {
    buffer += c;
    let newline;
    while ((newline = buffer.indexOf('\n')) >= 0) {
      const line = buffer.slice(0, newline); buffer = buffer.slice(newline + 1);
      const response = JSON.parse(line);
      const call = pending.get(response.id);
      if (!call) continue;
      clearTimeout(call.timer); pending.delete(response.id);
      if (response.error) call.reject(new Error(response.error.message));
      else call.resolve(response.result);
    }
  });
  bridge.on('exit', () => {
    for (const call of pending.values()) { clearTimeout(call.timer); call.reject(new Error(`Bridge exited: ${bridgeLog}`)); }
    pending.clear();
  });
  const initialized = await rpc('initialize', { protocolVersion: '2024-11-05', capabilities: {}, clientInfo: { name: 'codex-openpencil-smoke', version: '0.1.0' } });
  bridge.stdin.write(JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' }) + '\n');
  const tools = await rpc('tools/list', {});
  const before = content(await tool('get_editor_state'));
  const built = content(await tool('batch_design', { script: `
    const root = I(null, {type:'frame',name:'Codex Plugin Smoke',width:480,height:320,fill:'#F8FAFC'});
    I(root, {type:'text',name:'Headline',content:'Design with Codex',x:32,y:32,width:400,height:48,fontSize:32,fontWeight:700,fill:'#0F172A'});
    I(root, {type:'text',name:'Description',content:'Editable OpenPencil vectors',x:32,y:100,width:400,height:32,fontSize:18,fill:'#475569'});
    I(root, {type:'rectangle',name:'Accent',x:32,y:176,width:160,height:48,cornerRadius:12,fill:'#CFB537'});
  ` }));
  if (built.errors && built.errors !== '[]' && built.errors.length !== 0) throw new Error(`Batch errors: ${JSON.stringify(built)}`);
  const state = content(await tool('get_editor_state'));
  const nodes = content(await tool('read_nodes', { depth: -1 }));
  const serialized = JSON.stringify(nodes);
  if (!serialized.includes('Design with Codex')) throw new Error('Created text was not present in the document.');
  const rootId = state.top_level_nodes?.split('|')[0];
  if (!rootId) throw new Error(`Could not determine the fixture root: ${JSON.stringify(state)}`);
  const layout = content(await tool('snapshot_layout', { parentId: rootId }));
  const lint = content(await tool('lint_document', { nodeId: rootId }));
  const screenshot = await tool('get_screenshot', { nodeId: rootId });
  const image = screenshot.content?.find((c) => c.type === 'image');
  if (!image || image.mimeType !== 'image/png') throw new Error('No native PNG image was returned.');
  const png = Buffer.from(image.data, 'base64');
  if (png.subarray(0, 8).toString('hex') !== '89504e470d0a1a0a') throw new Error('Invalid PNG signature.');
  await writeFile(join(output, 'preview.png'), png);
  const savedPath = join(output, 'saved.op');
  await tool('save_document', { filePath: savedPath });
  const saved = JSON.parse(await readFile(savedPath, 'utf8'));
  if (!JSON.stringify(saved).includes('Design with Codex')) throw new Error('Saved document lost the created copy.');
  const reopened = content(await tool('open_document', { filePath: savedPath }));
  const reread = content(await tool('read_nodes', { filePath: savedPath, depth: -1 }));
  if (!JSON.stringify(reread).includes('Design with Codex')) throw new Error('Reopened document lost the created copy.');
  const report = { ok: true, server: initialized.serverInfo, toolCount: tools.tools.length, before, state, layout, lint, reopened,
    savedBytes: (await stat(savedPath)).size, pngBytes: png.length, pngSha256: createHash('sha256').update(png).digest('hex'),
    checks: ['stdio initialize', 'notification', 'tools/list', 'create', 'read', 'layout', 'lint', 'native PNG', 'save', 'reopen'],
  };
  await writeFile(join(output, 'report.json'), JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify({ ok: true, toolCount: report.toolCount, output, checks: report.checks }));
} finally {
  for (const call of pending.values()) clearTimeout(call.timer);
  // Only terminate processes this smoke run spawned; never stop the live editor.
  if (bridge) { bridge.stdin.end(); bridge.kill(); }
  host.kill();
}
