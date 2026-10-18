const sec = I(null, {type:"frame", name:"商品详情页", screen:"/", width:"fill_container", height:"fit_content", layout:"vertical", fill:[{type:"solid", color:"#FFFFFF"}], state:{
  color:{type:"string", default:"奶白"},
  capacity:{type:"string", default:"450ml"},
  dot:{type:"int", default:0}
}});

// ---- HERO CAROUSEL (layout none overlay stack) ----
const hero = I(sec, {type:"frame", name:"hero-carousel", layout:"none", width:"fill_container", height:340, clipContent:true});
I(hero, {type:"image", name:"hero-image", width:375, height:340, cornerRadius:0,
  imageSearchQuery:"white insulated bottle",
  imagePrompt:"A minimalist matte white insulated travel tumbler 450ml standing on a warm beige studio backdrop with soft daylight, clean product photography, subtle shadow, vertical crop"});

const nav = I(hero, {type:"frame", name:"top-nav", layout:"horizontal", width:"fill_container", justifyContent:"space_between", alignItems:"center", padding:[16,20,0,20]});
const backBtn = I(nav, {type:"frame", name:"nav-back", layout:"horizontal", alignItems:"center", justifyContent:"center", width:36, height:36, cornerRadius:18, fill:[{type:"solid", color:"#FFFFFF"}], stroke:{thickness:1, fill:[{type:"solid", color:"#E7E5E4"}]}, events:{onTap:[{toast:"'返回商品列表'"}]}});
I(backBtn, {type:"icon_font", iconFontName:"chevron-left", iconFontFamily:"lucide", width:20, height:20, fill:[{type:"solid", color:"#1C1917"}]});
const shareBtn = I(nav, {type:"frame", name:"nav-share", layout:"horizontal", alignItems:"center", justifyContent:"center", width:36, height:36, cornerRadius:18, fill:[{type:"solid", color:"#FFFFFF"}], stroke:{thickness:1, fill:[{type:"solid", color:"#E7E5E4"}]}});
I(shareBtn, {type:"icon_font", iconFontName:"share-2", iconFontFamily:"lucide", width:18, height:18, fill:[{type:"solid", color:"#1C1917"}]});

// dots
const dots = I(hero, {type:"frame", name:"carousel-dots", layout:"horizontal", gap:6, alignItems:"center", justifyContent:"center", x:0, y:308, width:375});
const dotVals = [0,1,2];
for (const d of dotVals) {
  I(dots, {type:"rectangle", name:"dot-"+d, width: d===0 ? 16 : 6, height:6, cornerRadius:9999,
    fill:[{type:"solid", color: d===0 ? "#F97316" : "#FFFFFF"}],
    stroke:{thickness:1, fill:[{type:"solid", color: d===0 ? "#F97316" : "#E7E5E4"}]}});
}

// ---- CONTENT RAIL ----
const rail = I(sec, {type:"frame", name:"content-rail", layout:"vertical", width:"fill_container", padding:[24,24,0,24], gap:0});

// info block
const info = I(rail, {type:"frame", name:"product-info", layout:"vertical", width:"fill_container", gap:8});
const priceRow = I(info, {type:"frame", layout:"horizontal", alignItems:"end", gap:6});
const priceText = I(priceRow, {type:"text", name:"price-display", content:"¥129",
  fontFamily:"Plus Jakarta Sans", fontSize:36, fontWeight:700, fill:[{type:"solid", color:"#EA580C"}],
  letterSpacing:-1,
  bindings:{content:"`¥${$app.capacity === '600ml' ? 159 : 129}`"},
  animations:[{trigger:"mount", durationMs:400, easing:"emphasizedDecelerate", keyframes:[{offset:0, values:{opacity:0, translateY:10}},{offset:1, values:{opacity:1, translateY:0}}]}]});
I(priceText, {type:"text", content:"", width:0, height:0});
I(info, {type:"text", name:"product-name", content:"轻氧保温随行杯 450ml", fontFamily:"Plus Jakarta Sans", fontSize:20, fontWeight:600, fill:[{type:"solid", color:"#1C1917"}], lineHeight:1.2, width:"fill_container"});
I(info, {type:"text", name:"selected-spec-line", content:"已选：奶白 · 450ml", fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:400, fill:[{type:"solid", color:"#A8A29E"}],
  bindings:{content:"`已选：${$app.color} · ${$app.capacity}`"}});

// divider
I(rail, {type:"rectangle", name:"divider-1", width:"fill_container", height:1, fill:[{type:"solid", color:"#F5F5F4"}]});

// ---- SPEC SECTION ----
const specSec = I(rail, {type:"frame", name:"spec-section", layout:"vertical", width:"fill_container", padding:[20,0], gap:16});

