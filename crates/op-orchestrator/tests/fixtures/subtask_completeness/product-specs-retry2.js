const sec = I(null, {type:"frame", name:"商品页 · 规格选择", width:"fill_container", height:"fit_content", layout:"vertical", padding:[0,24], gap:16});

const colorGroup = I(sec, {type:"frame", name:"颜色分组", width:"fill_container", height:"fit_content", layout:"vertical", gap:12});

I(colorGroup, {type:"text", name:"颜色标题", content:"颜色", fontFamily:"Plus Jakarta Sans", fontSize:15, fontWeight:600, fill:[{type:"solid", color:"#1C1917"}], width:"fit_content", height:"fit_content"});

const chipRow = I(colorGroup, {type:"frame", name:"规格胶囊组", width:"fill_container", height:"fit_content", layout:"horizontal", gap:12});

const qtyRow = I(sec, {type:"frame", name:"数量行", width:"fill_container", height:"fit_content", layout:"horizontal", justifyContent:"space_between", alignItems:"center"});

I(qtyRow, {type:"text", name:"数量标题", content:"数量", fontFamily:"Plus Jakarta Sans", fontSize:15, fontWeight:600, fill:[{type:"solid", color:"#1C1917"}], width:"fit_content", height:"fit_content"});

const stepper = I(qtyRow, {type:"frame", name:"数量步进器", width:"fit_content", height:36, layout:"horizontal", alignItems:"center", cornerRadius:9999, fill:[{type:"solid", color:"#FFFFFF"}], stroke:{thickness:1, fill:[{type:"solid", color:"#E7E5E4"}]}});

const minusBtn = I(stepper, {type:"frame", name:"减号按钮", width:36, height:36, layout:"horizontal", alignItems:"center", justifyContent:"center", cornerRadius:9999, events:{onTap:[{if:{expr:"$app.qty > 1", then:[{set:{"$app.qty":"$app.qty - 1"}}]}}]}});
I(minusBtn, {type:"icon_font", name:"减号图标", iconFontName:"minus", iconFontFamily:"lucide", width:16, height:16, fill:[{type:"solid", color:"#57534E"}]});

I(stepper, {type:"text", name:"数量值", content:"1", width:40, height:"fit_content", textAlign:"center", fontFamily:"Plus Jakarta Sans", fontSize:15, fontWeight:600, fill:[{type:"solid", color:"#1C1917"}], bindings:{content:"\"\" + $app.qty"}});

const plusBtn = I(stepper, {type:"frame", name:"加号按钮", width:36, height:36, layout:"horizontal", alignItems:"center", justifyContent:"center", cornerRadius:9999, events:{onTap:[{set:{"$app.qty":"$app.qty + 1"}}]}});
I(plusBtn, {type:"icon_font", name:"加号图标", iconFontName:"plus", iconFontFamily:"lucide", width:16, height:16, fill:[{type:"solid", color:"#57534E"}]});
