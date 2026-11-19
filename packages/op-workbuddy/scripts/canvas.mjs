import { createInterface } from 'node:readline';
import { resolve, join } from 'node:path';
import { writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { CanvasController } from '../plugins/openpencil/scripts/canvas-controller.mjs';

export async function runCanvas(args = process.argv.slice(2)) {
  if (args.length && (args.length !== 2 || args[0] !== '--file')) {
    throw new Error('Usage: workbuddy-openpencil canvas [--file /absolute/design.op]');
  }
  const controller = new CanvasController();
  const tty = process.stdin.isTTY && typeof process.stdin.setRawMode === 'function';
  const previousRawMode = process.stdin.isRaw;
  if (tty) process.stdin.setRawMode(true);
  const input = createInterface({ input: process.stdin, terminal: false });
  let stopping = false;
  const shutdown = () => { stopping = true; input.close(); };
  const interrupt = chunk => { if (chunk.includes(3) || chunk.includes(4)) shutdown(); };
  if (tty) process.stdin.on('data', interrupt);
  process.once('SIGINT', shutdown);
  process.once('SIGTERM', shutdown);
  try {
    const opened = await controller.open(args.length ? { filePath: resolve(args[1]) } : {});
    console.log(JSON.stringify({
      ready: true,
      ...opened.structuredContent,
      input: 'One JSON object per line: {"name":"get_editor_state","arguments":{}}. Send {"quit":true} to close. Never retry a failed write automatically.',
    }));
    let id = 0;
    for await (const line of input) {
      if (stopping) break;
      if (!line.trim()) continue;
      try {
        const request = JSON.parse(line);
        if (request.quit === true) break;
        const message = request.method
          ? { ...request, jsonrpc: '2.0', id: ++id }
          : { jsonrpc: '2.0', id: ++id, method: 'tools/call', params: { name: request.name, arguments: request.arguments || {} } };
        const reply = await controller.enqueue(() => controller.dispatch(message));
        const imagePaths = new Map();
        for (const [index, item] of (reply?.result?.content || []).entries()) {
          const extension = { 'image/png': 'png', 'image/jpeg': 'jpg', 'image/webp': 'webp' }[item.mimeType];
          if (item.type !== 'image' || !extension || typeof item.data !== 'string') continue;
          const path = join(controller.session.dir, `preview-${id}-${index}.${extension}`);
          await writeFile(path, Buffer.from(item.data, 'base64'), { flag: 'wx', mode: 0o600 });
          imagePaths.set(item.data, path);
          reply.result.content[index] = { type: 'text', text: JSON.stringify({ imagePath: path, mimeType: item.mimeType }) };
        }
        for (const item of reply?.result?.content || []) {
          if (item.type !== 'text') continue;
          try {
            const value = JSON.parse(item.text);
            if (imagePaths.has(value.image_base64)) {
              value.imagePath = imagePaths.get(value.image_base64);
              delete value.image_base64;
              item.text = JSON.stringify(value);
            }
          } catch { /* Preserve native non-JSON text. */ }
        }
        console.log(JSON.stringify(reply));
      } catch (error) {
        console.log(JSON.stringify({ error: error.message }));
      }
    }
  } finally {
    input.close();
    process.stdin.off('data', interrupt);
    if (tty) { process.stdin.setRawMode(Boolean(previousRawMode)); process.stdin.pause(); }
    await controller.close();
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await runCanvas(process.argv.slice(2));
}
