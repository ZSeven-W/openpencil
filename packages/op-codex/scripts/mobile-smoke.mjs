import { mkdir, mkdtemp, writeFile } from 'node:fs/promises';
import { isAbsolute, join } from 'node:path';
import { CanvasController } from '../plugins/openpencil/scripts/canvas-controller.mjs';

const usage = 'node scripts/mobile-smoke.mjs --runtime /absolute/runtime/root --output /absolute/evidence-dir';
const options = {};
for (let i = 2; i < process.argv.length; i++) {
  const key = process.argv[i];
  if (key === '--help') { console.log(usage); process.exit(0); }
  if (!['--runtime', '--output'].includes(key) || !process.argv[i + 1]) throw new Error(usage);
  options[key.slice(2)] = process.argv[++i];
}
if (![options.runtime, options.output].every(value => typeof value === 'string' && isAbsolute(value))) {
  throw new Error(`Both paths must be absolute. Usage: ${usage}`);
}
await mkdir(options.output, { recursive: true });
const output = await mkdtemp(join(options.output, 'run-'));
// Keep drafts and runtime settings inside this run. Never read or write the
// user's global runtime configuration, discovery record, or live document.
process.env.OPENPENCIL_CODEX_STATE_DIR = join(output, 'state');
let requestId = 0;
const sessions = new Set();
const cases = [
  { name:'phone-375', width:375, height:812, mobile:true },
  { name:'phone-390', width:390, height:844, mobile:true },
  { name:'phone-430', width:430, height:932, mobile:true },
  { name:'desktop-1440', width:1440, height:900, mobile:false },
];
const report = { status:'FAIL', runtime:options.runtime, output, isolation:'managed sessions with run-local state',
  visualInspection:'not-performed', cases:[] };

async function json(path, value) {
  await writeFile(path, JSON.stringify(value, null, 2) + '\n', { flag:'wx', mode:0o600 });
}
function content(result) {
  if (result.structuredContent) return result.structuredContent;
  const text = result.content?.find(item => item.type === 'text')?.text;
  if (!text) return {};
  try { return JSON.parse(text); } catch { return { text }; }
}
async function tool(controller, name, args = {}) {
  // open() must succeed first, so dispatch cannot fall back to discovery of a
  // user's native editor. Every mutation is sent exactly once.
  if (!controller.session) throw new Error('Refusing a tool call without an isolated session.');
  const reply = await controller.dispatch({ jsonrpc:'2.0', id:++requestId, method:'tools/call',
    params:{ name, arguments:args } });
  if (reply?.error || reply?.result?.isError) throw new Error(`${name} returned an error; the request was not retried.`);
  const result = reply?.result;
  if (!result) throw new Error(`${name} returned no result.`);
  const value = content(result);
  if (value.errors && value.errors !== '[]' && value.errors.length !== 0) {
    throw new Error(`${name} reported operation errors; the request was not retried.`);
  }
  return result;
}
async function open(filePath) {
  const controller = new CanvasController({ runtimeRoot:options.runtime });
  sessions.add(controller);
  // Do not dispatch initialize/tools/list before open: an unbound controller
  // intentionally supports discovery, which this smoke test must never use.
  await controller.open(filePath ? { filePath } : {});
  return controller;
}
async function close(controller) {
  await controller.close();
  sessions.delete(controller);
}
function roots(value) {
  const document = value.document;
  return document.pages?.[value.activePageIndex || 0]?.children || document.children || [];
}
function walk(node) {
  return [node, ...(node.children || []).flatMap(walk)];
}
function checkRoot(root, spec, stage) {
  const failures = [];
  const nodes = walk(root);
  const bars = nodes.filter(node => node.role === 'status-bar');
  const directBars = (root.children || []).filter(node => node.role === 'status-bar');
  const check = (condition, reason) => { if (!condition) failures.push(reason); };
  check(root.type === 'frame', 'Root is not a frame.');
  const expectedName = stage === 'created' ? spec.name : `${spec.name} edited`;
  check(root.name === expectedName, 'The expected edit is missing from the live or reopened document.');
  check(root.width === spec.width && root.height === spec.height, 'Authored viewport dimensions changed.');
  check(bars.length === (spec.mobile ? 1 : 0), `Expected ${spec.mobile ? 1 : 0} status-bar roles; found ${bars.length}.`);
  const details = { rootId:root.id, rootName:root.name, width:root.width, height:root.height, statusBarIds:bars.map(node => node.id) };
  if (!spec.mobile) return { passed:failures.length === 0, ...details, failures };
  check(directBars.length === 1, 'Phone root must have exactly one direct status-bar child.');
  const bar = directBars[0];
  const times = (bar?.children || []).filter(node => node.name === 'Time');
  const levels = (bar?.children || []).filter(node => node.name === 'Levels');
  const clocks = nodes.filter(node => node.type === 'text' && node.name === 'Time' &&
    typeof node.content === 'string' && /^\d{1,2}:\d{2}$/.test(node.content));
  check(times.length === 1, 'Canonical Time container is missing or duplicated.');
  check(levels.length === 1, 'Canonical Levels container is missing or duplicated.');
  check(clocks.length === 1 && times[0] && walk(times[0]).some(node => node.id === clocks[0]?.id),
    'Expected exactly one clock label inside Time.');
  const indicators = levels[0]?.children || [];
  for (const name of ['Cellular Connection', 'Wifi', 'Battery']) {
    const found = indicators.filter(node => node.name === name);
    check(found.length === 1, `${name} is missing or duplicated in Levels.`);
    if (name !== 'Battery') check(found[0]?.type === 'path' && typeof found[0].d === 'string' && found[0].d.length > 0,
      `${name} must contain the canonical vector path.`);
    else for (const part of ['Border', 'Cap', 'Capacity']) {
      check((found[0]?.children || []).filter(node => node.name === part).length === 1,
        `Battery ${part} is missing or duplicated.`);
    }
  }
  return { passed:failures.length === 0, ...details, clockCount:clocks.length,
    indicatorNames:indicators.map(node => node.name), failures };
}
async function screenshot(controller, rootId, filePath) {
  const result = await tool(controller, 'get_screenshot', { nodeId:rootId });
  const image = result.content?.find(item => item.type === 'image' && item.mimeType === 'image/png');
  if (!image?.data) throw new Error('get_screenshot returned no PNG image.');
  const bytes = Buffer.from(image.data, 'base64');
  if (bytes.length < 24 || bytes.subarray(0, 8).toString('hex') !== '89504e470d0a1a0a') {
    throw new Error('get_screenshot returned invalid PNG bytes.');
  }
  await writeFile(filePath, bytes, { flag:'wx', mode:0o600 });
  return { filePath, width:bytes.readUInt32BE(16), height:bytes.readUInt32BE(20), bytes:bytes.length };
}
async function inspect(controller, spec, directory, stage, rootId) {
  const value = await controller.session.document();
  await json(join(directory, `${stage}-document.json`), value);
  const root = roots(value).find(node => rootId ? node.id === rootId : node.name === spec.name);
  if (!root) throw new Error(`Root missing during ${stage}; document evidence was preserved.`);
  const readback = content(await tool(controller, 'batch_get', { nodeIds:[root.id], readDepth:8 }));
  await json(join(directory, `${stage}-readback.json`), readback);
  const checked = { stage, ...checkRoot(root, spec, stage) };
  try { checked.screenshot = await screenshot(controller, root.id, join(directory, `${stage}.png`)); }
  catch (error) { checked.passed = false; checked.failures.push(`Screenshot: ${error.message}`); }
  await json(join(directory, `${stage}-check.json`), checked);
  return checked;
}

