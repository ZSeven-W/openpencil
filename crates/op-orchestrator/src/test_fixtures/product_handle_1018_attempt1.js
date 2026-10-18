const sec = I(null, {type:"frame", name:"商品详情页", screen:"/", width:"fill_container", height:"fit_content", layout:"vertical", fill:[]});

// state: selected specs drive price
const rootState = null; // state goes on sec via re-insert below

// --- Carousel viewport (full-bleed, nav overlay) ---
const hero = I(sec, {type:"frame", name:"轮播视口", width:"fill_container", height:340, layout:"none", clipContent:true, fill:[{type:"solid", color:"#FFF7ED"}]});

// nav row (topmost)
const nav = I(hero, {type:"frame", name:"导航栏", x:0, y:0, width:375, height:56, layout:"horizontal", padding:[0,16], alignItems:"center", justifyContent:"space_between"});
const backBtn = I(nav, {type:"frame", name:"返回按钮", width:40, height:40, layout:"horizontal", alignItems:"center", justifyContent:"center", cornerRadius:20, fill:[{type:"solid", color:"#FFFFFFCC"}], events:{onTap:[{toast:"'返回商品列表'"}]}});
I(backBtn, {type:"icon_font", name:"返回图标", iconFontName:"chevron-left", iconFontFamily:"lucide", width:22, height:22, fill:[{type:"solid", color:"#1C1917"}]});
I(nav, {type:"text", name:"导航标题", content:"商品详情", fontFamily:"Plus Jakarta Sans", fontSize:16, fontWeight:600, width:"fit_content", height:"fit_content", fill:[{type:"solid", color:"#1C1917"}]});
const shareBtn = I(nav, {type:"frame", name:"分享按钮", width:40, height:40, layout:"horizontal", alignItems:"center", justifyContent:"center", cornerRadius:20, fill:[{type:"solid", color:"#FFFFFFCC"}]});
I(shareBtn, {type:"icon_font", name:"分享图标", iconFontName:"share-2", iconFontFamily:"lucide", width:20, height:20, fill:[{type:"solid", color:"#1C1917"}]});

// indicator dots (1/3)
const dots = I(hero, {type:"frame", name:"页码指示", x:167, y:308, width:40, height:6, layout:"horizontal", gap:6, alignItems:"center"});
I(dots, {type:"rectangle", name:"dot-active", width:16, height:6, cornerRadius:3, fill:[{type:"solid", color:"#F97316"}]});
I(dots, {type:"rectangle", name:"dot-2", width:6, height:6, cornerRadius:3, fill:[{type:"solid", color:"#FFFFFF"}]});
I(dots, {type:"rectangle", name:"dot-3", width:6, height:6, cornerRadius:3, fill:[{type:"solid", color:"#FFFFFF"}]});

// carousel images (bottom layer, side by side, swipeable)
const strip = I(hero, {type:"frame", name:"轮播图带", x:0, y:0, width:1125, height:340, layout:"horizontal", gap:0, clipContent:true});
const slides = [
  {q:"insulated bottle white", p:"一款奶白色简约不锈钢保温杯，450ml，置于柔和的米色背景上，产品居中特写，明亮自然光，电商产品摄影"},
  {q:"insulated bottle lifestyle", p:"奶白色保温随行杯放在木质桌面与绿植旁，温暖的生活场景照，浅景深，柔和晨光"},
  {q:"insulated bottle detail", p:"保温杯瓶口细节特写，展示哑光质感与金属光泽，浅色背景，柔和布光，产品细节摄影"}
];
for (const s of slides) {
  I(strip, {type:"image", name:"轮播图", width:375, height:340, imageSearchQuery:s.q, imagePrompt:s.p});
}

// --- Info rail (single 24px rail below media) ---
const rail = I(sec, {type:"frame", name:"内容区", width:"fill_container", height:"fit_content", layout:"vertical", padding:[0,24], gap:0});

// price row with roll-in animation
const priceRow = I(rail, {type:"frame", name:"价格区", width:"fill_container", height:"fit_content", layout:"horizontal", padding:[16,0,4,0], gap:2, alignItems:"end"});
const priceText = I(priceRow, {type:"text", name:"价格", content:"¥129", fontFamily:"Plus Jakarta Sans", fontSize:40, fontWeight:700, letterSpacing:-1.5, width:"fit_content", height:"fit_content", lineHeight:1.05, fill:[{type:"solid", color:"#EA580C"}], bindings:{content:"'¥' + (129 + ($app.capacity == '600ml' ? 20 : 0))"}});

