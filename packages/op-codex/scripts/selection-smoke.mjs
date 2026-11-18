import assert from 'node:assert/strict';
import { mkdir, mkdtemp, writeFile, readFile } from 'node:fs/promises';
import { isAbsolute, join } from 'node:path';
import { CanvasController } from '../plugins/openpencil/scripts/canvas-controller.mjs';

const options = {};
for (let i = 2; i < process.argv.length; i += 2) options[process.argv[i].replace(/^--/,'')] = process.argv[i+1];
if (![options.runtime,options.output].every(value=>typeof value==='string' && isAbsolute(value))) {
  throw new Error('Usage: node scripts/selection-smoke.mjs --runtime /absolute/runtime --output /absolute/evidence');
}
await mkdir(options.output,{recursive:true});
const output = await mkdtemp(join(options.output,'run-'));
process.env.OPENPENCIL_CODEX_STATE_DIR = join(output,'state');
const filePath = join(output,'selection-test.op');
const fill = color => [{type:'solid',color}];
const text = (id,name,content,x) => ({id,name,type:'text',x,y:34,width:232,height:42,fontFamily:'DM Sans',fontSize:26,fontWeight:600,content,fill:fill('#25271F')});
const initial = {version:'0.8.5',children:[{id:'board',name:'Chat selection test',type:'frame',x:0,y:0,width:800,height:400,fill:fill('#F3F2EC'),children:[
  {id:'card-a',name:'Selected card',type:'frame',x:36,y:88,width:344,height:224,cornerRadius:20,fill:fill('#D5DEA6'),children:[text('title-a','Selected title','Morning focus',28)]},
  {id:'card-b',name:'Untouched card',type:'frame',x:420,y:88,width:344,height:224,cornerRadius:20,fill:fill('#E4E1D4'),children:[text('title-b','Untouched title','Evening review',28)]},
]}]};
await writeFile(filePath,JSON.stringify(initial,null,2));
const controllers = new Set();
const report = {status:'FAIL',output,filePath,selectionIngress:'native MCP set_selection_set; browser pointer gestures not exercised',cases:[]};
let id = 0;
async function open() {
  const controller = new CanvasController({runtimeRoot:options.runtime}); controllers.add(controller);
  await controller.open({filePath}); return controller;
}
async function tool(controller,name,args={}) {
  assert.ok(controller.session,'Never route this test to a user editor');
  const reply = await controller.dispatch({jsonrpc:'2.0',id:++id,method:'tools/call',params:{name,arguments:args}});
  if (reply.error || reply.result.isError) throw new Error(`${name}: ${reply.error?.message || reply.result.content?.[0]?.text}`);
  return reply.result;
}
const find = (document,id) => {
  const visit = node => node.id === id ? node : (node.children || []).map(visit).find(Boolean);
  return document.children.map(visit).find(Boolean);
};
async function select(controller,ids) { await tool(controller,'set_selection_set',{node_ids:ids.join(',')}); }
async function capture(controller,name) {
  const result = await tool(controller,'get_screenshot',{nodeId:'board'});
  const image = result.content.find(item=>item.type==='image');
  assert.ok(image?.data);
  await writeFile(join(output,name),Buffer.from(image.data,'base64'));
}
try {
  let controller = await open();
  const baseline = (await controller.session.document()).document;
  const otherBefore = find(baseline,'card-b');
  await capture(controller,'before.png');
  await select(controller,['card-a']);
  const selected = (await tool(controller,'canvas_selection')).structuredContent;
  assert.deepEqual(selected.selection.nodeIds,['card-a']);
  // A chat edit resolves its target through canvas_selection before editing.
  const captured = selected.selection;
  await writeFile(join(output,'selection-scope.json'),JSON.stringify(selected,null,2));
  await select(controller,['card-b']);
  assert.deepEqual(captured.nodeIds,['card-a']);
  const read = await tool(controller,'batch_get',{nodeIds:captured.nodeIds,readDepth:3});
  assert.ok(JSON.stringify(read).includes('title-a'));
  await tool(controller,'update_node',{nodeId:captured.nodeIds[0],data:{fill:fill('#233C78')}});
  await tool(controller,'update_node',{nodeId:'title-a',data:{content:'Deep focus',fill:fill('#FFFFFF')}});
  let value = (await controller.session.document()).document;
  assert.equal(find(value,'title-a').content,'Deep focus');
  assert.deepEqual(find(value,'card-b'),otherBefore);
  await capture(controller,'selected-edit.png');
  await tool(controller,'canvas_save',{filePath});
  report.cases.push({name:'single-card-with-descendant',passed:true,untouchedSibling:true,retargetAfterSelectionChange:false});
  await select(controller,['title-a']);
  const nested = (await tool(controller,'canvas_selection')).structuredContent.selection;
  assert.equal(nested.nodes[0].parentId,'card-a');
  await tool(controller,'update_node',{nodeId:'title-a',data:{content:'Focus, restored'}});
  assert.deepEqual(find((await controller.session.document()).document,'card-b'),otherBefore);
  report.cases.push({name:'nested-title',passed:true,parentId:nested.nodes[0].parentId});
  await select(controller,['title-a','title-b']);
  const multi = (await tool(controller,'canvas_selection')).structuredContent.selection;
  assert.deepEqual(multi.nodeIds,['title-a','title-b']);
  const cardBefore = structuredClone(find((await controller.session.document()).document,'card-b'));
  for (const nodeId of multi.nodeIds) await tool(controller,'update_node',{nodeId,data:{fontSize:30}});
  value = (await controller.session.document()).document;
  cardBefore.children[0].fontSize = 30;
  assert.deepEqual(find(value,'card-b'),cardBefore);
  report.cases.push({name:'multi-selection',passed:true,nodeIds:multi.nodeIds});
  await select(controller,[]);
  assert.equal((await tool(controller,'canvas_selection')).structuredContent.selection.count,0);
  await select(controller,['card-b']);
  assert.deepEqual((await tool(controller,'canvas_selection')).structuredContent.selection.nodeIds,['card-b']);
  report.cases.push({name:'empty-and-current-selection',passed:true});
  await capture(controller,'after.png');
  const saved = (await tool(controller,'canvas_save',{filePath})).structuredContent;
  assert.ok((await readFile(filePath,'utf8')).includes('Focus, restored'));
  await controller.close(); controllers.delete(controller);
  controller = await open();
  const reopened = (await controller.session.document()).document;
  assert.deepEqual(reopened,value);
  await capture(controller,'reopened.png');
  report.cases.push({name:'save-and-reopen',passed:true,sha256:saved.sha256});
  report.status = 'PASS';
} catch (error) { report.error = error.message; process.exitCode = 1; }
finally {
  for (const controller of controllers) await controller.close();
  await writeFile(join(output,'report.json'),JSON.stringify(report,null,2));
  console.log(JSON.stringify(report,null,2));
}
