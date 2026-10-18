const sec = I(null, {type:"frame", name:"商品页", width:375, height:"fit_content", layout:"vertical", clipContent:true, fill:[{type:"solid",color:"#FFFFFF"}],
  state:{capacity:{type:"string",default:"350ml"}, color:{type:"string",default:"奶油白"}}});

// ---------- HERO ----------
const hero = I(sec, {type:"frame", name:"hero-viewport", layout:"none", width:375, height:400, clipContent:true});
I(hero, {type:"image", name:"hero-image", width:375, height:400,
  imageSearchQuery:"cream ceramic mug", imagePrompt:"A cream-white ceramic mug with soft oat-grey glaze detail on a warm linen surface, minimal lifestyle product photography, soft natural window light, warm tones, gentle shadow, vertical composition",
  effects:[{type:"shadow", offsetX:0, offsetY:8, blur:24, spread:0, color:"#1C191720"}]});

// floating back / share buttons
const backBtn = I(hero, {type:"frame", name:"btn-back", layout:"horizontal", justifyContent:"center", alignItems:"center", width:36, height:36, cornerRadius:18, fill:[{type:"solid",color:"#FFFFFF"}], effects:[{type:"shadow", offsetX:0, offsetY:2, blur:8, spread:0, color:"#1C19171A"}], events:{onTap:[{pop:null}]}});
I(backBtn, {type:"icon_font", iconFontName:"chevron-left", iconFontFamily:"lucide", width:20, height:20, fill:"#1C1917"});
const shareBtn = I(hero, {type:"frame", name:"btn-share", layout:"horizontal", justifyContent:"center", alignItems:"center", width:36, height:36, cornerRadius:18, fill:[{type:"solid",color:"#FFFFFF"}], effects:[{type:"shadow", offsetX:0, offsetY:2, blur:8, spread:0, color:"#1C19171A"}]});
I(shareBtn, {type:"icon_font", iconFontName:"share-2", iconFontFamily:"lucide", width:18, height:18, fill:"#1C1917"});
// hero scrim + title overlay
I(hero, {type:"rectangle", name:"hero-scrim", width:375, height:120, x:0, y:280, fill:[{type:"linear_gradient", angle:180, stops:[{offset:0, color:"#FFFFFF00"},{offset:1, color:"#FFFFFFCC"}]}]});
I(hero, {type:"text", name:"hero-caption", x:24, y:352, width:280, height:20, content:"轻氧家居 · 手作陶瓷系列", fontFamily:"Plus Jakarta Sans", fontSize:12, fontWeight:500, fill:"#57534E", letterSpacing:2});

// ---------- PRICE INFO CARD (overlapping) ----------
const infoCard = I(hero, {type:"frame", name:"info-card", layout:"vertical", x:0, y:376, width:375, padding:[24,24,20,24], gap:8, cornerRadius:24, fill:[{type:"solid",color:"#FFFFFF"}], effects:[{type:"shadow", offsetX:0, offsetY:-8, blur:24, spread:0, color:"#1C191714"}],
  animations:[{trigger:"mount", durationMs:500, easing:"emphasizedDecelerate", keyframes:[{offset:0, values:{translateY:32, opacity:0}},{offset:1, values:{translateY:0, opacity:1}}]}]});

const priceRow = I(infoCard, {type:"frame", name:"price-row", layout:"horizontal", alignItems:"end", gap:8, width:"fill_container",
  animations:[{trigger:"mount", durationMs:700, easing:"emphasizedDecelerate", keyframes:[{offset:0, values:{opacity:0, translateY:10}},{offset:0.5, values:{opacity:1, translateY:0}},{offset:1, values:{opacity:1, translateY:0}}]}]});
I(priceRow, {type:"text", name:"price-current", content:"¥89.00", fontFamily:"Plus Jakarta Sans", fontSize:32, fontWeight:700, fill:"#EA580C", letterSpacing:-0.5,
  animations:[{trigger:"mount", durationMs:800, easing:"emphasizedDecelerate", keyframes:[{offset:0, values:{opacity:0, translateX:-16}},{offset:1, values:{opacity:1, translateX:0}}]}]});
I(priceRow, {type:"text", name:"price-original", content:"¥119.00", fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:400, fill:"#A8A29E"});
const saleBadge = I(priceRow, {type:"frame", name:"sale-badge", layout:"horizontal", alignItems:"center", padding:[3,8], cornerRadius:8, fill:[{type:"solid",color:"#FFF7ED"}]});
I(saleBadge, {type:"text", content:"限时直降", fontFamily:"Plus Jakarta Sans", fontSize:11, fontWeight:500, fill:"#EA580C", letterSpacing:1});

