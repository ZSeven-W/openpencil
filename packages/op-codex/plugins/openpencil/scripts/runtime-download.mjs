import http from 'node:http';
import https from 'node:https';
import tls from 'node:tls';
import { execFileSync } from 'node:child_process';

export function proxyFor(target, env = process.env, platform = process.platform, systemProxy) {
  const url = new URL(target);
  const exceptions = env.no_proxy ?? env.NO_PROXY ?? '';
  if (exceptions.split(',').some(value => {
    const rule = value.trim().toLowerCase();
    if (!rule) return false;
    if (rule === '*') return true;
    const [host, port] = rule.replace(/^\*?\./, '').split(':');
    return (!port || port === (url.port || '443')) && (url.hostname === host || url.hostname.endsWith('.' + host));
  })) return null;
  const configured = env.https_proxy || env.HTTPS_PROXY || env.all_proxy || env.ALL_PROXY;
  if (configured) return configured;
  if (platform === 'darwin') {
    let text = systemProxy;
    if (text === undefined) {
      try { text = execFileSync('/usr/sbin/scutil', ['--proxy'], { encoding: 'utf8', timeout: 3000 }); }
      catch { return null; }
    }
    const field = name => text.match(new RegExp(`^\\s*${name}\\s*:\\s*(.+)$`, 'm'))?.[1]?.trim();
    if (field('HTTPSEnable') === '1' && field('HTTPSProxy') && /^\d+$/.test(field('HTTPSPort') || '')) {
      return `http://${field('HTTPSProxy')}:${field('HTTPSPort')}`;
    }
  }
  return null;
}

function tunnel(target, proxy, signal) {
  return new Promise((resolve, reject) => {
    const authority = `${target.hostname}:${target.port || '443'}`;
    const headers = { Host: authority };
    if (proxy.username || proxy.password) headers['Proxy-Authorization'] = 'Basic ' +
      Buffer.from(`${decodeURIComponent(proxy.username)}:${decodeURIComponent(proxy.password)}`).toString('base64');
    const request = (proxy.protocol === 'https:' ? https : http).request(proxy, {
      method: 'CONNECT', path: authority, headers, signal,
    });
    request.setTimeout(30_000, () => request.destroy(new Error('Runtime proxy connection timed out.')));
    request.once('error', reject);
    request.once('connect', (response, socket, head) => {
      if (response.statusCode !== 200) { socket.destroy(); reject(new Error(`Runtime HTTPS proxy refused CONNECT (HTTP ${response.statusCode}).`)); return; }
      if (head.length) socket.unshift(head);
      const secure = tls.connect({ socket, servername: target.hostname });
      secure.once('error', reject);
      resolve(secure);
    });
    request.end();
  });
}

// Node 22's global fetch ignores proxy environment variables and macOS system
// proxy settings. Use the standard HTTPS client with a verified TLS tunnel.
// Proxy credentials only go to CONNECT, never to GitHub or a redirect target.
export async function fetchRuntime(input, { signal, env, platform, systemProxy, redirects = 5 } = {}) {
  const url = new URL(input);
  if (url.protocol !== 'https:' || url.username || url.password) throw new Error('Runtime downloads require an HTTPS URL without embedded credentials.');
  const proxyUrl = proxyFor(url, env, platform, systemProxy);
  let agent;
  if (proxyUrl) {
    const proxy = new URL(proxyUrl);
    if (!['http:', 'https:'].includes(proxy.protocol)) throw new Error('Runtime downloads support HTTP/HTTPS proxies. Set HTTPS_PROXY to the HTTP proxy endpoint.');
    agent = new https.Agent({ keepAlive: false });
    agent.createConnection = (_options, callback) => { tunnel(url, proxy, signal).then(socket => callback(null, socket), callback); };
  }
  const response = await new Promise((resolve, reject) => {
    const request = https.get(url, { agent, signal, headers: { 'User-Agent': 'codex-openpencil-runtime', Accept: 'application/octet-stream' } }, resolve);
    request.setTimeout(30_000, () => request.destroy(new Error('Runtime download stalled.')));
    request.once('error', reject);
  });
  if ([301, 302, 303, 307, 308].includes(response.statusCode)) {
    response.destroy();
    if (redirects < 1 || !response.headers.location) throw new Error('Runtime download redirect limit exceeded.');
    return fetchRuntime(new URL(response.headers.location, url), { signal, env, platform, systemProxy, redirects: redirects - 1 });
  }
  const headers = new Headers();
  for (const [key, value] of Object.entries(response.headers)) {
    if (value !== undefined) headers.set(key, Array.isArray(value) ? value.join(', ') : value);
  }
  return { ok: response.statusCode >= 200 && response.statusCode < 300, status: response.statusCode, headers, body: response };
}
