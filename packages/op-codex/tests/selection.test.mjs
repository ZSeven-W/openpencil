import test from 'node:test';
import assert from 'node:assert/strict';
import http from 'node:http';
import { readCanvasSelection } from '../plugins/openpencil/scripts/canvas-selection.mjs';
import { CanvasController, canvasTools } from '../plugins/openpencil/scripts/canvas-controller.mjs';
import { EditorSession } from '../plugins/openpencil/scripts/editor-session.mjs';
import { createUiHarness, initializeApp, openPanel, settle } from './ui-harness.mjs';

const live = () => ({ version: 7, activePageIndex: 0, document: { version:'0.8.5', children:[
  {id:'card',name:'Card',type:'frame',children:[{id:'title',type:'text',name:'Headline',content:'Hello'}]},
  {id:'other',name:'Other card',type:'frame'},
] } });
const raw = (ids = 'title', pageId = '0', pageIndex = 0) => ({ active_page_id:pageId, active_page_index:String(pageIndex), selection_ids:ids });
function session(states = [raw()], value = live()) {
  let index = 0;
  return { source:'/design.op', draft:'/recovery.op', document:async () => value,
    forward:async () => ({result:{content:[{type:'text',text:JSON.stringify(states[Math.min(index++,states.length-1)])}]}}) };
}
const selection = ids => ({pageId:'0',pageIndex:0,nodeIds:ids,count:ids.length,nodes:ids.map(id=>({id,name:id,type:'frame'}))});

test('Selection resolves nested IDs on the active page without leaking document content', async () => {
  const current = await readCanvasSelection(session([raw('title,card,title,0')]));
  assert.deepEqual(current.selection.nodeIds,['card','title']);
  assert.equal(current.selection.nodes[1].parentId,'card');
  assert.equal(current.selection.nodes[1].name,'Headline');
  assert.ok(!JSON.stringify(current.selection).includes('Hello'));
  const empty = await readCanvasSelection(session([raw('')]));
  assert.equal(empty.selection.count,0);
  assert.deepEqual(empty.selection.nodes,[]);
  const multi = live();
  multi.document = {version:'0.8.5', pages:[{id:'p1',children:[{id:'other',type:'frame'}]},{id:'p2',children:[{id:'title',type:'text',name:'Page two'}]}]};
  multi.activePageIndex = 1;
  assert.equal((await readCanvasSelection(session([raw('title','p2',1)],multi))).selection.nodes[0].name,'Page two');
});

test('Missing nodes, page switches and failed selection reads cannot broaden an edit', async () => {
  await assert.rejects(readCanvasSelection(session([raw('deleted')])),error => error.code === 'SELECTION_STALE');
  await assert.rejects(readCanvasSelection(session([raw('title'),raw('other')])),error => error.code === 'SELECTION_TRANSITION');
  await assert.rejects(readCanvasSelection(session([raw('title','other-page',1)])),/切换/);
  await assert.rejects(readCanvasSelection({forward:async()=>({result:{isError:true}})}),/Cannot read/);
});

test('Managed selection reads are available to Codex without preparing an edit request', async () => {
  const controller = new CanvasController(); controller.session = session();
  const read = await controller.call('canvas_selection',{});
  assert.deepEqual(read.structuredContent.selection.nodeIds,['title']);
  assert.equal(read.structuredContent.version,7);
  assert.equal('editRequest' in read.structuredContent,false);
  assert.ok(!canvasTools.some(tool=>['canvas_prepare_edit','canvas_edit_status'].includes(tool.name)));
});

test('Selection transition codes survive both tool and panel transports without replaying a read', async t => {
  const controller = new CanvasController();
  const message = {jsonrpc:'2.0',id:1,method:'tools/call',params:{name:'canvas_status'}};
  controller.session = session([raw('title'),raw('other')]);
  const result = await controller.dispatch(message);
  assert.equal(result.result.structuredContent.code,'SELECTION_TRANSITION');
  controller.session = session([raw('title'),raw('other')]);
  const panel = await controller.ensurePanel(); t.after(() => panel.close());
  const response = await fetch(panel.url + '/rpc',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({name:'canvas_status'})});
  assert.equal(response.status,400);
  assert.equal((await response.json()).code,'SELECTION_TRANSITION');
});

test('Selection polling reuses unchanged document images and invalidates on version or page changes', async t => {
  let value = live(), reads = 0;
  const server = http.createServer((req,res) => {
    res.setHeader('Content-Type','application/json');
    if (req.url === '/api/mcp/version') res.end(JSON.stringify({version:value.version}));
    else { reads++; res.end(JSON.stringify(value)); }
  });
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  t.after(()=>{server.closeAllConnections();return new Promise(resolve=>server.close(resolve));});
  const active = Object.assign(Object.create(EditorSession.prototype),{port:server.address().port});
  await active.selectionDocument(0); await active.selectionDocument(0); assert.equal(reads,1);
  value.version++; await active.selectionDocument(0); assert.equal(reads,2);
  value.activePageIndex = 1; await active.selectionDocument(1); assert.equal(reads,3);
});

