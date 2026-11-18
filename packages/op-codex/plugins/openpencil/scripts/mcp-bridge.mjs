#!/usr/bin/env node
import { once } from 'node:events';
import { StringDecoder } from 'node:string_decoder';
import { discover, errorReply, MAX_BYTES, parsePort } from './transport.mjs';
import { CanvasController } from './canvas-controller.mjs';

function optionsFromArgs(args) {
  const options = { port: process.env.OPENPENCIL_MCP_PORT };
  let doctor = false;
  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--doctor') doctor = true;
    else if (args[i] === '--port' && args[i + 1]) options.port = parsePort(args[++i]);
    else throw new Error('Usage: node mcp-bridge.mjs [--doctor] [--port PORT]');
  }
  if (options.port !== undefined) options.port = parsePort(options.port);
  return { options, doctor };
}

async function write(reply) {
  if (reply && !process.stdout.write(`${JSON.stringify(reply)}\n`)) await once(process.stdout, 'drain');
}

async function serve(options) {
  const canvas = new CanvasController({ ...options,
    onToolsChanged: () => write({ jsonrpc: '2.0', method: 'notifications/tools/list_changed' }) });
  const shutdown = () => { process.stdin.destroy(); };
  process.once('SIGTERM', shutdown); process.once('SIGINT', shutdown);
  const decoder = new StringDecoder('utf8');
  let buffer = '';
  // Serialize messages to preserve the ordering of canvas edits and notifications.
  try { for await (const chunk of process.stdin) {
    buffer += decoder.write(chunk);
    let newline;
    while ((newline = buffer.indexOf('\n')) !== -1) {
      const line = buffer.slice(0, newline);
      buffer = buffer.slice(newline + 1);
      if (Buffer.byteLength(line) > MAX_BYTES) throw new Error('MCP input exceeds the size limit.');
      if (!line.trim()) continue;
      let message;
      try { message = JSON.parse(line); }
      catch { await write(errorReply(null, -32700, 'Invalid JSON.')); continue; }
      await write(await canvas.enqueue(() => canvas.dispatch(message)));
    }
    if (Buffer.byteLength(buffer) > MAX_BYTES) throw new Error('MCP input exceeds the size limit.');
  }
  buffer += decoder.end();
  if (buffer.trim()) await write(errorReply(null, -32700, 'MCP messages must end with a newline.'));
  } finally {
    process.off('SIGTERM', shutdown); process.off('SIGINT', shutdown);
    await canvas.close();
  }
}

try {
  const { options, doctor } = optionsFromArgs(process.argv.slice(2));
  if (doctor) {
    const endpoint = await discover(options);
    process.stdout.write(`${JSON.stringify({ ok: true, ...endpoint, transport: 'json-rpc' })}\n`);
  } else await serve(options);
} catch (error) {
  process.stderr.write(`OpenPencil: ${error.message}\n`);
  process.exitCode = 1;
}