I(infoCard, {type:"text", name:"product-title", content:"轻氧陶瓷马克杯", fontFamily:"Plus Jakarta Sans", fontSize:20, fontWeight:600, fill:"#1C1917", letterSpacing:-0.5});
I(infoCard, {type:"text", name:"product-subtitle", content:"哑光釉面 · 手感温润，一只属于晨间的杯子", fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:400, fill:"#57534E", lineHeight:1.6});

// ---------- SPECS ----------
const specs = I(sec, {type:"frame", name:"specs-section", layout:"vertical", width:"fill_container", padding:[20,24,4,24], gap:14});
const capGroup = I(specs, {type:"frame", name:"group-capacity", layout:"vertical", width:"fill_container", gap:10});
I(capGroup, {type:"text", content:"容量", fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:500, fill:"#57534E"});
const capRow = I(capGroup, {type:"frame", layout:"horizontal", gap:10, width:"fill_container"});
for (const c of ["350ml","450ml"]) {
  const chip = I(capRow, {type:"frame", name:"chip-capacity-"+c, layout:"horizontal", justifyContent:"center", alignItems:"center", padding:[9,20], cornerRadius:9999,
    fill:[{type:"solid",color:"#FFFFFF"}],
    stroke:{thickness:1, fill:[{type:"solid",color:"#E7E5E4"}]},
    bindings:{fill:"$state.capacity == '"+c+"' ? '#1C1917' : '#FFFFFF'", stroke:"$state.capacity == '"+c+"' ? {thickness:1, fill:[{type:'solid', color:'#1C1917'}]} : {thickness:1, fill:[{type:'solid', color:'#E7E5E4'}]}"},
    animations:[{trigger:"tap", durationMs:200, easing:"standard", keyframes:[{offset:0, values:{scaleX:1, scaleY:1}},{offset:0.5, values:{scaleX:0.95, scaleY:0.95}},{offset:1, values:{scaleX:1, scaleY:1}}]}],
    events:{onTap:[{set:{"$state.capacity":c}}]}});
  I(chip, {type:"text", content:c, fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:500, fill:"#57534E",
    bindings:{fill:"$state.capacity == '"+c+"' ? '#FFFFFF' : '#57534E'"}});
}
const colorGroup = I(specs, {type:"frame", name:"group-color", layout:"vertical", width:"fill_container", gap:10});
I(colorGroup, {type:"text", content:"颜色", fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:500, fill:"#57534E"});
const colorRow = I(colorGroup, {type:"frame", layout:"horizontal", gap:10, width:"fill_container"});
for (const c of ["奶油白","燕麦灰"]) {
  const chip = I(colorRow, {type:"frame", name:"chip-color-"+c, layout:"horizontal", justifyContent:"center", alignItems:"center", padding:[9,20], cornerRadius:9999,
    fill:[{type:"solid",color:"#FFFFFF"}],
    stroke:{thickness:1, fill:[{type:"solid",color:"#E7E5E4"}]},
    bindings:{fill:"$state.color == '"+c+"' ? '#1C1917' : '#FFFFFF'", stroke:"$state.color == '"+c+"' ? {thickness:1, fill:[{type:'solid', color:'#1C1917'}]} : {thickness:1, fill:[{type:'solid', color:'#E7E5E4'}]}"},
    animations:[{trigger:"tap", durationMs:200, easing:"standard", keyframes:[{offset:0, values:{scaleX:1, scaleY:1}},{offset:0.5, values:{scaleX:0.95, scaleY:0.95}},{offset:1, values:{scaleX:1, scaleY:1}}]}],
    events:{onTap:[{set:{"$state.color":c}}]}});
  I(chip, {type:"text", content:c, fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:500, fill:"#57534E",
    bindings:{fill:"$state.color == '"+c+"' ? '#FFFFFF' : '#57534E'"}});
}

// ---------- REVIEWS ----------
const reviews = I(sec, {type:"frame", name:"reviews-section", layout:"vertical", width:"fill_container", padding:[24,24,24,24], gap:16});
const revHead = I(reviews, {type:"frame", layout:"horizontal", justifyContent:"space_between", alignItems:"center", width:"fill_container"});
I(revHead, {type:"text", content:"用户评价", fontFamily:"Plus Jakarta Sans", fontSize:18, fontWeight:600, fill:"#1C1917", letterSpacing:-0.5});
const revMore = I(revHead, {type:"frame", layout:"horizontal", alignItems:"center", gap:2, events:{onTap:[{toast:"查看全部评价"}]}});
I(revMore, {type:"text", content:"全部", fontFamily:"Plus Jakarta Sans", fontSize:13, fontWeight:500, fill:"#A8A29E"});
I(revMore, {type:"icon_font", iconFontName:"chevron-right", iconFontFamily:"lucide", width:14, height:14, fill:"#A8A29E"});