// product name
I(rail, {type:"text", name:"商品名", content:"轻氧保温随行杯 450ml", fontFamily:"Plus Jakarta Sans", fontSize:22, fontWeight:600, letterSpacing:-0.5, lineHeight:1.3, width:"fill_container", textGrowth:"fixed-width", fill:[{type:"solid", color:"#1C1917"}]});

// spec summary line
I(rail, {type:"text", name:"已选规格", content:"已选：奶白 · 450ml", fontFamily:"Plus Jakarta Sans", fontSize:13, fontWeight:400, width:"fill_container", textGrowth:"fixed-width", fill:[{type:"solid", color:"#A8A29E"}], bindings:{content:"'已选：' + $app.color + ' · ' + $app.capacity"}});

// --- Specs ---
const specs = I(rail, {type:"frame", name:"规格选择区", width:"fill_container", height:"fit_content", layout:"vertical", padding:[20,0,0,0], gap:16});

function chipGroup(label, options, stateKey) {
  const g = I(specs, {type:"frame", name:label + "组", width:"fill_container", height:"fit_content", layout:"horizontal", gap:12, alignItems:"center"});
  I(g, {type:"text", name:label+"标签", content:label, fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:500, width:48, height:"fit_content", fill:[{type:"solid", color:"#57534E"}]});
  for (const o of options) {
    I(g, {type:"frame", name:"胶囊-" + o, width:"fit_content", height:36, layout:"horizontal", padding:[0,16], alignItems:"center", cornerRadius:18,
      stroke:{thickness:1, fill:[{type:"solid", color:"#E7E5E4"}]},
      fill:[{type:"solid", color:"#FFFFFF"}],
      bindings:{fill:"$app." + stateKey + " == '" + o + "' ? '#F97316' : '#FFFFFF'",
                stroke:"$app." + stateKey + " == '" + o + "' ? '#F97316' : '#E7E5E4'"},
      events:{onTap:[{set:{["$app." + stateKey]: "'" + o + "'"}}, {animate:{target:"$node", property:"scaleX", from:0.95, to:1, durationMs:200, easing:"ease_out", fillMode:"both"}}, {animate:{target:"$node", property:"scaleY", from:0.95, to:1, durationMs:200, easing:"ease_out", fillMode:"both"}}]},
      children:[]});
    const chip = g.children ? null : null;
  }
}
// build chips with labels (children inside frames via nested insert)
function chip(label, option, stateKey) {
  const c = I(specs.lastRow, {type:"frame", name:"胶囊-" + option, width:"fit_content", height:36, layout:"horizontal", padding:[0,16], alignItems:"center", justifyContent:"center", cornerRadius:18,
    stroke:{thickness:1, fill:[{type:"solid", color:"#E7E5E4"}]}, fill:[{type:"solid", color:"#FFFFFF"}],
    bindings:{fill:"$app." + stateKey + " == '" + option + "' ? '#F9731620' : '#FFFFFF'",
              stroke:"$app." + stateKey + " == '" + option + "' ? '#F97316' : '#E7E5E4'"},
    events:{onTap:[{set:{["$app." + stateKey]: "'" + option + "'"}}]}});
  I(c, {type:"text", name:"胶囊文字", content:option, fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:500, width:"fit_content", height:"fit_content",
    fill:[{type:"solid", color:"#1C1917"}],
    bindings:{fill:"$app." + stateKey + " == '" + option + "' ? '#EA580C' : '#1C1917'"}});
}
function group(label, options, stateKey) {
  specs.lastRow = I(specs, {type:"frame", name:label + "行", width:"fill_container", height:"fit_content", layout:"horizontal", gap:12, alignItems:"center"});
  I(specs.lastRow, {type:"text", name:label+"标签", content:label, fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:500, width:48, height:"fit_content", fill:[{type:"solid", color:"#57534E"}]});
  for (const o of options) chip(label, o, stateKey);
}
group("颜色", ["奶白", "墨绿"], "color");
group("容量", ["450ml", "600ml"], "capacity");

// --- Reviews ---
const revSection = I(rail, {type:"frame", name:"评价区", width:"fill_container", height:"fit_content", layout:"vertical", padding:[28,0,0,0], gap:16});
I(revSection, {type:"text", name:"评价标题", content:"用户评价 (3)", fontFamily:"Plus Jakarta Sans", fontSize:18, fontWeight:600, width:"fill_container", textGrowth:"fixed-width", fill:[{type:"solid", color:"#1C1917"}]});

