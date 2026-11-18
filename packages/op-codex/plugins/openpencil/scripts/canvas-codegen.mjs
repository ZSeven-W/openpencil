import { randomUUID } from 'node:crypto';
import { lstat, readFile } from 'node:fs/promises';
import { extname, isAbsolute } from 'node:path';
import { readCanvasSelection } from './canvas-selection.mjs';
import { snapshotMatches } from './document-format.mjs';

export const frameworks = ['react', 'react-tailwind', 'vue', 'svelte', 'html', 'flutter', 'swiftui', 'compose', 'react-native'];
const fail = message => { throw new Error(message); };
const equalIds = (a, b) => { const sorted = [...b].sort(); return Array.isArray(a) && a.length === b.length && [...a].sort().every((id, i) => id === sorted[i]); };
const summary = job => job && ({ requestId: job.requestId, uiRequestId: job.uiRequestId, framework: job.framework,
  pageId: job.pageId, nodeIds: job.nodeIds, expectedVersion: job.expectedVersion, status: job.status, revision: job.revision });

export function codegenSummary(controller) { return summary(controller.codegenJob); }

export async function prepareCodegen(controller, args) {
  if (!frameworks.includes(args.framework) || typeof args.uiRequestId !== 'string' || !/^host-\d+-\d+$/.test(args.uiRequestId)) fail('Invalid code-generation request.');
  // A repeated browser event cannot create another chat request.
  if (controller.codegenJob?.uiRequestId === args.uiRequestId) return controller.codegenJob;
  const session = controller.session;
  const { live, selection } = await readCanvasSelection(session, { fresh: true });
  const page = live.document.pages?.length ? live.document.pages[selection.pageIndex] : live.document;
  const nodeIds = selection.count ? selection.nodeIds : (page.children || []).map(node => node.id);
  if (!nodeIds.length) fail('Select a frame or add content before generating code.');
  if (selection.pageId !== args.pageId || !equalIds(args.nodeIds, nodeIds)) fail('The code-generation target changed. Select the intended page or elements and try again.');
  let snapshot;
  try { snapshot = JSON.parse(args.docJson); } catch { fail('A flushed canvas snapshot is required.'); }
  if (!snapshotMatches(snapshot, live)) fail('Canvas edits are still syncing. No code-generation request was sent.');
  const exported = await session.forward({ jsonrpc: '2.0', id: 'canvas-codegen-export', method: 'tools/call',
    params: { name: 'codegen_export', arguments: { framework: args.framework, pageId: selection.pageId, nodeIds: JSON.stringify(nodeIds) } } });
  if (exported.error || exported.result?.isError) fail(exported.error?.message || exported.result?.content?.[0]?.text || 'Code export failed.');
  const output = exported.result.structuredContent || JSON.parse(exported.result.content.find(item => item.type === 'text').text);
  if (!output.code || typeof output.code !== 'string' || Buffer.byteLength(output.code) > 2 * 1024 * 1024) fail('The runtime returned no source code.');
  // Recheck after exporting: a concurrent MCP edit must not acquire an older brief.
  const after = await session.document();
  if (after.version !== live.version) fail('The design changed during code export. No request was sent.');
  const requestId = randomUUID();
  const context = { requestId, framework: args.framework, pageId: selection.pageId, nodeIds,
    expectedVersion: live.version, canvasFile: session.source, recoveryPath: session.draft };
  const prompt = `Use the OpenPencil code-generation workflow for this request: ${JSON.stringify(context)}. ` +
    `Read canvas_codegen_status and require its requestId to equal ${requestId}; a mismatch or cancelled status means this request must stop. Inspect the captured nodes and their native screenshot, then implement them in the current project using its existing conventions. ` +
    'Use codegen_export as a starting point; preserve text, layout, images and design tokens. Finish a runnable implementation and relevant checks. ' +
    'Read the openpencil-design code-generation reference. Return the primary source to the Code panel with canvas_codegen_complete(requestId, codePath). ' +
    'Do not modify the design or silently follow later selection changes. If the design version changed, stop and explain.';
  const job = { ...context, uiRequestId: args.uiRequestId, code: output.files?.['component.tsx'] || output.code,
    files: output.files, prompt, status: 'prepared', revision: (controller.codegenJob?.revision || 0) + 1 };
  controller.codegenJob = job;
  return job;
}

export async function completeCodegen(controller, { requestId, codePath }) {
  const job = controller.codegenJob;
  if (!job || job.requestId !== requestId || job.status === 'cancelled') fail('This code-generation request is no longer active.');
  if (!isAbsolute(codePath || '')) fail('Use the absolute path of the primary generated source file.');
  const extensions = { react: ['.tsx', '.jsx'], 'react-tailwind': ['.tsx', '.jsx'], vue: ['.vue'], svelte: ['.svelte'], html: ['.html'], flutter: ['.dart'], swiftui: ['.swift'], compose: ['.kt'], 'react-native': ['.tsx', '.jsx'] };
  if (!extensions[job.framework].includes(extname(codePath).toLowerCase())) fail('The primary source extension must match the requested framework.');
  const stat = await lstat(codePath);
  if (!stat.isFile() || stat.isSymbolicLink() || stat.size > 2 * 1024 * 1024) fail('Generated source must be a regular file of at most 2 MiB.');
  const bytes = await readFile(codePath);
  if (bytes.length > 2 * 1024 * 1024) fail('Generated source exceeds 2 MiB.');
  let code; try { code = new TextDecoder('utf-8', { fatal: true }).decode(bytes); } catch { fail('Generated source must be UTF-8 text.'); }
  if (!code.trim() || code.includes('\u0000')) fail('Generated source must be non-empty UTF-8 text.');
  const live = await controller.session.document();
  if (live.version !== job.expectedVersion) fail('The design changed after this request. The previous preview was retained.');
  job.code = code; job.codePath = codePath; job.status = 'complete'; job.revision++;
  return { ...summary(job), codePath };
}

export function pollCodegen(controller, { afterRevision } = {}) {
  const job = controller.codegenJob;
  if (!job || job.revision === afterRevision) return null;
  return { ...summary(job), code: job.code, codePath: job.codePath };
}

export function cancelCodegen(controller, { uiRequestId }) {
  const job = controller.codegenJob;
  if (job?.uiRequestId === uiRequestId && job.status !== 'cancelled') {
    job.status = 'cancelled'; job.revision++;
  }
  return summary(job);
}
