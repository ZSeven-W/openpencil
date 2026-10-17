const sec = I(null, {type:"frame", name:"购物车页-金额汇总与结算", width:"fill_container", height:"fit_content", layout:"vertical", padding:[12,24,16,24], gap:12});

const rows = I(sec, {type:"frame", name:"金额汇总", width:"fill_container", height:"fit_content", layout:"vertical", gap:8});
const summary = [
  {label:"商品合计", value:"¥218", strong:false},
  {label:"运费", value:"免运费", strong:false, demo:true}
];
for (const r of summary) {
  const row = I(rows, {type:"frame", name:"汇总行-"+r.label, width:"fill_container", height:"fit_content", layout:"horizontal", justifyContent:"space_between", alignItems:"center"});
  I(row, {type:"text", name:r.label+"-label", content:r.label, fontFamily:"SF Pro", fontSize:13, fontWeight:400, fill:[{type:"solid", color:"#6B7280"}], width:"fit_content"});
  const rv = I(row, {type:"frame", name:r.label+"-value-group", width:"fit_content", height:"fit_content", layout:"horizontal", gap:6, alignItems:"center"});
  I(rv, {type:"text", name:r.label+"-value", content:r.value, fontFamily:"SF Pro", fontSize:13, fontWeight:500, fill:[{type:"solid", color:"#111827"}], width:"fit_content"});
  if (r.demo) I(rv, {type:"text", name:r.label+"-demo", content:"示例", fontFamily:"SF Pro", fontSize:10, fontWeight:500, fill:[{type:"solid", color:"#9CA3AF"}], width:"fit_content"});
}

const total = I(sec, {type:"frame", name:"合计行", width:"fill_container", height:"fit_content", layout:"horizontal", justifyContent:"end", alignItems:"end", gap:4, paddingTop:4});
I(total, {type:"text", name:"合计标签", content:"合计", fontFamily:"SF Pro", fontSize:13, fontWeight:400, fill:[{type:"solid", color:"#6B7280"}], width:"fit_content"});
I(total, {type:"text", name:"合计金额", content:"¥218", fontFamily:"SF Pro", fontSize:24, fontWeight:700, fill:[{type:"solid", color:"#2563EB"}], width:"fit_content", letterSpacing:-0.3,
  transition:{property:"content", durationMs:300, easing:"standard"},
  animations:[{trigger:"mount", durationMs:300, easing:"standard", keyframes:[{offset:0, values:{opacity:0, translateY:8}}, {offset:1, values:{opacity:1, translateY:0}}]}]});

const bar = I(sec, {type:"frame", name:"结算操作条", width:"fill_container", height:"fit_content", layout:"horizontal", justifyContent:"space_between", alignItems:"center", gap:12, paddingTop:4});
I(bar, {type:"checkbox", name:"全选复选框", label:"全选", checked:true, width:"fit_content", height:32,
  fill:[{type:"solid", color:"#2563EB"}], stroke:{thickness:1.5, fill:[{type:"solid", color:"#D1D5DB"}]}, cornerRadius:8});
I(bar, {type:"frame", name:"结算按钮", role:"button", width:140, height:48, cornerRadius:12, layout:"horizontal", alignItems:"center", justifyContent:"center", gap:0,
  fill:[{type:"solid", color:"#2563EB"}],
  effects:[{type:"shadow", offsetX:0, offsetY:4, blur:12, spread:0, color:"rgba(37,99,235,0.25)"}],
  events:{onTap:[{toast:"已提交结算 (2) 件商品"}]},
  animations:[{trigger:"mount", durationMs:240, easing:"standard", keyframes:[{offset:0, values:{scaleX:0.96, scaleY:0.96}}, {offset:0.5, values:{scaleX:1.02, scaleY:1.02}}, {offset:1, values:{scaleX:1, scaleY:1}}]}],
  children:[]});
I("结算按钮", {type:"text", name:"结算按钮文字", content:"结算 (2)", fontFamily:"SF Pro", fontSize:16, fontWeight:600, fill:[{type:"solid", color:"#FFFFFF"}], width:"fit_content"});