const reviewData = [
  {name:"柚子茶", stars:5, text:"杯子比想象中还温润，奶油白和木桌特别搭，晨间喝咖啡幸福感拉满。"},
  {name:"Momo", stars:5, text:"釉面很均匀，重量刚刚好，握在手里的弧度很舒服。"},
  {name:"林一舟", stars:4, text:"做工细致，450ml 容量对办公桌很友好，希望多出几个颜色。"}
];
for (const r of reviewData) {
  const card = I(reviews, {type:"frame", name:"review-"+r.name, layout:"vertical", width:"fill_container", padding:16, gap:10, cornerRadius:16, fill:[{type:"solid",color:"#FFFFFF"}],
    stroke:{thickness:1, fill:[{type:"solid",color:"#F5F5F4"}]},
    animations:[{trigger:"inView", durationMs:400, easing:"emphasizedDecelerate", keyframes:[{offset:0, values:{opacity:0, translateY:16}},{offset:1, values:{opacity:1, translateY:0}}]}]});
  const head = I(card, {type:"frame", layout:"horizontal", alignItems:"center", gap:10, width:"fill_container"});
  const avatar = I(head, {type:"ellipse", width:36, height:36, fill:[{type:"solid",color:"#FFF7ED"}], stroke:{thickness:1, fill:[{type:"solid",color:"#FED7AA"}]}});
  I(head, {type:"text", content:r.name, fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:500, fill:"#1C1917"});
  const stars = I(head, {type:"frame", layout:"horizontal", gap:2});
  for (let i=0;i<r.stars;i++) I(stars, {type:"icon_font", iconFontName:"star", iconFontFamily:"lucide", width:14, height:14, fill:"#F97316"});
  I(card, {type:"text", content:r.text, fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:400, fill:"#57534E", lineHeight:1.6, width:"fill_container"});
}

// ---------- BOTTOM BAR ----------
const bar = I(sec, {type:"frame", name:"bottom-buy-bar", layout:"horizontal", width:"fill_container", padding:[12,24,20,24], gap:12, alignItems:"center", fill:[{type:"solid",color:"#FFFFFF"}], stroke:{thickness:{top:1}, fill:[{type:"solid",color:"#F5F5F4"}]}, effects:[{type:"shadow", offsetX:0, offsetY:-4, blur:16, spread:0, color:"#1C191710"}]});
const svc = I(bar, {type:"frame", name:"btn-service", layout:"vertical", alignItems:"center", gap:2, width:48, events:{onTap:[{toast:"客服会话即将开启"}]}});
I(svc, {type:"icon_font", iconFontName:"message-circle", iconFontFamily:"lucide", width:20, height:20, fill:"#57534E"});
I(svc, {type:"text", content:"客服", fontFamily:"Plus Jakarta Sans", fontSize:11, fontWeight:400, fill:"#A8A29E"});
const cartBtn = I(bar, {type:"frame", name:"btn-add-cart", layout:"horizontal", justifyContent:"center", alignItems:"center", width:"fill_container", padding:[14,0], cornerRadius:12, fill:[{type:"solid",color:"#1C1917"}],
  animations:[{trigger:"tap", durationMs:200, easing:"standard", keyframes:[{offset:0, values:{scaleX:1, scaleY:1}},{offset:0.5, values:{scaleX:0.96, scaleY:0.96}},{offset:1, values:{scaleX:1, scaleY:1}}]}],
  events:{onTap:[{toast:"已加入购物车"}, {push:"\"/screen-1\""}]}});
I(cartBtn, {type:"text", content:"加入购物车", fontFamily:"Plus Jakarta Sans", fontSize:15, fontWeight:600, fill:"#FFFFFF"});
const buyBtn = I(bar, {type:"frame", name:"btn-buy-now", layout:"horizontal", justifyContent:"center", alignItems:"center", width:"fill_container", padding:[14,0], cornerRadius:12, fill:[{type:"solid",color:"#F97316"}], effects:[{type:"shadow", offsetX:0, offsetY:4, blur:12, spread:0, color:"#F9731640"}],
  animations:[{trigger:"tap", durationMs:200, easing:"standard", keyframes:[{offset:0, values:{scaleX:1, scaleY:1}},{offset:0.5, values:{scaleX:0.96, scaleY:0.96}},{offset:1, values:{scaleX:1, scaleY:1}}]}],
  events:{onTap:[{toast:"正在为你创建订单…"}]}});
I(buyBtn, {type:"text", content:"立即购买", fontFamily:"Plus Jakarta Sans", fontSize:15, fontWeight:600, fill:"#FFFFFF"});