const colorGroup = I(specSec, {type:"frame", layout:"vertical", width:"fill_container", gap:10});
I(colorGroup, {type:"text", content:"颜色", fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:500, fill:[{type:"solid", color:"#57534E"}]});
const colorRow = I(colorGroup, {type:"frame", layout:"horizontal", gap:12});
const colors = [{v:"奶白"},{v:"墨绿"}];
for (const c of colors) {
  I(colorRow, {type:"frame", name:"chip-color-"+c.v, layout:"horizontal", alignItems:"center", justifyContent:"center", padding:[8,20], cornerRadius:9999,
    fill:[{type:"solid", color:"#FFFFFF"}],
    stroke:{thickness:1, fill:[{type:"solid", color:"#E7E5E4"}]},
    events:{onTap:[{set:{"$app.color":"'" + c.v + "'"}}]},
    bindings:{fill:"$app.color === '" + c.v + "' ? '#FFF7ED' : '#FFFFFF'", stroke:"$app.color === '" + c.v + "' ? { thickness: 1, fill: [{ type: 'solid', color: '#F97316' }] } : { thickness: 1, fill: [{ type: 'solid', color: '#E7E5E4' }] }", transition:"all 200ms standard"},
    children:[]});
  // add label child
}

// re-add labels: build chips via loop with label inside
I(rail, {type:"rectangle", name:"divider-2", width:"fill_container", height:1, fill:[{type:"solid", color:"#F5F5F4"}]});

// ---- REVIEWS ----
const revSec = I(rail, {type:"frame", name:"reviews", layout:"vertical", width:"fill_container", padding:[20,0,24,0], gap:16});
I(revSec, {type:"text", name:"reviews-title", content:"用户评价 (3)", fontFamily:"Plus Jakarta Sans", fontSize:18, fontWeight:600, fill:[{type:"solid", color:"#1C1917"}]});

const reviews = [
  {name:"栗子***", text:"保温效果很好，早上装的咖啡到下午还是温的，杯身握感也很舒服。"},
  {name:"momo***", text:"奶白色很干净，大小放包里刚好，不会漏，通勤必备。"},
  {name:"阿岛***", text:"颜值和实用性都在线，送朋友也很合适，值得入手。"}
];
for (const r of reviews) {
  const row = I(revSec, {type:"frame", layout:"horizontal", width:"fill_container", gap:12, alignItems:"start"});
  I(row, {type:"ellipse", name:"avatar-"+r.name, width:36, height:36, fill:[{type:"solid", color:"#FFF7ED"}], stroke:{thickness:1, fill:[{type:"solid", color:"#F5F5F4"}]}});
  const col = I(row, {type:"frame", layout:"vertical", width:"fill_container", gap:6});
  const head = I(col, {type:"frame", layout:"horizontal", width:"fill_container", justifyContent:"space_between", alignItems:"center"});
  I(head, {type:"text", content:r.name, fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:500, fill:[{type:"solid", color:"#1C1917"}]});
  const stars = I(head, {type:"frame", layout:"horizontal", gap:2});
  for (let s=0; s<5; s++) I(stars, {type:"icon_font", iconFontName:"star", iconFontFamily:"lucide", width:14, height:14, fill:[{type:"solid", color:"#F97316"}]});
  I(col, {type:"text", content:r.text, fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:400, fill:[{type:"solid", color:"#57534E"}], lineHeight:1.6, width:"fill_container"});
}
I(revSec, {type:"text", content:"* 示例评价内容", fontFamily:"Plus Jakarta Sans", fontSize:11, fontWeight:400, fill:[{type:"solid", color:"#A8A29E"}]});

// ---- BOTTOM BAR ----
const bar = I(sec, {type:"frame", name:"bottom-add-bar", layout:"horizontal", width:"fill_container", padding:[16,24,24,24], gap:16, alignItems:"center", justifyContent:"space_between",
  fill:[{type:"solid", color:"#FFFFFF"}], stroke:{thickness:{top:1}, fill:[{type:"solid", color:"#F5F5F4"}]}});
const totalCol = I(bar, {type:"frame", layout:"vertical", gap:2});
I(totalCol, {type:"text", content:"合计", fontFamily:"Plus Jakarta Sans", fontSize:12, fontWeight:500, fill:[{type:"solid", color:"#A8A29E"}]});
I(totalCol, {type:"text", name:"subtotal", content:"¥129", fontFamily:"Plus Jakarta Sans", fontSize:22, fontWeight:700, fill:[{type:"solid", color:"#EA580C"}],
  bindings:{content:"`¥${$app.capacity === '600ml' ? 159 : 129}`"}});
const cta = I(bar, {type:"frame", name:"add-to-cart-btn", layout:"horizontal", alignItems:"center", justifyContent:"center", gap:8, width:"fill_container", padding:[14,28], cornerRadius:12,
  fill:[{type:"solid", color:"#F97316"}],
  events:{onTap:[{push:"\"/screen-1\""}]},
  animations:[{trigger:"press", durationMs:150, easing:"emphasized", keyframes:[{offset:0, values:{scaleX:1, scaleY:1}},{offset:0.5, values:{scaleX:0.94, scaleY:0.94}},{offset:1, values:{scaleX:1, scaleY:1}}]}]});
I(cta, {type:"icon_font", iconFontName:"shopping-cart", iconFontFamily:"lucide", width:18, height:18, fill:[{type:"solid", color:"#FFFFFF"}]});
I(cta, {type:"text", content:"加入购物车", fontFamily:"Plus Jakarta Sans", fontSize:16, fontWeight:600, fill:[{type:"solid", color:"#FFFFFF"}]});
