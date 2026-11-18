import test from 'node:test';
import assert from 'node:assert/strict';
import { createUiHarness } from './ui-harness.mjs';

const transition = '选区或页面正在切换，请确认选中的元素后重试。';
const selected = (node = 'title', pageId = '0') => ({pageId,nodeIds:[node],count:1,nodes:[{id:node,name:node,type:'text'}]});
async function panel() {
  const ui = await createUiHarness('panel');
  ui.host({type:'codex-canvas/theme',colorScheme:'light'});
  ui.editor({type:'op-bridge/ready',generation:1,revision:0});
  ui.rpc(() => ({version:7,selection:selected()}));
  await ui.state('refreshCanvas()');
  return ui;
}
function flush(ui) {
  const request = ui.editorMessages.findLast(item => item.type === 'op-bridge/snapshot');
  ui.editor({type:'op-bridge/snapshot-result',requestId:request.requestId,docJson:'{}',generation:1,revision:0});
}

test('Selection/page transition polling recovers automatically, including older bridge responses', async () => {
  for (const code of ['SELECTION_TRANSITION',undefined]) {
    const ui = await panel();
    ui.rpc(() => ({error:transition,code}),{raw:true});
    await ui.state('refreshCanvas()');
    assert.equal(ui.element('error').hidden,true);
    assert.equal(ui.element('selection-label').textContent,'正在同步选区…');
    assert.notEqual(ui.element('status').textContent,'画布连接中断');
    assert.equal(ui.hostMessages.at(-1).data.selection,null,'The host must not retain a stale target');
    ui.rpc(() => ({version:7,selection:selected('next-title','page-2')}));
    await ui.state('refreshCanvas()');
    assert.match(ui.element('selection-label').textContent,/next-title/);
    assert.equal(ui.element('error').hidden,true);
    assert.ok(ui.rpcCalls.every(call => ['canvas_status'].includes(JSON.parse(call.options.body).name)));
  }
});

test('Real connection failures remain visible and clear when the canvas reconnects', async () => {
  const ui = await panel();
  ui.rpc(() => { throw new Error('Runtime disconnected'); });
  await ui.state('refreshCanvas()');
  assert.equal(ui.element('error').hidden,false);
  assert.equal(ui.element('error-message').textContent,'Runtime disconnected');
  assert.equal(ui.element('status').textContent,'画布连接中断');
  ui.rpc(() => ({version:7,selection:selected()}));
  await ui.state('refreshCanvas()');
  assert.equal(ui.element('error').hidden,true);
  assert.equal(ui.element('status').textContent,'画布已连接');
  assert.match(ui.element('selection-label').textContent,/title/);
});

test('Successful selection polls cannot erase failed saves or sync conflict warnings', async () => {
  const ui = await panel();
  ui.rpc(({name}) => {
    if (name === 'canvas_save') throw new Error('Source changed outside the canvas');
    return {version:8,selection:selected()};
  });
  const saving = ui.state('save()'); flush(ui); await saving;
  assert.equal(ui.element('error-message').textContent,'Source changed outside the canvas');
  await ui.state('refreshCanvas()');
  assert.equal(ui.element('error').hidden,false);
  assert.equal(ui.element('status').textContent,'保存未完成');
  ui.rpc(() => { throw new Error('Offline'); }); await ui.state('refreshCanvas()');
  assert.equal(ui.element('error-message').textContent,'Source changed outside the canvas');
  ui.editor({type:'op-bridge/sync-conflict'});
  ui.rpc(() => ({version:8,selection:selected()})); await ui.state('refreshCanvas()');
  assert.equal(ui.element('error').hidden,false);
  assert.match(ui.element('error-message').textContent,/同步过程中发生变化/);
  assert.equal(ui.element('status').textContent,'同步冲突');
});

test('Deleted selection clears stale context and recovers when another element is selected', async () => {
  const ui = await panel();
  ui.rpc(() => ({error:'选中的元素已不存在，请重新选择。',code:'SELECTION_STALE'}),{raw:true});
  await ui.state('refreshCanvas()');
  assert.equal(ui.element('selection-label').textContent,'请重新选择元素');
  assert.equal(ui.hostMessages.at(-1).data.selection,null);
  assert.equal(ui.element('error').hidden,true);
  ui.rpc(() => ({version:7,selection:selected('other')}));
  await ui.state('refreshCanvas()');
  assert.match(ui.element('selection-label').textContent,/other/);
  assert.ok(ui.rpcCalls.every(call=>JSON.parse(call.options.body).name==='canvas_status'));
});

test('Dismissed persistent warnings stay closed across identical polls and reappear for a new failure', async () => {
  const ui = await panel();
  ui.rpc(() => { throw new Error('Offline'); }); await ui.state('refreshCanvas()');
  ui.element('error-dismiss').onclick();
  assert.equal(ui.element('error').hidden,true);
  await ui.state('refreshCanvas()'); assert.equal(ui.element('error').hidden,true);
  assert.equal(ui.element('status').textContent,'画布连接中断');
  ui.rpc(() => { throw new Error('Runtime ended'); }); await ui.state('refreshCanvas()');
  assert.equal(ui.element('error').hidden,false);
  assert.equal(ui.element('error-message').textContent,'Runtime ended');
});
