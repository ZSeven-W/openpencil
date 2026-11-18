import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, writeFile, rm, symlink } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { CanvasController, canvasTools } from '../plugins/openpencil/scripts/canvas-controller.mjs';
import { prepareCodegen, completeCodegen, pollCodegen, cancelCodegen } from '../plugins/openpencil/scripts/canvas-codegen.mjs';
import { createUiHarness, initializeApp, openPanel, settle } from './ui-harness.mjs';

function fixture() {
  const live = {version:7,activePageIndex:0,document:{version:'0.8.5',pages:[
    {id:'p1',children:[{id:'card',type:'frame',children:[{id:'title',type:'text',content:'Hello'}]}]},
    {id:'p2',children:[{id:'other',type:'frame'}]},
  ]}};
  const calls = []; let selected = 'card';
  const controller = new CanvasController();
  controller.session = {source:'/design.op',draft:'/recovery.op',document:async()=>structuredClone(live),
    forward:async message => {
      calls.push(message.params);
      if (message.params.name === 'get_editor_state') return {result:{structuredContent:{active_page_id:'p1',active_page_index:'0',selection_ids:selected}}};
      return {result:{structuredContent:{code:'<main>Hello</main>'}}};
    }};
  const args = {uiRequestId:'host-123-1',framework:'html',pageId:'p1',nodeIds:['card'],docJson:JSON.stringify(live.document)};
  return {controller,args,live,calls,select:value=>{selected=value;}};
}

test('Code request fixes page, nodes, framework and version; duplicates never export again', async () => {
  const {controller,args,calls,select} = fixture();
  const job = await prepareCodegen(controller,args);
  assert.equal(job.expectedVersion,7); assert.equal(job.framework,'html');
  assert.deepEqual(job.nodeIds,['card']); assert.equal(job.pageId,'p1');
  assert.equal(job.code,'<main>Hello</main>');
  select('title');
  assert.equal(await prepareCodegen(controller,args),job);
  assert.equal(calls.filter(call=>call.name==='codegen_export').length,1);
  assert.equal(pollCodegen(controller,{afterRevision:job.revision}),null);
  assert.equal((await controller.call('canvas_codegen_status',{})).structuredContent.requestId,job.requestId);
  assert.ok(job.prompt.includes('Do not modify the design'));
  assert.deepEqual(canvasTools.find(tool=>tool.name==='canvas_codegen_prepare')._meta.ui.visibility,['app']);
});

test('Empty selection exports only active-page roots, and stale targets/snapshots never hand off', async () => {
  const f = fixture(); f.select('');
  assert.deepEqual((await prepareCodegen(f.controller,f.args)).nodeIds,['card']);
  for (const change of [{pageId:'p2'},{nodeIds:['other']},{docJson:'{}'},{uiRequestId:'host-1'},{framework:'unknown'}]) {
    const fresh = fixture();
    await assert.rejects(prepareCodegen(fresh.controller,{...fresh.args,...change}));
    assert.ok(!fresh.calls.some(call=>call.name==='codegen_export'));
    assert.equal(fresh.controller.codegenJob,undefined);
  }
  const fresh = fixture(); const original = fresh.controller.session.forward;
  fresh.controller.session.forward = async message => { const out = await original(message); if (message.params.name==='codegen_export') fresh.live.version++; return out; };
  await assert.rejects(prepareCodegen(fresh.controller,fresh.args),/changed during/);
  assert.equal(fresh.controller.codegenJob,undefined);
});

test('Verified source returns only to the active, unchanged request with the correct language', async t => {
  const directory = await mkdtemp(join(tmpdir(),'op-codegen-')); t.after(()=>rm(directory,{recursive:true,force:true}));
  const path = join(directory,'index.html'); await writeFile(path,'<!doctype html><main>Verified source</main>');
  const f = fixture(); const job = await prepareCodegen(f.controller,f.args);
  f.select('title');
  await completeCodegen(f.controller,{requestId:job.requestId,codePath:path});
  assert.equal(job.status,'complete'); assert.equal(job.revision,2);
  assert.equal(pollCodegen(f.controller,{afterRevision:1}).code,'<!doctype html><main>Verified source</main>');
  await assert.rejects(completeCodegen(f.controller,{requestId:'superseded',codePath:path}),/no longer active/);
  await assert.rejects(completeCodegen(f.controller,{requestId:job.requestId,codePath:'relative.html'}),/absolute/);
  const wrong = join(directory,'index.tsx'); await writeFile(wrong,'export default 1');
  await assert.rejects(completeCodegen(f.controller,{requestId:job.requestId,codePath:wrong}),/extension/);
  if (process.platform !== 'win32') {
    const link = join(directory,'link.html'); await symlink(path,link);
    await assert.rejects(completeCodegen(f.controller,{requestId:job.requestId,codePath:link}),/regular file/);
  }
  f.live.version++;
  await assert.rejects(completeCodegen(f.controller,{requestId:job.requestId,codePath:path}),/design changed/);
  assert.equal(job.revision,2);
});

const panelMessage = (app,url,data,origin = new URL(url).origin,source = app.frame.contentWindow) => app.window.emit('message',{source,origin,data});

