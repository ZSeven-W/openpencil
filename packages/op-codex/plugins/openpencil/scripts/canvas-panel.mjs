import http from 'node:http';
import { randomBytes } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { MAX_BYTES } from './transport.mjs';

export async function panelHtml(grant) {
  const root = new URL('../ui/', import.meta.url);
  const [html, css, panelJs, selectionJs, codegenJs] = await Promise.all(['panel.html', 'panel.css', 'panel.js', 'selection.js', 'codegen.js'].map(name => readFile(new URL(name, root), 'utf8')));
  const js = panelJs + '\n' + selectionJs + '\n' + codegenJs;
  const json = JSON.stringify(grant).replace(/</g, '\\u003c');
  return html.replace('/* PANEL_CSS */', css).replace('/* PANEL_GRANT */', json).replace('/* PANEL_JS */', js);
}

export async function startPanel(controller) {
  const capability = randomBytes(32).toString('base64url');
  let origin;
  const route = `/canvas/${capability}`;
  const server = http.createServer(async (req, res) => {
    const reply = (status, value) => {
      if (res.destroyed || res.headersSent) return;
      res.writeHead(status, { 'Content-Type': 'application/json', 'Cache-Control': 'no-store' });
      res.end(JSON.stringify(value));
    };
    try {
      if (req.headers.host !== new URL(origin).host || (req.headers.origin && req.headers.origin !== origin)) {
        req.resume(); reply(403, { error: 'Canvas origin is not authorized.' }); return;
      }
      if (req.url === route && req.method === 'GET') {
        const session = await controller.enqueue(() => controller.open({}));
        const html = await panelHtml({ ...session._meta.canvas, rpcPath: `${route}/rpc`, source: session.structuredContent.filePath });
        res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8', 'Cache-Control': 'no-store',
          'Referrer-Policy': 'no-referrer', 'X-Content-Type-Options': 'nosniff',
          'Content-Security-Policy': `default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; frame-src ${session._meta.canvas.editorOrigin}; connect-src 'self'; img-src data:` });
        res.end(html); return;
      }
      if (req.url === `${route}/rpc` && req.method === 'POST') {
        const chunks = []; let size = 0;
        for await (const chunk of req) {
          size += chunk.length;
          if (size > MAX_BYTES) { reply(413, { error: 'Canvas request exceeds 32 MiB.' }); req.destroy(); return; }
          chunks.push(chunk);
        }
        const message = JSON.parse(Buffer.concat(chunks).toString('utf8'));
        if (!['canvas_status', 'canvas_selection', 'canvas_save', 'canvas_checkpoint', 'canvas_codegen_prepare', 'canvas_codegen_poll', 'canvas_codegen_cancel', 'canvas_launch_ready'].includes(message.name)) {
          reply(403, { error: 'Unsupported canvas control.' }); return;
        }
        const result = await controller.enqueue(() => controller.call(message.name, message.arguments || {}));
        reply(200, result); return;
      }
      req.resume(); reply(404, { error: 'Canvas route not found.' });
    } catch (error) { reply(400, { error: error.message, code: error.code }); }
  });
  await new Promise((resolve, reject) => { server.once('error', reject); server.listen(0, '127.0.0.1', resolve); });
  origin = `http://127.0.0.1:${server.address().port}`;
  return { origin, url: origin + route, close: async () => {
    server.closeAllConnections(); await new Promise(resolve => server.close(resolve));
  } };
}
