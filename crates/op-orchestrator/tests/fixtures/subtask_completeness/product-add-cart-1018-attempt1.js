```javascript
const sec = I(null, {type:"frame", name:"底部加购条", width:"fill_container", height:"fit_content", layout:"vertical", padding:[12,24,20,24], fill:[{type:"solid", color:"#FFFFFF"}], stroke:{thickness:{top:1}, fill:[{type:"solid", color:"#F5F5F4"}]}});

const row = I(sec, {type:"frame", name:"加购条内容行", width:"fill_container", height:"fit_content", layout:"horizontal", gap:12, alignItems:"center"});

const sideBtns = I(row, {type:"frame", name:"辅助按钮组", width:"fit_content", height:"fit_content", layout:"horizontal", gap:8, alignItems:"center"});
for (const b of [{icon:"message-circle", label:"客服"}, {icon:"store", label:"店铺"}]) {
  const btn = I(sideBtns, {type:"frame", name:"icon-btn-" + b.label, width:48, height:48, layout:"vertical", gap:2, alignItems:"center", justifyContent:"center", cornerRadius:12, fill:[{type:"solid", color:"#FFF7ED"}]});
  I(btn, {type:"icon_font", iconFontName:b.icon, iconFontFamily:"lucide", width:20, height:20, fill:[{type:"solid", color:"#57534E"}]});
  I(btn, {type:"text", content:b.label, fontFamily:"Plus Jakarta Sans", fontSize:10, fontWeight:500, fill:[{type:"solid", color:"#57534E"}]});
}

const cartBtn = I(row, {type:"frame", name:"加入购物车主按钮", width:"fill_container", height:52, layout:"horizontal", gap:8, alignItems:"center", justifyContent:"center", cornerRadius:12, fill:[{type:"solid", color:"#F97316"}], effects:[{type:"shadow", offsetX:0, offsetY:4, blur:12, spread:0, color:"#F9731633"}],
  events:{onTap:[{push:"\"/screen-1\""}]},
  animations:[{trigger:"mount", durationMs:200, easing:"standard", keyframes:[{offset:0, values:{scaleX:0.96, scaleY:0.96}}, {offset:1, values:{scaleX:1, scaleY:1}}]}]});

I(cartBtn, {type:"icon_font", iconFontName:"shopping-cart", iconFontFamily:"lucide", width:20, height:20, fill:[{type:"solid", color:"#FFFFFF"}]});
I(cartBtn, {type:"text", content:"加入购物车", fontFamily:"Plus Jakarta Sans", fontSize:16, fontWeight:600, fill:[{type:"solid", color:"#FFFFFF"}]});

const specCol = I(cartBtn, {type:"frame", name:"按钮内规格摘要列", width:"fit_content", height:"fit_content", layout:"vertical", gap:1, alignItems:"start"});
I(specCol, {type:"text", content:"¥129", fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:700, fill:[{type:"solid", color:"#FFFFFF"}]});
I(specCol, {type:"text", content:"白色 · M", fontFamily:"Plus Jakarta Sans", fontSize:10, fontWeight:400, fill:[{type:"solid", color:"#FFEDD5"}]});
```