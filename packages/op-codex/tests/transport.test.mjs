import { test } from 'node:test';
import assert from 'node:assert/strict';
import http from 'node:http';
import { mkdtemp, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { discover, forward, post, parsePort } from '../plugins/openpencil/scripts/transport.mjs';

async function server(t, responder) {
  const calls = [];
  const host = http.createServer(async (request, response) => {
    let body = '';
    for await (const chunk of request) body += chunk;
    const message = JSON.parse(body);
    calls.push(message);
    if (responder) return responder(message, response, calls);
    if (!Object.hasOwn(message, 'id')) { response.writeHead(204).end(); return; }
    response.setHeader('Content-Type', 'application/json');
    response.end(JSON.stringify({ jsonrpc: '2.0', id: message.id, result: message.method === 'ping'
      ? { _meta: { server: 'openpencil-mcp', mode: 'live', token: 'fixture-token' } }
      : { content: [{ type: 'text', text: '你好' }, { type: 'image', data: 'AA==', mimeType: 'image/png' }] },
    }));
  });
  host.listen(0, '127.0.0.1');
  await once(host, 'listening');
  t.after(() => new Promise((resolve) => { host.closeAllConnections(); host.close(resolve); }));
  return { port: host.address().port, calls };
}

async function record(t, value) {
  const dir = await mkdtemp(join(tmpdir(), 'codex-openpencil-test-'));
  t.after(() => rm(dir, { recursive: true, force: true }));
  const file = join(dir, 'discovery');
  await writeFile(file, JSON.stringify(value));
  return file;
}

test('only integer loopback ports are accepted', () => {
  for (const value of [0, 65536, -1, 1.5, 'http://example.com', '80/path', '', ' 80']) {
    assert.throws(() => parsePort(value));
  }
  assert.equal(parsePort('3100'), 3100);
});

test('discovery proves the instance token and does not expose it', async (t) => {
  const { port } = await server(t);
  const discoveryFile = await record(t, { port, transport: 'json-rpc', token: 'fixture-token' });
  assert.deepEqual(await discover({ discoveryFile }), { port, mode: 'live' });
});

test('stale discovery refuses writes rather than falling back', async (t) => {
  const { port, calls } = await server(t);
  const discoveryFile = await record(t, { port, transport: 'json-rpc', token: 'different-token' });
  const reply = await forward({ jsonrpc: '2.0', id: 7, method: 'tools/call', params: { name: 'batch_design' } }, { discoveryFile });
  assert.match(reply.error.message, /stale/);
  assert.deepEqual(calls.map((c) => c.method), ['ping']);
  assert.ok(!JSON.stringify(reply).includes('different-token'));
});

test('legacy REST, missing token, and missing discovery fail closed', async (t) => {
  for (const value of [{ port: 3100, transport: 'rest', token: 't' }, { port: 3100, transport: 'json-rpc' }]) {
    const discoveryFile = await record(t, value);
    await assert.rejects(discover({ discoveryFile }), /Invalid OpenPencil/);
  }
  await assert.rejects(discover({ discoveryFile: join(tmpdir(), 'absent-openpencil-record') }), /No valid/);
});

test('explicit endpoint still must identify as OpenPencil', async (t) => {
  const { port } = await server(t, (m, r) => r.end(JSON.stringify({ jsonrpc: '2.0', id: m.id, result: {} })));
  await assert.rejects(discover({ port }), /not a Rust OpenPencil/);
});

test('forwards arguments, IDs, Unicode and image content without conversion', async (t) => {
  const { port, calls } = await server(t);
  const message = { jsonrpc: '2.0', id: 'read', method: 'tools/call', params: { name: 'read_nodes', arguments: { nodeIds: ['你好'] } } };
  const reply = await forward(message, { port });
  assert.deepEqual(calls[1], message);
  assert.equal(reply.id, 'read');
  assert.equal(reply.result.content[0].text, '你好');
  assert.deepEqual(reply.result.content[1], { type: 'image', data: 'AA==', mimeType: 'image/png' });
});

test('notifications produce no stdio response', async (t) => {
  const { port, calls } = await server(t);
  assert.equal(await forward({ jsonrpc: '2.0', method: 'notifications/initialized' }, { port }), null);
  assert.equal(calls.at(-1).method, 'notifications/initialized');
});

test('JSON-RPC tool failures remain unchanged', async (t) => {
  const error = { code: -32602, message: 'Unknown node' };
  const { port } = await server(t, (m, r) => r.end(JSON.stringify({ jsonrpc: '2.0', id: m.id, error })));
  assert.deepEqual((await post(port, { jsonrpc: '2.0', id: 3, method: 'tools/call' })).error, error);
});

test('mutations are sent once even when HTTP fails', async (t) => {
  const { port, calls } = await server(t, (m, r) => {
    if (m.method === 'ping') r.end(JSON.stringify({ jsonrpc: '2.0', id: m.id, result: { _meta: { server: 'openpencil-mcp' } } }));
    else r.writeHead(503).end('private-server-body');
  });
  const reply = await forward({ jsonrpc: '2.0', id: 4, method: 'tools/call' }, { port });
  assert.match(reply.error.message, /HTTP 503/);
  assert.equal(calls.filter((m) => m.method === 'tools/call').length, 1);
  assert.ok(!JSON.stringify(reply).includes('private-server-body'));
});

test('timeout after a write does not retry', async (t) => {
  const { port, calls } = await server(t, () => {});
  await assert.rejects(post(port, { jsonrpc: '2.0', id: 5, method: 'tools/call' }, { timeout: 30 }), /not retried/);
  assert.equal(calls.length, 1);
});

test('rejects redirects, malformed and mismatched HTTP replies', async (t) => {
  for (const payload of ['not-json', JSON.stringify({ jsonrpc: '2.0', id: 99, result: {} }), '']) {
    const { port } = await server(t, (_, r) => r.end(payload));
    await assert.rejects(post(port, { jsonrpc: '2.0', id: 1, method: 'ping' }), /response/);
  }
  const { port, calls } = await server(t, (_, r) => r.writeHead(302, { Location: 'https://example.com' }).end());
  await assert.rejects(post(port, { jsonrpc: '2.0', id: 1, method: 'ping' }), /HTTP 302/);
  assert.equal(calls.length, 1);
});

test('bounds response size', async (t) => {
  const { port } = await server(t, (_, r) => r.end('x'.repeat(1024)));
  await assert.rejects(post(port, { jsonrpc: '2.0', id: 1, method: 'ping' }, { maxBytes: 64 }), /size limit/);
});

test('invalid requests cannot reach an endpoint', async () => {
  for (const message of [[], null, {}, { jsonrpc: '2.0', method: 'ping', id: {} }]) {
    assert.equal((await forward(message, { port: 1 })).error.code, -32600);
  }
});

test('stdio process handles handshake, notification, split Unicode, malformed input and tools', async (t) => {
  const { port } = await server(t);
  const child = spawn(process.execPath, ['plugins/openpencil/scripts/mcp-bridge.mjs', '--port', String(port)], { stdio: ['pipe', 'pipe', 'pipe'] });
  t.after(() => child.kill());
  let out = '', err = '';
  child.stdout.on('data', (chunk) => { out += chunk; });
  child.stderr.on('data', (chunk) => { err += chunk; });
  child.stdin.write(JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'initialize', params: { clientInfo: { name: 'Codex test' } } }) + '\n');
  child.stdin.write(JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' }) + '\n');
  child.stdin.write('bad-json\n');
  const unicode = Buffer.from(JSON.stringify({ jsonrpc: '2.0', id: 2, method: 'tools/call', params: { text: '你好' } }) + '\n');
  const split = unicode.indexOf(Buffer.from('你好')) + 1;
  child.stdin.write(unicode.subarray(0, split));
  child.stdin.end(unicode.subarray(split));
  const [code] = await once(child, 'close');
  assert.equal(code, 0, err);
  assert.equal(err, '');
  const replies = out.trim().split('\n').map((line) => JSON.parse(line));
  assert.deepEqual(replies.map((r) => r.id), [1, null, 2]);
  assert.equal(replies[1].error.code, -32700);
  assert.equal(replies[2].result.content[0].text, '你好');
});
