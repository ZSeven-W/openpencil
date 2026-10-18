const sec = I(null, {type:"frame", name:"规格选择", width:"fill_container", height:"fit_content", layout:"vertical", padding:[0,24], gap:14});

const state = I(sec, {type:"frame", name:"spec-state", width:0, height:0, state:{
  color:{type:"string", default:"白"},
  size:{type:"string", default:"M"},
  qty:{type:"int", default:1}
}});

const colorRow = I(sec, {type:"frame", name:"color-row", layout:"horizontal", width:"fill_container", height:"fit_content", gap:10, alignItems:"center"});
I(colorRow, {type:"text", name:"color-label", content:"颜色", fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:600, fill:[{type:"solid", color:"#1C1917"}], width:"fit_content", textGrowth:"auto"});
const colorChips = I(colorRow, {type:"frame", name:"color-chips", layout:"horizontal", width:"fill_container", height:"fit_content", gap:10});
for (const c of [{v:"白"},{v:"黑"},{v:"雾蓝"}]) {
  I(colorChips, {type:"frame", name:"chip-color-"+c.v, layout:"horizontal", alignItems:"center", justifyContent:"center", height:34, cornerRadius:17, padding:[0,16],
    fill:[{type:"solid", color:"#FFFFFF"}], stroke:{thickness:1, fill:[{type:"solid", color:"#E7E5E4"}]},
    bindings:{fill:"$app.color == '"+c.v+"' ? '#FFF7ED' : '#FFFFFF'", stroke:"$app.color == '"+c.v+"' ? '#F97316' : '#E7E5E4'"},
    events:{onTap:[{set:{"$app.color":"'"+c.v+"'"}}]},
    children:[{type:"text", name:"chip-color-text-"+c.v, content:c.v, fontFamily:"Plus Jakarta Sans", fontSize:13, fontWeight:500,
      fill:[{type:"solid", color:"#57534E"}],
      bindings:{fill:"$app.color == '"+c.v+"' ? '#EA580C' : '#57534E'"}}]});
}

const sizeRow = I(sec, {type:"frame", name:"size-row", layout:"horizontal", width:"fill_container", height:"fit_content", gap:10, alignItems:"center"});
I(sizeRow, {type:"text", name:"size-label", content:"尺码", fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:600, fill:[{type:"solid", color:"#1C1917"}], width:"fit_content", textGrowth:"auto"});
const sizeChips = I(sizeRow, {type:"frame", name:"size-chips", layout:"horizontal", width:"fill_container", height:"fit_content", gap:10});
for (const s of [{v:"S"},{v:"M"},{v:"L"},{v:"XL"}]) {
  I(sizeChips, {type:"frame", name:"chip-size-"+s.v, layout:"horizontal", alignItems:"center", justifyContent:"center", height:34, cornerRadius:17, padding:[0,16],
    fill:[{type:"solid", color:"#FFFFFF"}], stroke:{thickness:1, fill:[{type:"solid", color:"#E7E5E4"}]},
    bindings:{fill:"$app.size == '"+s.v+"' ? '#FFF7ED' : '#FFFFFF'", stroke:"$app.size == '"+s.v+"' ? '#F97316' : '#E7E5E4'"},
    events:{onTap:[{set:{"$app.size":"'"+s.v+"'"}}]},
    children:[{type:"text", name:"chip-size-text-"+s.v, content:s.v, fontFamily:"Plus Jakarta Sans", fontSize:13, fontWeight:500,
      fill:[{type:"solid", color:"#57534E"}],
      bindings:{fill:"$app.size == '"+s.v+"' ? '#EA580C' : '#57534E'"}}]});
}

const expandRow = I(sec, {type:"frame", name:"expand-row", layout:"horizontal", width:"fill_container", height:"fit_content", alignItems:"center", justifyContent:"space_between"});
I(expandRow, {type:"text", name:"expand-label", content:"展开全部规格", fontFamily:"Plus Jakarta Sans", fontSize:13, fontWeight:400, fill:[{type:"solid", color:"#A8A29E"}], width:"fit_content", textGrowth:"auto"});
I(expandRow, {type:"icon_font", name:"expand-chevron", iconFontName:"chevron-down", iconFontFamily:"lucide", width:16, height:16, fill:[{type:"solid", color:"#A8A29E"}]});

const qtyRow = I(sec, {type:"frame", name:"qty-row", layout:"horizontal", width:"fill_container", height:"fit_content", alignItems:"center", justifyContent:"space_between"});
I(qtyRow, {type:"text", name:"qty-label", content:"购买数量", fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:600, fill:[{type:"solid", color:"#1C1917"}], width:"fit_content", textGrowth:"auto"});
const stepper = I(qtyRow, {type:"frame", name:"qty-stepper", layout:"horizontal", height:34, cornerRadius:17, alignItems:"center", fill:[{type:"solid", color:"#F5F5F4"}], stroke:{thickness:1, fill:[{type:"solid", color:"#E7E5E4"}]}, clipContent:true});
const minusBtn = I(stepper, {type:"frame", name:"qty-minus", layout:"horizontal", width:36, height:34, alignItems:"center", justifyContent:"center",
  events:{onTap:[{if:{expr:"$app.qty > 1", then:[{set:{"$app.qty":"$app.qty - 1"}}]}}]},
  children:[{type:"icon_font", name:"qty-minus-icon", iconFontName:"minus", iconFontFamily:"lucide", width:16, height:16, fill:[{type:"solid", color:"#57534E"}]}]});
I(stepper, {type:"text", name:"qty-value", content:"1", width:36, textAlign:"center", fontFamily:"Plus Jakarta Sans", fontSize:15, fontWeight:600, fill:[{type:"solid", color:"#1C1917"}], textGrowth:"auto",
  bindings:{content:to_str($app.qty)}, animations:[{trigger:"mount", durationMs:200, easing:"standard", keyframes:[{offset:0, values:{opacity:0, translateY:6}}, {offset:1, values:{opacity:1, translateY:0}}]}]});
const plusBtn = I(stepper, {type:"frame", name:"qty-plus", layout:"horizontal", width:36, height:34, alignItems:"center", justifyContent:"center",
  events:{onTap:[{set:{"$app.qty":"$app.qty + 1"}}]},
  children:[{type:"icon_font", name:"qty-plus-icon", iconFontName:"plus", iconFontFamily:"lucide", width:16, height:16, fill:[{type:"solid", color:"#F97316"}]}]});
