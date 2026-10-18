const sec = I(null, {type:"frame", name:"规格选择", width:"fill_container", height:"fit_content", layout:"vertical", padding:[0,24], gap:16});

const specState = {
  color: { type:"string", default:"云白" },
  size:  { type:"string", default:"M" },
  qty:   { type:"int", default:1 }
};
I(sec, {type:"frame", name:"spec-state-holder", width:0, height:0, opacity:0, state:specState});

const title = I(sec, {type:"text", name:"spec-section-title", content:"规格", fontFamily:"SF Pro", fontSize:17, fontWeight:600, fill:[{type:"solid", color:"#111827"}], width:"fit_content", height:22, lineHeight:1.2, letterSpacing:-0.3});

function chipRow(labelText, options, stateKey) {
  const row = I(sec, {type:"frame", name:"spec-row-"+stateKey, layout:"horizontal", width:"fill_container", height:"fit_content", gap:12, alignItems:"center"});
  I(row, {type:"text", name:"spec-row-label-"+stateKey, content:labelText, fontFamily:"SF Pro", fontSize:13, fontWeight:500, fill:[{type:"solid", color:"#6B7280"}], width:40, height:18, lineHeight:1.4});
  const chips = I(row, {type:"frame", name:"spec-chips-"+stateKey, layout:"horizontal", width:"fill_container", height:"fit_content", gap:10, alignItems:"center"});
  for (const o of options) {
    I(chips, {type:"frame", name:"spec-chip-"+stateKey+"-"+o, layout:"horizontal", width:"fit_content", height:36, padding:[0,16], cornerRadius:8, alignItems:"center", justifyContent:"center",
      fill:[{type:"solid", color:"#FFFFFF"}],
      stroke:{thickness:1, fill:[{type:"solid", color:"$state."+stateKey+" === '"+o+"' ? '#2563EB' : '#E5E7EB'"}]},
      bindings:{ fill:"$state."+stateKey+" === '"+o+"' ? '#2563EB15' : '#FFFFFF'" },
      events:{ onTap:[ { set: { ["$state."+stateKey]: "'"+o+"'" } } ] },
      children:[
        I(chips, {type:"text", name:"spec-chip-text-"+stateKey+"-"+o, content:o, fontFamily:"SF Pro", fontSize:13, fontWeight:500,
          fill:[{type:"solid", color:"#6B7280"}],
          width:"fit_content", height:16, lineHeight:1.2,
          bindings:{ fill:"$state."+stateKey+" === '"+o+"' ? '#2563EB' : '#374151'" }})
      ]});
  }
  return row;
}

chipRow("颜色", ["云白","雾灰"], "color");
chipRow("尺码", ["S","M","L","XL"], "size");

const qtyRow = I(sec, {type:"frame", name:"spec-row-qty", layout:"horizontal", width:"fill_container", height:"fit_content", gap:12, alignItems:"center"});
I(qtyRow, {type:"text", name:"spec-row-label-qty", content:"数量", fontFamily:"SF Pro", fontSize:13, fontWeight:500, fill:[{type:"solid", color:"#6B7280"}], width:40, height:18, lineHeight:1.4});
const stepper = I(qtyRow, {type:"frame", name:"qty-stepper", layout:"horizontal", width:"fit_content", height:36, cornerRadius:8, alignItems:"center", stroke:{thickness:1, fill:[{type:"solid", color:"#E5E7EB"}]}, fill:[{type:"solid", color:"#FFFFFF"}]});
I(stepper, {type:"frame", name:"qty-decrease", layout:"horizontal", width:36, height:36, alignItems:"center", justifyContent:"center", events:{ onTap:[ { if: { expr:"$state.qty > 1", then:[ { set:{ "$state.qty":"$state.qty - 1" } } ] } } ] },
  children:[ I(stepper, {type:"icon_font", name:"qty-minus-icon", iconFontName:"minus", iconFontFamily:"lucide", width:16, height:16, fill:[{type:"solid", color:"#6B7280"}]}) ]});
I(stepper, {type:"text", name:"qty-value", content:"1", fontFamily:"SF Pro", fontSize:15, fontWeight:500, fill:[{type:"solid", color:"#111827"}], width:32, height:20, textAlign:"center", lineHeight:1.3, bindings:{ content:to_str($state.qty) }});
I(stepper, {type:"frame", name:"qty-increase", layout:"horizontal", width:36, height:36, alignItems:"center", justifyContent:"center", events:{ onTap:[ { set:{ "$state.qty":"$state.qty + 1" } } ] },
  children:[ I(stepper, {type:"icon_font", name:"qty-plus-icon", iconFontName:"plus", iconFontFamily:"lucide", width:16, height:16, fill:[{type:"solid", color:"#2563EB"}]}) ]});