function panelMessage(app, url, data, origin = new URL(url).origin) {
  app.window.emit('message',{source:app.frame.contentWindow,origin,data});
}
test('Selection context is synchronized, deduplicated and cleared without sending a chat prompt', async () => {
  const app = await createUiHarness('app');
  await initializeApp(app,'light',{serverTools:{},message:{text:{}},updateModelContext:{text:{}}});
  const url = await openPanel(app);
  const value = {type:'codex-canvas/context',filePath:'/design.op',selection:selection(['title'])};
  panelMessage(app,url,value); panelMessage(app,url,value);
  const updates = () => app.hostMessages.filter(item=>item.data.method==='ui/update-model-context');
  assert.equal(updates().length,1);
  assert.deepEqual(JSON.parse(updates()[0].data.params.content[0].text).selection.nodeIds,['title']);
  panelMessage(app,url,{...value,selection:selection([])});
  assert.equal(updates().length,2);
  panelMessage(app,url,{...value,selection:null});
  assert.equal(updates().length,3);
  assert.equal(JSON.parse(updates()[2].data.params.content[0].text).selection,null);
  panelMessage(app,url,{type:'codex-canvas/prompt',requestId:'obsolete',message:'Change'});
  await settle();
  assert.ok(!app.hostMessages.some(item=>item.data.method==='ui/message'));
});

test('Failed context updates can resynchronize, while unsupported hosts send no context', async () => {
  const app = await createUiHarness('app'); await initializeApp(app,'light');
  const url = await openPanel(app);
  const value = {type:'codex-canvas/context',filePath:'/design.op',selection:selection(['title'])};
  panelMessage(app,url,value);
  const first = app.hostMessages.at(-1).data;
  app.host({jsonrpc:'2.0',id:first.id,error:{message:'Unavailable'}}); await settle();
  panelMessage(app,url,value);
  assert.equal(app.hostMessages.filter(item=>item.data.method==='ui/update-model-context').length,2);
  const unsupported = await createUiHarness('app'); await initializeApp(unsupported,'light',{serverTools:{}});
  panelMessage(unsupported,await openPanel(unsupported),value);
  assert.ok(!unsupported.hostMessages.some(item=>item.data.method==='ui/update-model-context'));
});

async function readyPanel(embedded = true) {
  const ui = await createUiHarness('panel',{embedded});
  ui.editor({type:'op-bridge/ready',generation:1,revision:0});
  ui.rpc(() => ({selection:selection(['title']),version:7}));
  await ui.state('refreshCanvas()');
  return ui;
}

test('Selection reminder follows single, multiple and cleared targets without an edit workflow', async () => {
  for (const embedded of [true,false]) {
    const ui = await readyPanel(embedded);
    assert.equal(ui.element('selection-label').textContent,'已选 1 个元素 · title');
    ui.rpc(() => ({selection:selection(['card','title']),version:7}));
    await ui.state('refreshCanvas()');
    assert.equal(ui.element('selection-label').textContent,'已选 2 个元素 · card、title');
    ui.rpc(() => ({selection:selection([]),version:7}));
    await ui.state('refreshCanvas()');
    assert.equal(ui.element('selection-label').textContent,'未选中元素');
    assert.equal(ui.editorMessages.filter(item=>item.type==='op-bridge/snapshot').length,0);
    assert.ok(ui.rpcCalls.every(item=>JSON.parse(item.options.body).name==='canvas_status'));
    assert.ok(!ui.hostMessages.some(item=>item.data.type==='codex-canvas/prompt'));
  }
});

test('The panel rejects removed edit controls without dispatching them', async t => {
  const controller = new CanvasController();
  const panel = await controller.ensurePanel(); t.after(() => panel.close());
  let dispatched = false;
  controller.call = async () => { dispatched = true; };
  for (const name of ['canvas_prepare_edit','canvas_edit_status']) {
    const response = await fetch(panel.url+'/rpc',{method:'POST',body:JSON.stringify({name})});
    assert.equal(response.status,403);
  }
  assert.equal(dispatched,false);
});

test('Fast selection polling cannot starve the recovery checkpoint timer', async () => {
  const ui = await readyPanel();
  ui.state('checkpointSoon()'); const timerId = ui.state('checkpointTimer');
  ui.state('checkpointSoon()'); assert.equal(ui.state('checkpointTimer'),timerId);
  ui.timers.get(timerId)(); await settle();
  assert.ok(ui.rpcCalls.some(item=>JSON.parse(item.options.body).name==='canvas_checkpoint'));
  assert.equal(ui.state('checkpointTimer'),undefined);
});
