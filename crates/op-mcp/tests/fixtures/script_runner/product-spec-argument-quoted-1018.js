const sec = I(null, {type:"frame", name:"规格选择", width:"fill_container", height:"fit_content", layout:"vertical", padding:[8,24,16,24], gap:14, fill:[{type:"solid",color:"#FFFFFF"}],
  state:{
    selColor:{type:"string", default:"白"},
    selSize:{type:"string", default:"M"},
    qty:{type:"int", default:1}
  }});

const headRow = I(sec, {type:"frame", name:"spec-header", layout:"horizontal", width:"fill_container", justifyContent:"space_between", alignItems:"center"});
I(headRow, {type:"text", name:"spec-title", content:"选择规格", fontFamily:"Plus Jakarta Sans", fontSize:16, fontWeight:600, fill:[{type:"solid",color:"#1C1917"}]});
const allSpec = I(headRow, {type:"frame", name:"expand-all", layout:"horizontal", gap:2, alignItems:"center",
  events:{onTap:[{toast:"'展开全部规格'"}]}});
I(allSpec, {type:"text", content:"展开全部规格", fontFamily:"Plus Jakarta Sans", fontSize:13, fontWeight:400, fill:[{type:"solid",color:"#A8A29E"}]});
I(allSpec, {type:"icon_font", iconFontName:"chevron-down", iconFontFamily:"lucide", width:16, height:16, fill:"#A8A29E"});

// color row
const colorRow = I(sec, {type:"frame", name:"color-row", layout:"horizontal", width:"fill_container", gap:8, alignItems:"center"});
I(colorRow, {type:"text", name:"color-label", content:"颜色", fontFamily:"Plus Jakarta Sans", fontSize:13, fontWeight:500, fill:[{type:"solid",color:"#57534E"}], width:34});
const colors = ["白","黑","雾蓝"];
const colorSwatches = {白:"#FFFFFF", 黑:"#1C1917", 雾蓝:"#94A3B8"};
for (const c of colors) {
  const sel = "$app.selColor == '"+c+"'";
  const chip = I(colorRow, {type:"frame", name:"color-pill-"+c, layout:"horizontal", height:32, padding:[0,14], gap:6, alignItems:"center", cornerRadius:9999,
    fill:[{type:"solid",color:"#FFFFFF"}],
    stroke:{thickness:1, fill:[{type:"solid",color:"#E7E5E4"}]},
    bindings:{stroke:{thickness:1.5, fill:"$$app.selColor == '"+c+"' ? '#F97316' : '#E7E5E4'"}, fill:"$app.selColor == '"+c+"' ? '#FFF7ED' : '#FFFFFF'"},
    events:{onTap:[{set:{"$app.selColor":"'"+c+"'"}}]}});
  I(chip, {type:"ellipse", name:"color-dot-"+c, width:12, height:12, fill:[{type:"solid",color:colorSwatches[c]}], stroke:{thickness:1, fill:[{type:"solid",color:"#D6D3D1"}]}});
  I(chip, {type:"text", name:"color-text-"+c, content:c, fontFamily:"Plus Jakarta Sans", fontSize:13, fontWeight:500,
    fill:[{type:"solid",color:"#57534E"}],
    bindings:{fill:"$app.selColor == '"+c+"' ? '#EA580C' : '#57534E'"}});
}

// size row
const sizeRow = I(sec, {type:"frame", name:"size-row", layout:"horizontal", width:"fill_container", gap:8, alignItems:"center"});
I(sizeRow, {type:"text", name:"size-label", content:"尺码", fontFamily:"Plus Jakarta Sans", fontSize:13, fontWeight:500, fill:[{type:"solid",color:"#57534E"}], width:34});
for (const s of ["S","M","L","XL"]) {
  const chip = I(sizeRow, {type:"frame", name:"size-pill-"+s, layout:"horizontal", height:32, padding:[0,16], alignItems:"center", justifyContent:"center", cornerRadius:9999,
    fill:[{type:"solid",color:"#FFFFFF"}],
    bindings:{fill:"$app.selSize == '"+s+"' ? '#F97316' : '#FFFFFF'"},
    stroke:{thickness:1, fill:[{type:"solid",color:"#E7E5E4"}]},
    bindings:{stroke:{thickness:1.5, fill:"$app.selSize == '"+s+"' ? '#F97316' : '#E7E5E4'"}},
    events:{onTap:[{set:{"$app.selSize":"'"+s+"'"}}]}});
  I(chip, {type:"text", name:"size-text-"+s, content:s, fontFamily:"Plus Jakarta Sans", fontSize:13, fontWeight:500,
    fill:[{type:"solid",color:"#57534E"}],
    bindings:{fill:"$app.selSize == '"+s+"' ? '#FFFFFF' : '#57534E'"},
    animations:[{trigger:"mount", durationMs:300, easing:"standard", keyframes:[{offset:0,values:{opacity:0, translateY:4}},{offset:1,values:{opacity:1, translateY:0}}]}]});
}

// qty row
const qtyRow = I(sec, {type:"frame", name:"qty-row", layout:"horizontal", width:"fill_container", justifyContent:"space_between", alignItems:"center"});
I(qtyRow, {type:"text", name:"qty-label", content:"数量", fontFamily:"Plus Jakarta Sans", fontSize:13, fontWeight:500, fill:[{type:"solid",color:"#57534E"}]});
const stepper = I(qtyRow, {type:"frame", name:"qty-stepper", layout:"horizontal", gap:0, alignItems:"center", cornerRadius:9999,
  fill:[{type:"solid",color:"#FFFFFF"}], stroke:{thickness:1, fill:[{type:"solid",color:"#E7E5E4"}]}});
const minusBtn = I(stepper, {type:"frame", name:"qty-minus", layout:"horizontal", width:36, height:32, alignItems:"center", justifyContent:"center",
  events:{onTap:[{if:{expr:"$app.qty > 1", then:[{set:{"$app.qty":"$app.qty - 1"}}]}}]}});
I(minusBtn, {type:"icon_font", iconFontName:"minus", iconFontFamily:"lucide", width:16, height:16, fill:"#57534E"});
I(stepper, {type:"text", name:"qty-value", content:"1", fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:600, fill:[{type:"solid",color:"#1C1917"}], width:36, textAlign:"center",
  bindings:{content:to_str("$app.qty")}});
const plusBtn = I(stepper, {type:"frame", name:"qty-plus", layout:"horizontal", width:36, height:32, alignItems:"center", justifyContent:"center",
  events:{onTap:[{set:{"$app.qty":"$app.qty + 1"}}]}});
I(plusBtn, {type:"icon_font", iconFontName:"plus", iconFontFamily:"lucide", width:16, height:16, fill:"#F97316"});
