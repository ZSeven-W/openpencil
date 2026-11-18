import { readFile } from 'node:fs/promises';
import { EditorSession } from './editor-session.mjs';
import { startPanel } from './canvas-panel.mjs';
import { forward, errorReply } from './transport.mjs';
import { readCanvasSelection } from './canvas-selection.mjs';
import { importCanvasImage } from './canvas-images.mjs';
import { prepareCanvasBrowser, browserLaunchStatus } from './browser-launch.mjs';
import { prepareCodegen, completeCodegen, pollCodegen, cancelCodegen, codegenSummary, frameworks } from './canvas-codegen.mjs';

export const CANVAS_URI = 'ui://openpencil/canvas/v0.3.13.html';
const schema = properties => ({ type: 'object', properties, additionalProperties: false });
const filePath = { type: 'string', description: 'Absolute .op path; omit to create or reopen this session’s canvas.' };
export const canvasTools = [
  { name: 'open_canvas', title: 'OpenPencil Canvas', description: 'Open the editable Rust canvas entrypoint: embedded canvas where allowed, or a browser launcher in restricted hosts such as Codex. Optional filePath loads an existing .op into an isolated managed session. All subsequent OpenPencil tools target this canvas until the bridge closes. Omit the path for a new document. Returns the same local panel URL as canvas_open_browser.',
    inputSchema: schema({ filePath }), annotations: { destructiveHint: false, openWorldHint: false },
    _meta: { ui: { resourceUri: CANVAS_URI }, 'openai/ui': { entrypoints: [{ type: 'thread' }] } } },
  { name: 'canvas_open_browser', description: 'Open or reconnect this chat’s managed editable canvas and return its local panelUrl. Show this URL in the Codex built-in browser with open_in_codex (browser target) or @Browser. Preserve any supplied theme/launch fragment. Use an external browser only when the user explicitly requests it. Use this when the host hides the open_canvas UI entrypoint or cannot embed local HTTP frames. All subsequent OpenPencil tools target this same canvas.',
    inputSchema: schema({ filePath }), annotations: { destructiveHint: false, openWorldHint: false } },
  { name: 'canvas_prepare_browser', description: 'Prepare this conversation’s existing canvas URL and short-lived ready receipt for opening through the current Codex chat. Does not launch a browser or send a message. Never accepts a URL or file path; the host controls in-app browser navigation.',
    inputSchema: { ...schema({ theme: { type: 'string', enum: ['light', 'dark'] } }), required: ['theme'] },
    annotations: { destructiveHint: false, openWorldHint: false }, _meta: { ui: { visibility: ['app'] } } },
  { name: 'canvas_launch_status', description: 'Read whether the browser canvas for this launch has actually connected.',
    inputSchema: { ...schema({ launchId: { type: 'string' } }), required: ['launchId'] },
    annotations: { readOnlyHint: true, openWorldHint: false }, _meta: { ui: { visibility: ['app'] } } },
  { name: 'canvas_status', description: 'Read managed canvas file, recovery path, document version and selection.', inputSchema: schema({}),
    annotations: { readOnlyHint: true, openWorldHint: false } },
  { name: 'canvas_selection', description: 'Read the current managed canvas selection: exact node IDs, names, types, parents, active page and document version. Call before modifying selected elements from a chat prompt; read these nodes with batch_get, then edit only their subtrees. Empty selection means no target, never the entire page.',
    inputSchema: schema({}), annotations: { readOnlyHint: true, openWorldHint: false } },
  { name: 'canvas_import_image', description: 'Embed a local PNG/JPEG/WebP (at most 8 MiB) into an existing image node in the managed canvas. Reads file bytes directly without putting base64 in chat. Requires the current document version and active page; preserves node ID, geometry and unrelated nodes. Clears acquisition hints after import. Does not generate images, call a provider, or save the source .op. After import inspect the native preview, canvas_save and verify reopen; never automatically retry an uncertain write.',
    inputSchema: { ...schema({ filePath: { type: 'string', description: 'Absolute path to a local image asset.' },
      nodeId: { type: 'string', minLength: 1 }, pageId: { type: 'string', minLength: 1 },
      expectedVersion: { type: 'integer', minimum: 0 } }), required: ['filePath', 'nodeId', 'pageId', 'expectedVersion'] },
    annotations: { destructiveHint: true, openWorldHint: false }, _meta: { ui: { visibility: ['model'] } } },
  { name: 'canvas_save', description: 'Save the managed canvas to its original path or a new absolute .op path. Refuses external changes or an existing Save As destination. UI must flush its snapshot first; never automatically retry.',
    inputSchema: schema({ filePath, docJson: { type: 'string', description: 'UI-only flushed document snapshot, used to reject stale saves.' } }),
    annotations: { destructiveHint: true, openWorldHint: false } },
  { name: 'canvas_checkpoint', description: 'Persist a recovery draft of the managed canvas without overwriting its source.', inputSchema: schema({}),
    annotations: { destructiveHint: false, openWorldHint: false }, _meta: { ui: { visibility: ['app'] } } },
  { name: 'canvas_codegen_prepare', description: 'Capture the Code panel target after flushing browser edits and export baseline source. Does not call a model or write project files.',
    inputSchema: { ...schema({ uiRequestId: { type: 'string' }, framework: { type: 'string', enum: frameworks }, pageId: { type: 'string' },
      nodeIds: { type: 'array', items: { type: 'string' } }, docJson: { type: 'string' } }), required: ['uiRequestId', 'framework', 'pageId', 'nodeIds', 'docJson'] },
    annotations: { destructiveHint: false, openWorldHint: false }, _meta: { ui: { visibility: ['app'] } } },
  { name: 'canvas_codegen_status', description: 'Read the latest Code panel conversion request, its fixed page/node IDs, design version, baseline source and handoff prompt. Use this before implementing a Code panel request; later selection changes do not change its target.',
    inputSchema: schema({}), annotations: { readOnlyHint: true, openWorldHint: false } },
  { name: 'canvas_codegen_complete', description: 'Return the primary implemented source file to the Code panel preview, copy and download actions. Pass the requestId from canvas_codegen_status and an absolute codePath after implementing and verifying the project. Reads the file; never writes it or changes the design. Rejects superseded requests and changed design versions.',
    inputSchema: { ...schema({ requestId: { type: 'string' }, codePath: { type: 'string' } }), required: ['requestId', 'codePath'] },
    annotations: { destructiveHint: false, openWorldHint: false } },
  { name: 'canvas_codegen_cancel', description: 'Cancel the matching panel request without replaying a host message.', inputSchema: { ...schema({ uiRequestId: { type: 'string' } }), required: ['uiRequestId'] },
    annotations: { destructiveHint: false, openWorldHint: false }, _meta: { ui: { visibility: ['app'] } } },
  { name: 'canvas_codegen_poll', description: 'Read a changed Code panel result.', inputSchema: schema({ afterRevision: { type: 'integer' } }),
    annotations: { readOnlyHint: true, openWorldHint: false }, _meta: { ui: { visibility: ['app'] } } },
];

