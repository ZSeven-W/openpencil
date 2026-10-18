const sec = I(null, {type:"frame", name:"结算条", width:"fill_container", height:"fit_content", layout:"vertical", padding:[10,24,24,24]});

const row = I(sec, {type:"frame", name:"checkout-bar-row", width:"fill_container", height:64, layout:"horizontal", alignItems:"center", justifyContent:"space_between"});

const left = I(row, {type:"frame", name:"total-group", layout:"horizontal", width:"fit_content", height:"fit_content", gap:8, alignItems:"end"});
const totalLabel = I(left, {type:"text", name:"total-label", content:"合计", fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:500, fill:[{type:"solid", color:"#57534E"}]});
const totalVal = I(left, {type:"text", name:"total-value", content:"¥328", fontFamily:"Plus Jakarta Sans", fontSize:24, fontWeight:700, letterSpacing:-0.5, fill:[{type:"solid", color:"#1C1917"}]});
const feeNote = I(left, {type:"text", name:"fee-note", content:"不含运费", fontFamily:"Plus Jakarta Sans", fontSize:11, fontWeight:400, fill:[{type:"solid", color:"#A8A29E"}]});

const cta = I(row, {type:"frame", name:"checkout-btn", width:170, height:48, layout:"horizontal", alignItems:"center", justifyContent:"center", cornerRadius:12, fill:[{type:"solid", color:"#F97316"}], effects:[{type:"shadow", offsetX:0, offsetY:4, blur:14, spread:0, color:"#F9731633"}],
  events:{ onTap:[ { "push": "\"/screen-1\"" } ] },
  animations:[{trigger:"onTap", durationMs:150, easing:"ease_out", keyframes:[{offset:0, values:{scaleX:1, scaleY:1}}, {offset:0.5, values:{scaleX:0.96, scaleY:0.96}}, {offset:1, values:{scaleX:1, scaleY:1}}]}]});

I(cta, {type:"text", name:"checkout-btn-label", content:"去结算 (2)", fontFamily:"Plus Jakarta Sans", fontSize:16, fontWeight:600, fill:[{type:"solid", color:"#FFFFFF"}]});
