const sec = I(null, {type:"frame", name:"商品页 · 规格选择", width:"fill_container", height:"fit_content", layout:"vertical", padding:[0,24], gap:16,
  fill:[], state:{ color:{type:"string", default:"月白"}, qty:{type:"int", default:1} }});

const colorRow = I(sec, {type:"frame", name:"颜色分组", layout:"horizontal", width:"fill_container", height:"fit_content", gap:12, alignItems:"center"});
I(colorRow, {type:"text", name:"颜色标签", content:"颜色", fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:600, fill:[{type:"solid", color:"#1C1917"}], width:"fit_content", textGrowth:"auto"});
const pillsRow = I(colorRow, {type:"frame", name:"规格胶囊组", layout:"horizontal", width:"fill_container", height:"fit_content", gap:12});
for (const c of ["月白","燕麦","苔绿"]) {
  const pill = I(pillsRow, {type:"frame", name:"规格胶囊-" + c, layout:"horizontal", width:"fit_content", height:38, cornerRadius:999, padding:[0,18],
    alignItems:"center", justifyContent:"center",
    fill:[{type:"solid", color:"#1C1917"}],
    stroke:{thickness:1, fill:[{type:"solid", color:"#1C1917"}]},
    bindings:{
      fill: "$state.color === '" + c + "' ? '#1C1917' : '#FFFFFF'",
      stroke: "$state.color === '" + c + "' ? '#1C1917' : '#E7E5E4'"
    },
    events:{onTap:[{set:{"$state.color": "'" + c + "'"}}]}});
  I(pill, {type:"text", name:"胶囊文字-" + c, content:c, fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:500, width:"fit_content", textGrowth:"auto",
    fill:[{type:"solid", color:"#FFFFFF"}],
    bindings:{ fill: "$state.color === '" + c + "' ? '#FFFFFF' : '#57534E'" }});
}

const qtyRow = I(sec, {type:"frame", name:"数量分组", layout:"horizontal", width:"fill_container", height:"fit_content", alignItems:"center", justifyContent:"space_between"});
I(qtyRow, {type:"text", name:"数量标签", content:"数量", fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:600, fill:[{type:"solid", color:"#1C1917"}], width:"fit_content", textGrowth:"auto"});
const stepper = I(qtyRow, {type:"frame", name:"数量步进器", layout:"horizontal", width:"fit_content", height:36, cornerRadius:999,
  fill:[{type:"solid", color:"#FFFFFF"}], stroke:{thickness:1, fill:[{type:"solid", color:"#E7E5E4"}]}, alignItems:"center"});
const minusBtn = I(stepper, {type:"frame", name:"减少按钮", layout:"horizontal", width:44, height:36, alignItems:"center", justifyContent:"center",
  events:{onTap:[{if:{expr:"$state.qty > 1", then:[{set:{"$state.qty":"$state.qty - 1"}}]}}]}});
I(minusBtn, {type:"icon_font", name:"减号图标", iconFontName:"minus", iconFontFamily:"lucide", width:16, height:16, fill:[{type:"solid", color:"#57534E"}]});
const qtyText = I(stepper, {type:"text", name:"数量值", content:"1", fontFamily:"Plus Jakarta Sans", fontSize:15, fontWeight:600, width:36, textAlign:"center", textGrowth:"fixed-width", height:20,
  fill:[{type:"solid", color:"#1C1917"}],
  bindings:{content:"String($state.qty)"}});
const plusBtn = I(stepper, {type:"frame", name:"增加按钮", layout:"horizontal", width:44, height:36, alignItems:"center", justifyContent:"center",
  events:{onTap:[{set:{"$state.qty":"$state.qty + 1"}}]}});
I(plusBtn, {type:"icon_font", name:"加号图标", iconFontName:"plus", iconFontFamily:"lucide", width:16, height:16, fill:[{type:"solid", color:"#F97316"}]});
