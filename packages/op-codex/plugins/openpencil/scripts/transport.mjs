import http from 'node:http';
import { readFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join } from 'node:path';

export const MAX_BYTES = 32 * 1024 * 1024;

export function parsePort(value) {
  const port = Number(value);
  if (!/^\d+$/.test(String(value)) || !Number.isInteger(port) || port < 1 || port > 65535) {
    throw new Error('OpenPencil port must be an integer from 1 to 65535.');
  }
  return port;
}

// Direct loopback HTTP: no proxy, remote URL, redirect, or shell execution.
export function post(port, message, { timeout = 120_000, maxBytes = MAX_BYTES } = {}) {
  return new Promise((resolve, reject) => {
    const body = JSON.stringify(message);
    const request = http.request({
      host: '127.0.0.1', port, path: '/mcp', method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Accept: 'application/json, text/event-stream',
        'Content-Length': Buffer.byteLength(body),
      },
    }, (response) => {
      const chunks = [];
      let size = 0;
      response.on('data', (chunk) => {
        size += chunk.length;
        if (size > maxBytes) {
          response.destroy(new Error('OpenPencil response exceeds the size limit.'));
          return;
        }
        chunks.push(chunk);
      });
      response.on('error', reject);
      response.on('end', () => {
        if (response.statusCode < 200 || response.statusCode >= 300) {
          reject(new Error(`OpenPencil returned HTTP ${response.statusCode}; the request was not retried.`));
          return;
        }
        const text = Buffer.concat(chunks).toString('utf8');
        if (!text.trim()) {
          if (!Object.hasOwn(message, 'id')) resolve(null);
          else reject(new Error('OpenPencil returned an empty response to a request.'));
          return;
        }
        try {
          const reply = JSON.parse(text);
          if (!Object.hasOwn(message, 'id')) { resolve(null); return; }
          if (reply.jsonrpc !== '2.0' || reply.id !== message.id ||
              (!Object.hasOwn(reply, 'result') && !Object.hasOwn(reply, 'error'))) {
            throw new Error();
          }
          resolve(reply);
        } catch {
          reject(new Error('OpenPencil returned an invalid JSON-RPC response.'));
        }
      });
    });
    const timer = setTimeout(() => request.destroy(new Error(
      'OpenPencil request timed out; it was not retried. Inspect the canvas before repeating an edit.',
    )), timeout);
    request.on('close', () => clearTimeout(timer));
    request.on('error', (error) => reject(error.code
      ? new Error(`Cannot reach local OpenPencil (${error.code}). Start its MCP server, then reconnect.`)
      : error));
    request.end(body);
  });
}

export function identityOf(reply) {
  const result = reply?.result;
  const identity = result?._meta ?? result;
  if (identity?.server !== 'openpencil-mcp') {
    throw new Error('The local endpoint is not a Rust OpenPencil MCP server.');
  }
  return identity;
}

export async function discover({ port, discoveryFile = join(homedir(), '.openpencil', '.op-mcp-port') } = {}) {
  let expectedToken;
  if (port !== undefined) {
    port = parsePort(port);
  } else {
    let record;
    try { record = JSON.parse(await readFile(discoveryFile, 'utf8')); }
    catch { throw new Error('No valid OpenPencil discovery record. Start the Rust editor and enable MCP, or set OPENPENCIL_MCP_PORT.'); }
    if (record.transport !== 'json-rpc' || typeof record.token !== 'string' || !record.token) {
      throw new Error('Invalid OpenPencil discovery record: expected json-rpc transport and an instance token.');
    }
    port = parsePort(record.port);
    expectedToken = record.token;
  }
  const identity = identityOf(await post(port, { jsonrpc: '2.0', id: 'identity', method: 'ping' }, { timeout: 3_000 }));
  if (expectedToken !== undefined && (identity.mode !== 'live' || identity.token !== expectedToken)) {
    throw new Error('OpenPencil discovery record is stale: instance identity does not match. Reopen the editor.');
  }
  return { port, mode: identity.mode ?? 'unknown' };
}

export function errorReply(id, code, message) {
  return { jsonrpc: '2.0', id, error: { code, message } };
}

export async function forward(message, options) {
  if (!message || typeof message !== 'object' || Array.isArray(message) ||
      message.jsonrpc !== '2.0' || typeof message.method !== 'string' ||
      (Object.hasOwn(message, 'id') && message.id !== null &&
       typeof message.id !== 'string' && typeof message.id !== 'number')) {
    return errorReply(null, -32600, 'Expected a JSON-RPC 2.0 request or notification.');
  }
  try {
    const endpoint = await discover(options);
    return await post(endpoint.port, message);
  } catch (error) {
    // Never include raw server bodies or the discovery token in a diagnostic.
    if (!Object.hasOwn(message, 'id')) return null;
    return errorReply(message.id, -32000, error.message);
  }
}
