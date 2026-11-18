import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { mkdir, mkdtemp, writeFile, readFile } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { EditorSession } from '../plugins/openpencil/scripts/editor-session.mjs';
import { CANVAS_URI } from '../plugins/openpencil/scripts/canvas-controller.mjs';

const root=resolve('artifacts'); await mkdir(root,{recursive:true});
const output=await mkdtemp(join(root,'canvas-'));
const filePath=join(output,'design.op');
await writeFile(filePath,JSON.stringify({version:'0.8.5',children:[{type:'frame',id:'canvas-demo',name:'Codex Canvas',x:80,y:80,width:800,height:500,
  fill:[{type:'solid',color:'#F8F6EF'}],children:[
    {type:'text',id:'canvas-headline',name:'Headline',x:52,y:58,width:690,height:64,content:'A canvas for the conversation.',fontSize:36,fontFamily:'DM Sans',fontWeight:600,fill:[{type:'solid',color:'#28291F'}]},
    {type:'text',id:'canvas-subtitle',name:'Subtitle',x:54,y:135,width:650,height:36,content:'Mouse edits. Codex edits. One editable document.',fontSize:18,fontFamily:'DM Sans',fill:[{type:'solid',color:'#767466'}]},
    {type:'rectangle',id:'canvas-card',name:'Accent',x:54,y:218,width:310,height:170,cornerRadius:16,fill:[{type:'solid',color:'#D2BB40'}]},
    {type:'text',id:'canvas-note',name:'Note',x:400,y:235,width:340,height:110,content:'Pan, zoom, edit layers,\nthen save as .op.',fontSize:25,fontFamily:'DM Sans',fill:[{type:'solid',color:'#48493A'}]},
  ]}]},null,2));
const child=spawn(process.execPath,[process.env.OPENPENCIL_TEST_BRIDGE || 'plugins/openpencil/scripts/mcp-bridge.mjs'],{stdio:['pipe','pipe','inherit']});
const pending=new Map(); let id=0;
const outputLines=createInterface({input:child.stdout});
outputLines.on('line',line=>{const value=JSON.parse(line);const request=pending.get(value.id);if(request){pending.delete(value.id);value.error?request.reject(new Error(value.error.message)):request.resolve(value.result);}});
child.on('exit',()=>{for(const p of pending.values())p.reject(new Error('Bridge exited.'));pending.clear();});
function request(method,params={}) {return new Promise((resolve,reject)=>{const current=++id;pending.set(current,{resolve,reject});child.stdin.write(JSON.stringify({jsonrpc:'2.0',id:current,method,params})+'\n');});}
async function tool(name,args={}) {const result=await request('tools/call',{name,arguments:args});if(result.isError)throw new Error(result.content[0].text);return result;}
try {
  await request('initialize',{protocolVersion:'2025-11-25',clientInfo:{name:'canvas-smoke',version:'1'},capabilities:{}});
  const opened=await tool('canvas_open_browser',{filePath});
  const tools=await request('tools/list');
  assert.ok(tools.tools.some(t=>t.name==='batch_design'));
  assert.ok(tools.tools.some(t=>t.name==='open_canvas'));
  assert.ok(tools.tools.some(t=>t.name==='canvas_open_browser'));
  const resource=await request('resources/read',{uri:CANVAS_URI});
  assert.equal(resource.contents[0].mimeType,'text/html;profile=mcp-app');
  assert.equal((await fetch(opened.structuredContent.panelUrl)).status,200);
  console.log(JSON.stringify({panelUrl:opened.structuredContent.panelUrl,filePath,output,toolCount:tools.tools.length},null,2));
  await writeFile(join(output,'session.json'),JSON.stringify(opened.structuredContent,null,2));
  if(process.argv.includes('--hold')) {
    console.log('Commands: inspect, agent-edit, verify-save, quit');
    const input=createInterface({input:process.stdin});
    for await(const line of input) {
      try {
        if(line==='quit')break;
        if(line==='inspect')console.log(JSON.stringify(await tool('batch_get',{readDepth:4})));
        if(line==='agent-edit'){await tool('update_node',{nodeId:'canvas-headline',data:{content:'Edited by Codex — still editable.'}});console.log('Agent edit applied.');}
        if(line==='verify-save'){
          const saved=JSON.parse(await readFile(filePath,'utf8'));
          const found=JSON.stringify(saved).includes('Edited by Codex');
          console.log(JSON.stringify({persistedAgentEdit:found,source:saved}));
        }
      }catch(error){console.log(error.message);}
    }
    input.close();
  } else {
    await tool('update_node',{nodeId:'canvas-headline',data:{content:'Edited by Codex — still editable.'}});
    await tool('canvas_save',{filePath});
    assert.ok((await readFile(filePath,'utf8')).includes('Edited by Codex'));
    console.log('Managed runtime / routing / native tool edit / save smoke: OK');
  }
} finally { child.stdin.end(); await new Promise(resolve=>child.once('close',resolve)); }
const reopened = await EditorSession.start({filePath,allowOrigin:'http://127.0.0.1:1'});
try {
  const value = await reopened.document();
  assert.ok(JSON.stringify(value.document).includes('Edited by Codex'));
  console.log('Saved design reopened in a new Rust session: OK');
} finally { await reopened.close(); }
