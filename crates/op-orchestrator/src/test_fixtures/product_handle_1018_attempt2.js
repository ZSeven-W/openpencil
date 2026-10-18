```js
const sec = I(null, {type:"frame", name:"商品详情页", screen:"/", width:"fill_container", height:"fit_content", layout:"vertical", fill:[{type:"solid",color:"#FFFFFF"}],
  state: {
    color: {type:"string", default:"奶白"},
    cap:   {type:"string", default:"450ml"}
  }});

// ── 导航栏 ──
const nav = I(sec, {type:"frame", name:"nav-bar", width:"fill_container", height:56, layout:"horizontal", alignItems:"center", justifyContent:"space_between", padding:[0,16]});
const backBtn = I(nav, {type:"frame", name:"back-button", width:40, height:40, layout:"horizontal", alignItems:"center", justifyContent:"center", cornerRadius:20, fill:[{type:"solid",color:"#FFF7ED"}]});
I(backBtn, {type:"icon_font", name:"back-icon", iconFontName:"chevron-left", iconFontFamily:"lucide", width:20, height:20, fill:[{type:"solid",color:"#F97316"}]});
I(nav, {type:"text", name:"nav-title", width:"fit_content", height:"fit_content", textGrowth:"auto", content:"商品详情", fontFamily:"Plus Jakarta Sans", fontSize:17, fontWeight:600, fill:[{type:"solid",color:"#1C1917"}]});
const shareBtn = I(nav, {type:"frame", name:"share-button", width:40, height:40, layout:"horizontal", alignItems:"center", justifyContent:"center", cornerRadius:20, fill:[{type:"solid",color:"#FFF7ED"}]});
I(shareBtn, {type:"icon_font", name:"share-icon", iconFontName:"share-2", iconFontFamily:"lucide", width:18, height:18, fill:[{type:"solid",color:"#57534E"}]});

// ── 大图轮播占位 (protagonist) ──
const hero = I(sec, {type:"frame", name:"carousel", width:"fill_container", height:380, layout:"none", clipContent:true, fill:[{type:"solid",color:"#FFF7ED"}]});
I(hero, {type:"image", name:"product-hero-image", x:0, y:0, width:375, height:380,
  imageSearchQuery:"white insulated bottle",
  imagePrompt:"A white minimalist stainless steel insulated travel bottle standing on a clean warm cream studio background, soft daylight, centered product photography"});
const dotWrap = I(hero, {type:"frame", name:"page-dots", x:163, y:352, width:56, height:8, layout:"horizontal", gap:8, alignItems:"center", justifyContent:"center"});
I(dotWrap, {type:"rectangle", name:"dot-active", width:16, height:6, cornerRadius:3, fill:[{type:"solid",color:"#1C1917"}]});
I(dotWrap, {type:"rectangle", name:"dot-2", width:6, height:6, cornerRadius:3, fill:[{type:"solid",color:"#E7E5E4"}]});
I(dotWrap, {type:"rectangle", name:"dot-3", width:6, height:6, cornerRadius:3, fill:[{type:"solid",color:"#E7E5E4"}]});

// ── 内容栏（24px rail）──
const body = I(sec, {type:"frame", name:"content-rail", width:"fill_container", height:"fit_content", layout:"vertical", padding:[24,24,0,24], gap:20});

// 商品信息块
const info = I(body, {type:"frame", name:"product-info", width:"fill_container", height:"fit_content", layout:"vertical", gap:8});
I(info, {type:"text", name:"product-name", width:"fill_container", height:"fit_content", textGrowth:"fixed-width", content:"轻氧保温随行杯 450ml", fontFamily:"Plus Jakarta Sans", fontSize:20, fontWeight:700, lineHeight:1.25, fill:[{type:"solid",color:"#1C1917"}]});
const priceRow = I(info, {type:"frame", name:"price-row", width:"fit_content", height:"fit_content", layout:"horizontal", alignItems:"center", gap:8});
const price = I(priceRow, {type:"text", name:"price-number", width:"fit_content", height:"fit_content", textGrowth:"auto",
  content:"¥129", fontFamily:"Plus Jakarta Sans", fontSize:32, fontWeight:700, letterSpacing:-1, fill:[{type:"solid",color:"#EA580C"}],
  bindings:{content:"$app.cap === '600ml' ? '¥159' : '¥129'"}});
I(priceRow, {type:"text", name:"selected-spec", width:"fit_content", height:"fit_content", textGrowth:"auto",
  content:"已选：奶白 · 450ml", fontFamily:"Plus Jakarta Sans", fontSize:12, fontWeight:500, fill:[{type:"solid",color:"#A8A29E"}],
  bindings:{content:"'已选：' + $app.color + ' · ' + $app.cap"}});

// 规格选择区
const spec = I(body, {type:"frame", name:"spec-section", width:"fill_container", height:"fit_content", layout:"vertical", gap:12});
const colorGroup = I(spec, {type:"frame", name:"color-group", width:"fill_container", height:"fit_content", layout:"horizontal", alignItems:"center", gap:12});
I(colorGroup, {type:"text", name:"color-label", width:48, height:"fit_content", textGrowth:"auto", content:"颜色", fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:500, fill:[{type:"solid",color:"#57534E"}]});
const colorChips = I(colorGroup, {type:"frame", name:"color-chips", width:"fit_content", height:"fit_content", layout:"horizontal", gap:8});
for (const c of ["奶白","墨绿"]) {
  I(colorChips, {type:"frame", name:"chip-color-" + c, width:"fit_content", height:36, layout:"horizontal", alignItems:"center", justifyContent:"center", padding:[0,16], cornerRadius:9999,
    fill:[{type:"solid",color:"#FFFFFF"}],
    stroke:{thickness:1, fill:[{type:"solid",color:"#E7E5E4"}]},
    bindings:{fill:"$app.color === '" + c + "' ? '#FFF7ED' : '#FFFFFF'", stroke:"$app.color === '" + c + "' ? '#F97316' : '#E7E5E4'"},
    events:{onTap:[{set:{"$app.color":"'"+c+"'"}},{animate:{target:"price-number",property:"translateY",from:8,to:0,durationMs:240,easing:"ease_out",iterations:1,direction:"normal",fillMode:"backwards"}},{animate:{target:"price-number",property:"opacity",from:0,to:1,durationMs:240,easing:"ease_out",iterations:1,direction:"normal",fillMode:"backwards"}}]},
    children:[{type:"text", name:"chip-color-text-" + c, width:"fit_content", height:"fit_content", textGrowth:"auto", content:c, fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:500,
      fill:[{type:"solid",color:"#57534E"}], bindings:{fill:"$app.color === '" + c + "' ? '#EA580C' : '#57534E'"}}]});
}
const capGroup = I(spec, {type:"frame", name:"capacity-group", width:"fill_container", height:"fit_content", layout:"horizontal", alignItems:"center", gap:12});
I(capGroup, {type:"text", name:"cap-label", width:48, height:"fit_content", textGrowth:"auto", content:"容量", fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:500, fill:[{type:"solid",color:"#57534E"}]});
const capChips = I(capGroup, {type:"frame", name:"cap-chips", width:"fit_content", height:"fit_content", layout:"horizontal", gap:8});
for (const v of ["450ml","600ml"]) {
  I(capChips, {type:"frame", name:"chip-cap-" + v, width:"fit_content", height:36, layout:"horizontal", alignItems:"center", justifyContent:"center", padding:[0,16], cornerRadius:9999,
    fill:[{type:"solid",color:"#FFFFFF"}],
    stroke:{thickness:1, fill:[{type:"solid",color:"#E7E5E4"}]},
    bindings:{fill:"$app.cap === '" + v + "' ? '#FFF7ED' : '#FFFFFF'", stroke:"$app.cap === '" + v + "' ? '#F97316' : '#E7E5E4'"},
    events:{onTap:[{set:{"$app.cap":"'"+v+"'"}},{animate:{target:"price-number",property:"translateY",from:8,to:0,durationMs:240,easing:"ease_out",iterations:1,direction:"normal",fillMode:"backwards"}},{animate:{target:"price-number",property:"opacity",from:0,to:1,durationMs:240,easing:"ease_out",iterations:1,direction:"normal",fillMode:"backwards"}}]},
    children:[{type:"text", name:"chip-cap-text-" + v, width:"fit_content", height:"fit_content", textGrowth:"auto", content:v, fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:500,
      fill:[{type:"solid",color:"#57534E"}], bindings:{fill:"$app.cap === '" + v + "' ? '#EA580C' : '#57534E'"}}]});
}

// 评价区
I(body, {type:"rectangle", name:"divider", width:"fill_container", height:1, fill:[{type:"solid",color:"#F5F5F4"}]});
const reviews = I(body, {type:"frame", name:"reviews-section", width:"fill_container", height:"fit_content", layout:"vertical", gap:16});
I(reviews, {type:"text", name:"reviews-title", width:"fit_content", height:"fit_content", textGrowth:"auto", content:"用户评价 (3)", fontFamily:"Plus Jakarta Sans", fontSize:16, fontWeight:600, fill:[{type:"solid",color:"#1C1917"}]});
const rvData = [
  {name:"云间月", stars:5, text:"保温效果很好，早上倒的热水到下午还是温热的，杯身奶白色也很有质感。"},
  {name:"林小满", stars:5, text:"大小刚好放进包里，杯口不漏，随行带着通勤很方便。"},
  {name:"一颗橘子", stars:4, text:"颜值在线，清洗也方便，希望再出一个更小的容量。"}
];
for (const r of rvData) {
  const row = I(reviews, {type:"frame", name:"review-row-" + r.name, width:"fill_container", height:"fit_content", layout:"horizontal", gap:12});
  const av = I(row, {type:"frame", name:"avatar-frame", width:36, height:36, cornerRadius:18, layout:"horizontal", alignItems:"center", justifyContent:"center", fill:[{type:"solid",color:"#FFF7ED"}], clipContent:true});
  I(av, {type:"image", name:"reviewer-avatar", width:36, height:36,
    imageSearchQuery:"person portrait avatar",
    imagePrompt:"A friendly casual portrait of a young person, soft natural lighting, warm neutral background, head and shoulders"});
  const txt = I(row, {type:"frame", name:"review-text-col", width:"fill_container", height:"fit_content", layout:"vertical", gap:4});
  const nameRow = I(txt, {type:"frame", name:"review-meta-row", width:"fit_content", height:"fit_content", layout:"horizontal", alignItems:"center", gap:8});
  I(nameRow, {type:"text", name:"reviewer-name", width:"fit_content", height:"fit_content", textGrowth:"auto", content:r.name, fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:600, fill:[{type:"solid",color:"#1C1917"}]});
  const starRow = I(nameRow, {type:"frame", name:"star-row", width:"fit_content", height:"fit_content", layout:"horizontal", gap:2});
  for (let i = 0; i < 5; i++) {
    I(starRow, {type:"icon_font", name:"star-" + i, iconFontName:"star", iconFontFamily:"lucide", width:12, height:12,
      fill:[{type:"solid", color: i < r.stars ? "#F97316" : "#E7E5E4"}]});
  }
  I(txt, {type:"text", name:"review-body", width:"fill_container", height:"fit_content", textGrowth:"fixed-width", content:r.text, fontFamily:"Plus Jakarta Sans", fontSize:13, fontWeight:400, lineHeight:1.6, fill:[{type:"solid",color:"#57534E"}]});
}
I(body, {type:"frame", name:"bottom-spacing", width:"fill_container", height:96, layout:"none"});

// ── 底部加购条 ──
const bar = I(sec, {type:"frame", name:"add-to-cart-bar", width:"fill_container", height:88, layout:"horizontal", alignItems:"center", justifyContent:"space_between", padding:[0,24], fill:[{type:"solid",color:"#FFFFFF"}], stroke:{thickness:{top:1}, fill:[{type:"solid",color:"#F5F5F4"}]}});
const totalCol = I(bar, {type:"frame", name:"total-col", width:"fit_content", height:"fit_content", layout:"vertical", gap:2});
I(totalCol, {type:"text", name:"total-label", width:"fit_content", height:"fit_content", textGrowth:"auto", content:"合计", fontFamily:"Plus Jakarta Sans", fontSize:11, fontWeight:500, fill:[{type:"solid",color:"#A8A29E"}]});
I(totalCol, {type:"text", name:"total-amount", width:"fit_content", height:"fit_content", textGrowth:"auto", content:"¥129", fontFamily:"Plus Jakarta Sans", fontSize:22, fontWeight:700, letterSpacing:-0.5, fill:[{type:"solid",color:"#EA580C"}], bindings:{content:"$app.cap === '600ml' ? '¥159' : '¥129'"}});
const cta = I(bar, {type:"frame", name:"add-to-cart-button", width:180, height:52, layout:"horizontal", alignItems:"center", justifyContent:"center", gap:8, cornerRadius:12,
  fill:[{type:"solid",color:"#F97316"}],
  events:{onTap:[{animate:{target:"add-to-cart-button",property:"scaleX",from:0.94,to:1,durationMs:180,easing:"ease_out",iterations:1,direction:"normal",fillMode:"backwards"}},{animate:{target:"add-to-cart-button",property:"scaleY",from:0.94,to:1,durationMs:180,easing:"ease_out",iterations:1,direction:"normal",fillMode:"backwards"}},{push:"\"/screen-1\""}]}});
I(cta, {type:"icon_font", name:"cart-icon", iconFontName:"shopping-cart", iconFontFamily:"lucide", width:18, height:18, fill:[{type:"solid",color:"#FFFFFF"}]});
I(cta, {type:"text", name:"cta-label", width:"fit_content", height:"fit_content", textGrowth:"auto", content:"加入购物车", fontFamily:"Plus Jakarta Sans", fontSize:16, fontWeight:600, fill:[{type:"solid",color:"#FFFFFF"}]});

// 导航返回提示
backBtn.events = {onTap:[{toast:"'已返回商品列表'"}]};
shareBtn.events = {onTap:[{toast:"'已复制商品链接'"}]};
```