async function runCase(spec) {
  const directory = join(output, spec.name);
  await mkdir(directory);
  const result = { name:spec.name, viewport:`${spec.width}x${spec.height}`, expectedStatusBars:spec.mobile ? 1 : 0,
    status:'FAIL', directory, stages:[] };
  report.cases.push(result);
  let controller;
  try {
    controller = await open();
    result.runtimeVersion = controller.session.version;
    const guidelines = content(await tool(controller, 'get_guidelines', { topic:spec.mobile ? 'mobile' : 'dashboard' }));
    await json(join(directory, 'guidelines.json'), guidelines);
    // No status-bar nodes or icons are authored here: this tests runtime seeding.
    await tool(controller, 'batch_design', { script:`I(null, ${JSON.stringify({ type:'frame', name:spec.name,
      width:spec.width, height:spec.height, layout:'vertical', fill:'#F7F8F3', children:[] })});` });
    const created = await inspect(controller, spec, directory, 'created');
    result.stages.push(created);
    await tool(controller, 'update_node', { nodeId:created.rootId, data:{ name:`${spec.name} edited` } });
    result.stages.push(await inspect(controller, spec, directory, 'edited', created.rootId));
    const filePath = join(directory, 'saved.op');
    result.saved = content(await tool(controller, 'canvas_save', { filePath }));
    await close(controller); controller = null;
    controller = await open(filePath);
    result.stages.push(await inspect(controller, spec, directory, 'reopened', created.rootId));
    result.status = result.stages.every(stage => stage.passed) ? 'PASS' : 'FAIL';
  } catch (error) {
    result.error = error.message;
    // A timed-out mutation may have applied. Preserve a read-only snapshot;
    // never resend the creation or edit to make the test pass.
    if (controller?.session && !controller.session.closed) {
      try { await json(join(directory, 'failure-document.json'), await controller.session.document()); }
      catch { result.failureEvidenceUnavailable = true; }
    }
  } finally {
    if (controller) {
      try { await close(controller); }
      catch (error) { result.status = 'FAIL'; result.closeError = error.message; }
    }
    await json(join(directory, 'result.json'), result);
  }
}
try {
  for (const spec of cases) await runCase(spec);
  report.status = report.cases.every(result => result.status === 'PASS') ? 'PASS' : 'FAIL';
} catch (error) {
  report.error = error.message;
} finally {
  for (const controller of sessions) {
    try { await controller.close(); } catch { report.status = 'FAIL'; report.cleanupFailed = true; }
  }
  await json(join(output, 'report.json'), report);
}
console.log(JSON.stringify({ status:report.status, output, report:join(output, 'report.json'),
  cases:report.cases.map(result => {
    const failures = result.stages.flatMap(stage => stage.failures);
    return { name:result.name, status:result.status,
      statusBars:result.stages.map(stage => stage.statusBarIds.length),
      failureCount:failures.length, failures:[...new Set(failures)].slice(0, 3),
      ...(result.error ? { error:result.error } : {}) };
  }) }, null, 2));
process.exitCode = report.status === 'PASS' ? 0 : 1;
