import test from 'node:test';
import assert from 'node:assert/strict';
import http from 'node:http';
import { proxyFor, fetchRuntime } from '../plugins/openpencil/scripts/runtime-download.mjs';

test('HTTPS proxy selection honors environment, NO_PROXY and existing macOS settings', () => {
  const url = 'https://github.com/owner/repo';
  const system = 'HTTPSEnable : 1\nHTTPSProxy : 127.0.0.1\nHTTPSPort : 7897\n';
  assert.equal(proxyFor(url, {}, 'darwin', system), 'http://127.0.0.1:7897');
  assert.equal(proxyFor(url, { HTTPS_PROXY: 'http://env:8080' }, 'darwin', system), 'http://env:8080');
  assert.equal(proxyFor(url, { https_proxy: 'http://lower:8080', HTTPS_PROXY: 'http://env:8080' }, 'linux'), 'http://lower:8080');
  assert.equal(proxyFor(url, { HTTPS_PROXY: 'http://env:8080', NO_PROXY: '.github.com' }, 'darwin', system), null);
  assert.equal(proxyFor(url, { ALL_PROXY: 'http://all:8080', no_proxy: '*' }, 'linux'), null);
  assert.equal(proxyFor(url, { HTTPS_PROXY: 'http://env:8080', NO_PROXY: 'github.com:123' }, 'linux'), 'http://env:8080');
  assert.equal(proxyFor('https://sub.github.com', { NO_PROXY: 'github.com:443' }, 'darwin', system), null);
  assert.equal(proxyFor(url, {}, 'darwin', 'HTTPSEnable : 0'), null);
  assert.equal(proxyFor(url, {}, 'linux'), null);
});

test('Download transport rejects insecure URLs and unsupported proxy protocols', async () => {
  await assert.rejects(fetchRuntime('http://github.com/'), /HTTPS/);
  await assert.rejects(fetchRuntime('https://user:password@github.com/'), /credentials/);
  await assert.rejects(fetchRuntime('https://github.com/', { env: { HTTPS_PROXY: 'socks5://127.0.0.1:1' } }), /HTTP\/HTTPS proxies/);
});

test('CONNECT proxy failures surface once, with credentials confined to the proxy', async t => {
  let count = 0;
  const server = http.createServer();
  const sockets = new Set();
  server.on('connection', socket => { sockets.add(socket); socket.once('close', () => sockets.delete(socket)); });
  server.on('connect', (request, socket) => {
    count++; assert.equal(request.url, 'github.com:443');
    assert.equal(request.headers['proxy-authorization'], 'Basic ' + Buffer.from('test:fixture').toString('base64'));
    socket.end('HTTP/1.1 407 Proxy Authentication Required\r\n\r\n');
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  t.after(() => { for (const socket of sockets) socket.destroy(); return new Promise(resolve => server.close(resolve)); });
  await assert.rejects(fetchRuntime('https://github.com/', { env: { HTTPS_PROXY: `http://test:fixture@127.0.0.1:${server.address().port}` },
    signal: AbortSignal.timeout(2000) }), /407/);
  assert.equal(count, 1);
});
