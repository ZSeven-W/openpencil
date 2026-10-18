const sec = I(null, {type:"frame", name:"商品页 · 规格选择", width:"fill_container", height:"fit_content", layout:"vertical", padding:[0,24], gap:16});

const colorBlock = I(sec, {type:"frame", name:"颜色分组", layout:"vertical", width:"fill_container", gap:12});
I(colorBlock, {type:"text", name:"颜色标题", content:"颜色", fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:600, fill:[{type:"solid", color:"#1C1917"}]});

const chipRow = I(colorBlock, {type:"frame", name:"规格胶囊组", layout:"horizontal", width:"fill_container", gap:12});
const colors = [
  {name:"月白", selected:true},
  {name:"燕麦", selected:false},
  {name:"苔绿", selected:false}
];
let chipIds = [];
for (const c of colors) {
  const chip = I(chipRow, {type:"frame", name:"规格胶囊-" + c.name, layout:"horizontal", width:"fit_content", height:"fit_content", padding:[8,18], cornerRadius:9999, alignItems:"center", justifyContent:"center",
    fill: c.selected ? [{type:"solid", color:"#1C1917"}] : [{type:"solid", color:"#FFFFFF"}],
    stroke: c.selected ? {thickness:1, fill:[{type:"solid", color:"#1C1917"}]} : {thickness:1, fill:[{type:"solid", color:"#E7E5E4"}]},
    transition: "all 200ms ease",
    events: { onTap: [ { "set": { "$app.color": "'" + c.name + "'" } } ] }
  });
  I(chip, {type:"text", name:"胶囊文字-" + c.name, content:c.name, fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:500,
    fill: c.selected ? [{type:"solid", color:"#FFFFFF"}] : [{type:"solid", color:"#57534E"}],
    bindings: c.selected ? null : { fill: "$app.color === '" + c.name + "' ? '#FFFFFF' : '#57534E'" }
  });
  chipIds.push(chip);
}

const qtyBlock = I(sec, {type:"frame", name:"数量分组", layout:"vertical", width:"fill_container", gap:12});
I(qtyBlock, {type:"text", name:"数量标题", content:"数量", fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:600, fill:[{type:"solid", color:"#1C1917"}]});

const stepper = I(qtyBlock, {type:"frame", name:"数量步进器", layout:"horizontal", width:"fit_content", height:"fit_content", alignItems:"center", gap:0, cornerRadius:8, stroke:{thickness:1, fill:[{type:"solid", color:"#E7E5E4"}]}, fill:[{type:"solid", color:"#FFFFFF"}], clipContent:true});

const minusBtn = I(stepper, {type:"frame", name:"步进减", layout:"horizontal", width:36, height:36, alignItems:"center", justifyContent:"center",
  events: { onTap: [ { "if": { expr: "$app.qty > 1", then: [ { "set": { "$app.qty": "$app.qty - 1" } } ] } } ] }
});
I(minusBtn, {type:"icon_font", iconFontName:"minus", iconFontFamily:"lucide", width:16, height:16, fill:[{type:"solid", color:"#57534E"}]});

const qtyWrap = I(stepper, {type:"frame", name:"步进数值", layout:"horizontal", width:48, height:36, alignItems:"center", justifyContent:"center", stroke:{thickness:{left:1, right:1}, fill:[{type:"solid", color:"#E7E5E4"}]}});
I(qtyWrap, {type:"text", name:"数量值", content:"1", fontFamily:"Plus Jakarta Sans", fontSize:15, fontWeight:600, fill:[{type:"solid", color:"#1C1917"}], bindings:{content:"String($app.qty)"}});

const plusBtn = I(stepper, {type:"frame", name:"步进加", layout:"horizontal", width:36, height:36, alignItems:"center", justifyContent:"center",
  events: { onTap: [ { "set": { "$app.qty": "$app.qty + 1" } } ] }
});
I(plusBtn, {type:"icon_font", iconFontName:"plus", iconFontFamily:"lucide", width:16, height:16, fill:[{type:"solid", color:"#57534E"}]});