const toolResult = value => ({ content: [{ type: 'text', text: JSON.stringify(value) }], structuredContent: value });

export class CanvasController {
  constructor(options = {}) { this.options = options; this.queue = Promise.resolve(); this.session = null; this.panel = null; }
  enqueue(work) { const result = this.queue.then(work); this.queue = result.catch(() => {}); return result; }
  async ensurePanel() { if (!this.panel) this.panel = await startPanel(this); return this.panel; }
  async open({ filePath } = {}) {
    const panel = await this.ensurePanel();
    if (this.session && filePath && filePath !== this.session.source && filePath !== this.session.draft) {
      throw new Error('This conversation already has a canvas. Save it, then open the other document in a new conversation.');
    }
    if (!this.session) {
      this.session = await EditorSession.start({ filePath, allowOrigin: panel.origin, runtimeRoot: this.options.runtimeRoot });
      await this.options.onToolsChanged?.();
    }
    this.session.assertLive();
    const value = { filePath: this.session.source, recoveryPath: this.session.draft, panelUrl: panel.url, version: this.session.version };
    return { ...toolResult(value), _meta: { canvas: { editorOrigin: `http://127.0.0.1:${this.session.port}`,
      token: this.session.token, panelUrl: panel.url } } };
  }
  async call(name, args) {
    if (name === 'open_canvas' || name === 'canvas_open_browser') return this.open(args);
    if (!this.session) throw new Error('Open a managed canvas with canvas_open_browser or open_canvas first.');
    if (name === 'canvas_prepare_browser') return toolResult(prepareCanvasBrowser(this, args));
    if (name === 'canvas_launch_status') return toolResult(browserLaunchStatus(this, args));
    if (name === 'canvas_launch_ready') return toolResult(browserLaunchStatus(this, args, true));
    if (name === 'canvas_import_image') return toolResult(await importCanvasImage(this.session, args));
    if (name === 'canvas_save') return toolResult(await this.session.save(args));
    if (name === 'canvas_checkpoint') return toolResult(await this.session.checkpoint());
    if (name === 'canvas_codegen_prepare') return toolResult(await prepareCodegen(this, args));
    if (name === 'canvas_codegen_status') return toolResult(this.codegenJob || null);
    if (name === 'canvas_codegen_complete') return toolResult(await completeCodegen(this, args));
    if (name === 'canvas_codegen_cancel') return toolResult(cancelCodegen(this, args));
    if (name === 'canvas_codegen_poll') return toolResult(pollCodegen(this, args));
    const current = await readCanvasSelection(this.session);
    return toolResult({ filePath: this.session.source, recoveryPath: this.session.draft, version: current.live.version, selection: current.selection, codegen: codegenSummary(this) });
  }
  async resource() {
    const panel = await this.ensurePanel();
    const template = await readFile(new URL('../ui/app.html', import.meta.url), 'utf8');
    return { contents: [{ uri: CANVAS_URI, mimeType: 'text/html;profile=mcp-app', text: template,
      _meta: { ui: { prefersBorder: false, permissions: { clipboardWrite: {} }, csp: { frameDomains: [panel.origin], connectDomains: [], resourceDomains: [] } },
        'openai/ui': { availableDisplayModes: ['inline', 'fullscreen'], preferredDisplayMode: 'fullscreen' } } }] };
  }
  async dispatch(message) {
    if (!message || Array.isArray(message) || message.jsonrpc !== '2.0' || typeof message.method !== 'string' ||
        (Object.hasOwn(message, 'id') && message.id !== null && !['string','number'].includes(typeof message.id))) {
      return errorReply(null, -32600, 'Expected a JSON-RPC 2.0 request or notification.');
    }
    const reply = result => ({ jsonrpc: '2.0', id: message.id, result });
    try {
      if (message.method === 'resources/read' && message.params?.uri === CANVAS_URI) return reply(await this.resource());
      if (message.method === 'resources/list') return reply({ resources: [{ uri: CANVAS_URI, name: 'OpenPencil Canvas', mimeType: 'text/html;profile=mcp-app' }] });
      if (message.method === 'tools/call' && canvasTools.some(tool => tool.name === message.params?.name)) {
        return reply(await this.call(message.params.name, message.params.arguments || {}));
      }
      if (this.session && message.method === 'tools/call' && message.params?.name === 'save_document') {
        if (message.params.arguments?.sourceFilePath) throw new Error('Source-copy save is unavailable while a managed canvas is active.');
        return reply(await this.call('canvas_save', message.params.arguments || {}));
      }
      const upstream = this.session ? await this.session.forward(message) : await forward(message, this.options);
      if (!Object.hasOwn(message, 'id')) return null;
      if (message.method === 'initialize') {
        const result = upstream?.result || { protocolVersion: message.params?.protocolVersion || '2025-11-25', serverInfo: { name: 'codex-openpencil', version: '0.3.13' } };
        result.capabilities = { ...result.capabilities, tools: { listChanged: true }, resources: {} };
        return reply(result);
      }
      if (message.method === 'tools/list') return reply({ ...upstream?.result, tools: [...(upstream?.result?.tools || []), ...canvasTools] });
      return upstream;
    } catch (error) {
      if (!Object.hasOwn(message, 'id')) return null;
      if (message.method === 'tools/call') return reply({ content: [{ type: 'text', text: error.message }], isError: true,
        ...(error.code ? { structuredContent: { code: error.code } } : {}) });
      return errorReply(message.id, -32000, error.message);
    }
  }
  async close() {
    await this.queue;
    try { if (this.session) await this.session.close(); } finally { if (this.panel) await this.panel.close(); }
  }
}
