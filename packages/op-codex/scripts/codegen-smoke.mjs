import assert from 'node:assert/strict';
import { mkdir, mkdtemp, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { CanvasController } from '../plugins/openpencil/scripts/canvas-controller.mjs';

const artifacts = resolve('artifacts'); await mkdir(artifacts,{recursive:true});
const output = await mkdtemp(join(artifacts,'codegen-'));
const filePath = join(output,'design.op');
await writeFile(filePath,JSON.stringify({version:'0.8.5',children:[{
  id:'card',type:'frame',name:'Code panel QA',x:40,y:40,width:640,height:400,
  fill:[{type:'solid',color:'#F5F3ED'}],children:[
    {id:'title',type:'text',name:'Title',x:40,y:50,width:550,height:65,content:'Less friction. More focus.',fontFamily:'Inter',fontSize:34,fontWeight:600,fill:[{type:'solid',color:'#252721'}]},
    {id:'body',type:'text',name:'Description',x:42,y:135,width:540,height:50,content:'One design, ready for your next implementation.',fontFamily:'Inter',fontSize:17,fill:[{type:'solid',color:'#64685F'}]},
    {id:'button',type:'frame',name:'Start button',x:42,y:245,width:180,height:52,cornerRadius:12,fill:[{type:'solid',color:'#315748'}],children:[
      {id:'label',type:'text',name:'Button label',x:24,y:15,width:138,height:24,content:'Start your project',fontFamily:'Inter',fontSize:15,fontWeight:600,fill:[{type:'solid',color:'#FFFFFF'}]},
    ]},
  ],
}]}));
const controller = new CanvasController();
const call = async (name,args={}) => {
  const reply = await controller.dispatch({jsonrpc:'2.0',id:1,method:'tools/call',params:{name,arguments:args}});
  if (reply.error || reply.result?.isError) throw new Error(reply.error?.message || reply.result.content[0].text);
  return reply.result;
};
try {
  const opened = await controller.open({filePath});
  for (const [index,framework,extension] of [[1,'html','html'],[2,'react','tsx'],[3,'vue','vue']]) {
    const live = await controller.session.document();
    const prepared = await call('canvas_codegen_prepare',{uiRequestId:`host-123-${index}`,framework,pageId:'0',nodeIds:['card'],docJson:JSON.stringify(live.document)});
    const job = prepared.structuredContent;
    assert.ok(job.code.includes('Less friction'));
    const sourcePath = join(output,`component.${extension}`); await writeFile(sourcePath,job.code);
    const revision = job.revision;
    await call('canvas_codegen_complete',{requestId:job.requestId,codePath:sourcePath});
    const result = (await call('canvas_codegen_poll',{afterRevision:revision})).structuredContent;
    assert.equal((await call('canvas_codegen_status')).structuredContent.status,'complete');
    assert.equal(result.code,job.code);
  }
  const job = controller.codegenJob;
  await call('update_node',{nodeId:'title',data:{content:'Updated design'}});
  await assert.rejects(call('canvas_codegen_complete',{requestId:job.requestId,codePath:join(output,'component.vue')}),/design changed/);
  console.log(JSON.stringify({output,panelUrl:opened.structuredContent.panelUrl,frameworks:['html','react','vue'],staleCompletionRejected:true}));
  console.log('Native code export / captured target / primary-source return / stale design smoke: OK');
} finally { await controller.close(); }
