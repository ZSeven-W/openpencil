import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { createInterface } from 'node:readline';
import { mkdtemp, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const dir = await mkdtemp(join(tmpdir(), 'openpencil-terminal-test-'));
const file = join(dir, 'terminal.op');
await writeFile(file, JSON.stringify({ version: '0.8.5', children: [{ type: 'frame', id: 'terminal-board',
  name: 'Terminal test', width: 500, height: 300, fill: [{ type: 'solid', color: '#F4F1E9' }], children: [
    { type: 'text', id: 'terminal-title', x: 30, y: 30, content: 'Before', fontSize: 28, fontFamily: 'DM Sans', fill: [{ type: 'solid', color: '#222820' }] },
  ] }] }));
const canvasArgs = [fileURLToPath(new URL('./canvas.mjs', import.meta.url)), '--file', file];
let executable = process.execPath, commandArgs = canvasArgs;
if (process.argv.includes('--pty')) {
  if (process.platform === 'win32') throw new Error('The POSIX PTY regression requires Python 3 and its standard-library pty module.');
  // Test-only relay; production setup and canvas remain dependency-free Node.
  executable = 'python3';
  const relay = `import os,pty,select,subprocess,sys
master,slave=pty.openpty()
child=subprocess.Popen(sys.argv[1:],stdin=slave,stdout=slave,stderr=slave,close_fds=True)
os.close(slave)
inputs=[0,master]
while child.poll() is None:
    for fd in select.select(inputs,[],[],0.1)[0]:
        try: data=os.read(fd,65536)
        except OSError: data=b''
        if not data:
            inputs.remove(fd)
            continue
        os.write(master if fd==0 else 1,data)
os.close(master)
sys.exit(child.wait())`;
  commandArgs = ['-c', relay, process.execPath, ...canvasArgs];
}
const child = spawn(executable, commandArgs, { stdio: ['pipe', 'pipe', 'inherit'] });
const ended = once(child, 'close');
const lines = createInterface({ input: child.stdout });
let waiting;
const replies = [];
lines.on('line', line => {
  try { const value = JSON.parse(line); if (waiting) { const ready = waiting; waiting = null; ready(value); } else replies.push(value); }
  catch { /* Ignore non-protocol process diagnostics. */ }
});
async function next() {
  if (replies.length) return replies.shift();
  let timer;
  try { return await Promise.race([new Promise(resolve => { waiting = resolve; }),
    new Promise((_, reject) => { timer = setTimeout(() => reject(new Error('Terminal smoke response timed out.')), 30_000); })]); }
  finally { clearTimeout(timer); waiting = null; }
}
async function call(name, args = {}) {
  child.stdin.write(JSON.stringify({ name, arguments: args }) + '\n');
  const reply = await next();
  assert.ok(!reply.error && !reply.result?.isError, `${name} failed; not retried.`);
  return reply.result;
}
try {
  assert.equal((await next()).ready, true);
  const longName = '长指令验证 '.repeat(1200);
  await call('update_node', { nodeId: 'terminal-title', data: { content: 'Terminal edit verified', name: longName } });
  const preview = await call('get_screenshot', { nodeId: 'terminal-board' });
  assert.ok(!JSON.stringify(preview).includes('image_base64'), 'Terminal replies must not duplicate huge base64 payloads.');
  const images = preview.content.map(item => { try { return JSON.parse(item.text); } catch { return {}; } }).filter(item => item.imagePath);
  assert.ok(images.length);
  assert.equal((await readFile(images[0].imagePath)).subarray(0, 8).toString('hex'), '89504e470d0a1a0a');
  await call('canvas_save', { filePath: file });
  const saved = await readFile(file, 'utf8');
  assert.ok(saved.includes('Terminal edit verified'));
  assert.ok(saved.includes(longName));
  console.log(`Terminal fallback${process.argv.includes('--pty') ? ' through a real PTY' : ''}: long native edit, inspectable PNG preview and saved file: OK`);
} finally {
  child.stdin.end(JSON.stringify({ quit: true }) + '\n');
  await ended; lines.close(); await rm(dir, { recursive: true, force: true });
}
