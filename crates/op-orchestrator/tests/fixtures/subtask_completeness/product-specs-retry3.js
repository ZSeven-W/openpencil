const sec = I(null, {type:"frame", name:"商品页 · 规格选择", width:"fill_container", height:"fit_content", layout:"vertical", padding:[8,24,8,24], gap:16,
  state:{ "selColor":{type:"string", default:"月白"}, "qty":{type:"int", default:1} }});

const colorRow = I(sec, {type:"frame", name:"颜色分组", layout:"vertical", width:"fill_container", gap:10});
I(colorRow, {type:"text", name:"颜色标题", content:"颜色", fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:600, fill:[{type:"solid", color:"#1C1917"}]});

const pillsRow = I(colorRow, {type:"frame", name:"规格胶囊组", layout:"horizontal", width:"fill_container", gap:10});
const colors = ["月白","燕麦","苔绿"];
for (const c of colors) {
  I(pillsRow, {type:"frame", name:"规格胶囊-" + c, layout:"horizontal", justifyContent:"center", alignItems:"center",
    padding:[8,18], cornerRadius:9999,
    fill:[{type:"solid", color:"#FFFFFF"}],
    stroke:{thickness:1, fill:[{type:"solid", color:"#E7E5E4"}]},
    events:{onTap:[{set:{"$state.selColor":"\"" + c + "\""}}]}});
}

const qtyRow = I(sec, {type:"frame", name:"数量行", layout:"horizontal", width:"fill_container", justifyContent:"space_between", alignItems:"center"});
I(qtyRow, {type:"text", name:"数量标题", content:"数量", fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:600, fill:[{type:"solid", color:"#1C1917"}]});

const stepper = I(qtyRow, {type:"frame", name:"数量步进器", layout:"horizontal", alignItems:"center", gap:0, cornerRadius:9999, fill:[{type:"solid", color:"#FFFFFF"}], stroke:{thickness:1, fill:[{type:"solid", color:"#E7E5E4"}]}, clipContent:true});

const minus = I(stepper, {type:"frame", name:"步进减少", layout:"horizontal", justifyContent:"center", alignItems:"center", width:40, height:36, cornerRadius:9999, fill:[{type:"solid", color:"#FFFFFF"}], events:{onTap:[{if:{expr:"$state.qty > 1", then:[{set:{"$state.qty":"$state.qty - 1"}}]}}]}});
I(minus, {type:"icon_font", iconFontName:"minus", iconFontFamily:"lucide", width:16, height:16, fill:[{type:"solid", color:"#57534E"}]});

const qtyBox = I(stepper, {type:"frame", name:"步进数值", layout:"horizontal", justifyContent:"center", alignItems:"center", width:44, height:36});
I(qtyBox, {type:"text", name:"数量文本", content:"1", fontFamily:"Plus Jakarta Sans", fontSize:15, fontWeight:600, textAlign:"center", fill:[{type:"solid", color:"#1C1917"}], bindings:{content:"\"\" + $state.qty"}});

const plus = I(stepper, {type:"frame", name:"步进增加", layout:"horizontal", justifyContent:"center", alignItems:"center", width:40, height:36, cornerRadius:9999, fill:[{type:"solid", color:"#FFFFFF"}], events:{onTap:[{set:{"$state.qty":"$state.qty + 1"}}]}});
I(plus, {type:"icon_font", iconFontName:"plus", iconFontFamily:"lucide", width:16, height:16, fill:[{type:"solid", color:"#57534E"}]});