const reviews = [
  {name:"山鹿", stars:5, text:"保温效果很好，早上装的咖啡到下午还是温热的，奶白色也很有质感。"},
  {name:"Momo", stars:5, text:"大小刚好放进包里，杯口不漏，通勤路上用非常方便，值得入手。"},
  {name:"阿澈", stars:4, text:"手感不错，颜色和图片一致，就是希望出个更大容量版本。"}
];
for (const r of reviews) {
  const row = I(revSection, {type:"frame", name:"评价行-" + r.name, width:"fill_container", height:"fit_content", layout:"horizontal", padding:[16,0], gap:12, alignItems:"start", stroke:{thickness:{bottom:1}, fill:[{type:"solid", color:"#F5F5F4"}]}});
  const av = I(row, {type:"frame", name:"头像", width:40, height:40, cornerRadius:20, layout:"horizontal", alignItems:"center", justifyContent:"center", fill:[{type:"solid", color:"#FFF7ED"}]});
  I(av, {type:"icon_font", name:"头像图标", iconFontName:"user", iconFontFamily:"lucide", width:20, height:20, fill:[{type:"solid", color:"#EA580C"}]});
  const col = I(row, {type:"frame", name:"评价内容", width:"fill_container", height:"fit_content", layout:"vertical", gap:6});
  const top = I(col, {type:"frame", name:"评价头部", width:"fill_container", height:"fit_content", layout:"horizontal", justifyContent:"space_between", alignItems:"center"});
  I(top, {type:"text", name:"昵称", content:r.name, fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:600, width:"fit_content", height:"fit_content", fill:[{type:"solid", color:"#1C1917"}]});
  const stars = I(top, {type:"frame", name:"星级", width:"fit_content", height:"fit_content", layout:"horizontal", gap:2, alignItems:"center"});
  for (let i = 0; i < 5; i++) I(stars, {type:"icon_font", name:"星", iconFontName:"star", iconFontFamily:"lucide", width:14, height:14, fill:[{type:"solid", color: i < r.stars ? "#F97316" : "#E7E5E4"}]});
  I(col, {type:"text", name:"评价文字", content:r.text, fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:400, lineHeight:1.6, width:"fill_container", textGrowth:"fixed-width", fill:[{type:"solid", color:"#57534E"}]});
}

// spacer before bottom bar
I(rail, {type:"frame", name:"底部留白", width:"fill_container", height:24, fill:[]});

// --- Bottom add-to-cart bar ---
const bar = I(sec, {type:"frame", name:"底部加购条", width:"fill_container", height:76, layout:"horizontal", padding:[0,24], alignItems:"center", justifyContent:"space_between", fill:[{type:"solid", color:"#FFFFFF"}], stroke:{thickness:{top:1}, fill:[{type:"solid", color:"#E7E5E4"}]}});
const totalCol = I(bar, {type:"frame", name:"合计区", width:"fit_content", height:"fit_content", layout:"vertical", gap:2});
I(totalCol, {type:"text", name:"合计标签", content:"合计", fontFamily:"Plus Jakarta Sans", fontSize:12, fontWeight:500, width:"fit_content", height:"fit_content", fill:[{type:"solid", color:"#A8A29E"}]});
const totalText = I(totalCol, {type:"text", name:"合计金额", content:"¥129", fontFamily:"Plus Jakarta Sans", fontSize:22, fontWeight:700, letterSpacing:-0.5, width:"fit_content", height:"fit_content", fill:[{type:"solid", color:"#EA580C"}], bindings:{content:"'¥' + (129 + ($app.capacity == '600ml' ? 20 : 0))"}});
const cartBtn = I(bar, {type:"frame", name:"加入购物车按钮", width:160, height:48, layout:"horizontal", alignItems:"center", justifyContent:"center", gap:8, cornerRadius:12, fill:[{type:"solid", color:"#F97316"}],
  events:{onTap:[
    {animate:{target:"$node", property:"scaleX", from:0.94, to:1, durationMs:220, easing:"ease_out", fillMode:"both"}},
    {animate:{target:"$node", property:"scaleY", from:0.94, to:1, durationMs:220, easing:"ease_out", fillMode:"both"}},
    {push:"\"/screen-1\""}
  ]}});
I(cartBtn, {type:"icon_font", name:"购物车图标", iconFontName:"shopping-cart", iconFontFamily:"lucide", width:18, height:18, fill:[{type:"solid", color:"#FFFFFF"}]});
I(cartBtn, {type:"text", name:"按钮文字", content:"加入购物车", fontFamily:"Plus Jakarta Sans", fontSize:15, fontWeight:600, width:"fit_content", height:"fit_content", fill:[{type:"solid", color:"#FFFFFF"}]});
