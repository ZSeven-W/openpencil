const sec = I(null, {type:"frame", name:"结算条", width:"fill_container", height:"fit_content", layout:"vertical", padding:[12,24,28,24]});

const bar = I(sec, {type:"frame", name:"checkout-bar-row", width:"fill_container", height:"fit_content", layout:"horizontal", gap:16, alignItems:"center", justifyContent:"space_between"});

const totalCol = I(bar, {type:"frame", name:"total-group", width:"fit_content", height:"fit_content", layout:"vertical", gap:2});

const totalRow = I(totalCol, {type:"frame", name:"total-row", width:"fit_content", height:"fit_content", layout:"horizontal", gap:6, alignItems:"end"});
I(totalRow, {type:"text", name:"total-label", content:"合计", fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:500, fill:[{type:"solid", color:"#1C1917"}], width:"fit_content", textGrowth:"auto"});
I(totalRow, {type:"text", name:"total-amount", content:"¥328", fontFamily:"Plus Jakarta Sans", fontSize:24, fontWeight:700, letterSpacing:-0.5, fill:[{type:"solid", color:"#EA580C"}], width:"fit_content", textGrowth:"auto"});
I(totalCol, {type:"text", name:"total-note", content:"不含运费", fontFamily:"Plus Jakarta Sans", fontSize:11, fontWeight:400, fill:[{type:"solid", color:"#A8A29E"}], width:"fit_content", textGrowth:"auto"});

const cta = I(bar, {type:"frame", name:"checkout-button", width:170, height:52, layout:"horizontal", alignItems:"center", justifyContent:"center", gap:6, cornerRadius:12, fill:[{type:"solid", color:"#F97316"}], effects:[{type:"shadow", offsetX:0, offsetY:4, blur:14, spread:0, color:"#F9731633"}], events:{onTap:[{push:"\"/screen-1\""}]}, animations:[{trigger:"tap", durationMs:120, easing:"ease_out", keyframes:[{offset:0, values:{scaleX:1, scaleY:1}}, {offset:0.5, values:{scaleX:0.96, scaleY:0.96}}, {offset:1, values:{scaleX:1, scaleY:1}}]}]});

I(cta, {type:"text", name:"checkout-button-label", content:"去结算", fontFamily:"Plus Jakarta Sans", fontSize:16, fontWeight:600, fill:[{type:"solid", color:"#FFFFFF"}], width:"fit_content", textGrowth:"auto"});
I(cta, {type:"text", name:"checkout-button-count", content:"(2)", fontFamily:"Plus Jakarta Sans", fontSize:16, fontWeight:600, fill:[{type:"solid", color:"#FFFFFF"}], width:"fit_content", textGrowth:"auto"});