test('Explicit Code action uses capability-gated ui/message once and waits for acknowledgment', async () => {
  const app = await createUiHarness('app');
  await initializeApp(app,'light',{serverTools:{},message:{text:{}}}); const url = await openPanel(app);
  const value = {type:'codex-canvas/codegen',requestId:'job-1',prompt:'Implement this captured design'};
  panelMessage(app,url,value,'https://foreign.example'); panelMessage(app,url,value,undefined,{});
  assert.ok(!app.hostMessages.some(item=>item.data.method==='ui/message'));
  panelMessage(app,url,value); panelMessage(app,url,value);
  const sent = app.hostMessages.filter(item=>item.data.method==='ui/message');
  assert.equal(sent.length,1); assert.equal(sent[0].data.params.role,'user');
  assert.equal(sent[0].data.params.content[0].text,value.prompt);
  assert.ok(!app.frameMessages.some(item=>item.data.type==='codex-canvas/codegen-result'));
  app.host({jsonrpc:'2.0',id:sent[0].data.id,result:{}}); await settle();
  assert.equal(app.frameMessages.at(-1).data.status,'accepted');
});

test('Unsupported and failed hosts never claim conversion completion or retry a message', async () => {
  for (const supported of [false,true]) {
    const app = await createUiHarness('app');
    await initializeApp(app,'light',{serverTools:{},...(supported?{message:{text:{}}}:{})}); const url = await openPanel(app);
    const value = {type:'codex-canvas/codegen',requestId:'job-1',prompt:'Convert'};
    panelMessage(app,url,value); await settle();
    if (supported) {
      const sent = app.hostMessages.find(item=>item.data.method==='ui/message').data;
      app.host({jsonrpc:'2.0',id:sent.id,error:{message:'Unavailable'}}); await settle();
    }
    assert.equal(app.frameMessages.at(-1).data.status,supported?'unknown':'unsupported');
    panelMessage(app,url,value); await settle();
    assert.equal(app.hostMessages.filter(item=>item.data.method==='ui/message').length,supported?1:0);
  }
});

async function readyCodePanel(embedded) {
  const ui = await createUiHarness('panel',{embedded});
  ui.editor({type:'op-bridge/ready',generation:1,revision:0});
  ui.rpc(message => message.name==='canvas_codegen_prepare'
    ? {requestId:'job-1',uiRequestId:'host-123-1',framework:'html',revision:1,status:'prepared',code:'<main>Baseline</main>',prompt:'Convert'}
    : null);
  return ui;
}
async function generate(ui) {
  ui.editor({type:'op-bridge/codegen-request',requestId:'host-123-1',framework:'html',pageId:'p1',nodeIds:['card']});
  const snapshot = ui.editorMessages.find(item=>item.type==='op-bridge/snapshot');
  ui.editor({type:'op-bridge/snapshot-result',requestId:snapshot.requestId,docJson:'{}'}); await settle();
}

test('Panel flushes first, shows baseline, polls final source, and ignores late handoff acknowledgments', async () => {
  const ui = await readyCodePanel(true); await generate(ui);
  assert.equal(ui.rpcCalls.length,1);
  assert.equal(JSON.parse(ui.rpcCalls[0].options.body).name,'canvas_codegen_prepare');
  assert.equal(ui.editorMessages.at(-1).code,'<main>Baseline</main>');
  assert.equal(ui.hostMessages.at(-1).data.type,'codex-canvas/codegen');
  ui.rpc(()=>({requestId:'job-1',uiRequestId:'host-123-1',status:'complete',revision:2,code:'<main>Tested</main>'}));
  await ui.state('refreshCodegen()');
  assert.equal(ui.editorMessages.at(-1).code,'<main>Tested</main>');
  const count = ui.editorMessages.length;
  ui.host({type:'codex-canvas/codegen-result',requestId:'job-1',status:'accepted'});
  assert.equal(ui.editorMessages.length,count);
  ui.editor({type:'op-bridge/codegen-request',requestId:'host-123-1',framework:'html',nodeIds:['card']}); await settle();
  assert.equal(ui.rpcCalls.length,2);
});

test('Top-level browser returns usable source and an honest chat continuation, without a host submission', async () => {
  const ui = await readyCodePanel(false); await generate(ui);
  assert.ok(ui.editorMessages.at(-1).notice.includes('继续转代码'));
  assert.ok(!ui.hostMessages.some(item=>item.data.type==='codex-canvas/codegen'));
  const before = ui.rpcCalls.length;
  ui.editor({type:'op-bridge/codegen-request',requestId:'host-123-2',framework:'html',nodeIds:[]},'https://foreign.example');
  await settle(); assert.equal(ui.rpcCalls.length,before);
});

test('Cancel during flush sends no implementation prompt and invalidates completion', async () => {
  const ui = await readyCodePanel(true);
  ui.editor({type:'op-bridge/codegen-request',requestId:'host-123-1',framework:'html',pageId:'p1',nodeIds:['card']});
  const snapshot = ui.editorMessages.find(item=>item.type==='op-bridge/snapshot');
  ui.editor({type:'op-bridge/codegen-cancel',requestId:'host-123-1'});
  ui.editor({type:'op-bridge/snapshot-result',requestId:snapshot.requestId,docJson:'{}'}); await settle();
  assert.ok(!ui.hostMessages.some(item=>item.data.type==='codex-canvas/codegen'));
  assert.ok(!ui.rpcCalls.some(item=>JSON.parse(item.options.body).name==='canvas_codegen_prepare'));
  const f = fixture(); const job = await prepareCodegen(f.controller,f.args);
  cancelCodegen(f.controller,{uiRequestId:'other'}); assert.equal(job.status,'prepared');
  cancelCodegen(f.controller,{uiRequestId:job.uiRequestId}); assert.equal(job.status,'cancelled');
  await assert.rejects(completeCodegen(f.controller,{requestId:job.requestId,codePath:'/unused.html'}),/no longer active/);
});
