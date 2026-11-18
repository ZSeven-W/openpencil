import test from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, readFile, writeFile, rm, mkdir, symlink } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { registerNode } from '../scripts/node-registration.mjs';
import { verifyPluginHost } from '../scripts/plugin-host-check.mjs';

test('installed launcher initializes MCP with a registered Node outside the GUI PATH', async t => {
  const home = await mkdtemp(join(tmpdir(), 'openpencil launch & home '));
  t.after(() => rm(home, { recursive: true, force: true }));
  const state = join(home, '.codex-openpencil');
  let node = process.execPath;
  if (process.platform !== 'win32') {
    await mkdir(join(home, 'bin'));
    node = join(home, 'bin', 'node & current');
    await symlink(process.execPath, node);
  }
  await registerNode(state, node);
  assert.equal(await readFile(join(state, 'node-path'), 'utf8'), `${node}\n`);
  const windows = process.platform === 'win32';
  const launcher = resolve(`plugins/openpencil/scripts/launch-mcp${windows ? '.cmd' : ''}`);
  const env = { ...process.env, HOME: home, USERPROFILE: home,
    PATH: windows ? `${process.env.SystemRoot}\\System32;${process.env.SystemRoot}` : '/usr/bin:/bin' };
  // Only the fixed launcher path is interpreted by cmd.exe; MCP inputs use stdin.
  const child = spawn(launcher, [], { env, shell: windows, stdio: ['pipe', 'pipe', 'pipe'] });
  const closed = new Promise((resolve, reject) => { child.once('close', resolve); child.once('error', reject); });
  let stdout = '', stderr = '';
  child.stdout.on('data', value => { stdout += value; });
  child.stderr.on('data', value => { stderr += value; });
  child.stdin.end(`${JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'initialize', params: {
    protocolVersion: '2025-11-25', clientInfo: { name: 'launcher-regression', version: '1' }, capabilities: {},
  } })}\n`);
  assert.equal(await closed, 0, stderr);
  const reply = JSON.parse(stdout.trim());
  assert.equal(reply.id, 1);
  assert.equal(reply.result.serverInfo.name, 'codex-openpencil');
  await assert.rejects(registerNode(state, `${node}\ninvalid`), /Invalid Node/);
});

test('host check distinguishes a loaded plugin from registration and never starts a design', async t => {
  const dir = await mkdtemp(join(tmpdir(), 'openpencil host check '));
  t.after(() => rm(dir, { recursive: true, force: true }));
  const fixture = join(dir, 'fake-codex.mjs');
  const calls = join(dir, 'calls.jsonl');
  await writeFile(fixture, `import { createInterface } from 'node:readline';
import { appendFileSync } from 'node:fs';
for await (const line of createInterface({input:process.stdin})) {
 const q=JSON.parse(line); appendFileSync(process.env.CALLS,q.method+'\\n');
 if(!q.id) continue;
 const controls=Object.fromEntries(['open_canvas','canvas_open_browser','canvas_prepare_browser','canvas_launch_status','canvas_status','canvas_selection','canvas_import_image','canvas_save','canvas_codegen_status','canvas_codegen_complete'].map(n=>[n,{}]));
 if(process.env.CASE==='old-version') delete controls.canvas_import_image;
 if(process.env.CASE==='old-launcher') delete controls.canvas_prepare_browser;
 const result=q.method==='initialize'?{}:{data:process.env.CASE==='missing'?[]:[{name:'codex_openpencil',
 pluginId:process.env.CASE==='foreign'?null:'openpencil@zseven-openpencil',tools:process.env.CASE==='incomplete'?{}:controls,
 toolsError:process.env.CASE==='failed'?'Node executable missing':null}]};
 console.log(JSON.stringify({id:q.id,result}));
}`);
  const options = { command: { binary: process.execPath, prefix: [fixture] }, env: { ...process.env, CALLS: calls }, timeout: 3000 };
  const status = await verifyPluginHost(options);
  assert.equal(status.ok, true);
  assert.equal(status.toolCount, 10);
  assert.deepEqual((await readFile(calls, 'utf8')).trim().split('\n'), ['initialize', 'initialized', 'mcpServerStatus/list']);
  for (const [scenario, error] of [['missing', /did not load/], ['foreign', /another source/], ['incomplete', /incomplete/], ['old-version', /incomplete/], ['old-launcher', /incomplete/], ['failed', /Node executable missing/]]) {
    await assert.rejects(verifyPluginHost({ ...options, env: { ...options.env, CASE: scenario } }), error);
  }
});
