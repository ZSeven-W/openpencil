const sec = I(null, {type:"frame", name:"商家详情", width:"fill_container", height:"fit_content", layout:"vertical", clipContent:true});

// ---- Full-bleed hero photo with overlaid controls ----
const heroWrap = I(sec, {type:"frame", name:"hero-viewport", width:"fill_container", height:200, layout:"none", clipContent:true});
// controls first (topmost)
const ctrlRow = I(heroWrap, {type:"frame", name:"hero-controls", x:0, y:16, width:375, height:36, layout:"horizontal", justifyContent:"space_between", padding:[0,16], fill:[]});
const backBtn = I(ctrlRow, {type:"frame", name:"back-button", width:36, height:36, cornerRadius:18, fill:[{type:"solid", color:"#00000066"}], layout:"horizontal", alignItems:"center", justifyContent:"center", events:{onTap:[{pop:null}]}});
I(backBtn, {type:"icon_font", iconFontName:"chevron-left", iconFontFamily:"lucide", width:20, height:20, fill:[{type:"solid", color:"#FFFFFF"}]});
const rightCtrl = I(ctrlRow, {type:"frame", name:"right-controls", layout:"horizontal", gap:12});
for (const ic of ["heart","share"]) {
  const b = I(rightCtrl, {type:"frame", name:ic+"-button", width:36, height:36, cornerRadius:18, fill:[{type:"solid", color:"#00000066"}], layout:"horizontal", alignItems:"center", justifyContent:"center"});
  I(b, {type:"icon_font", iconFontName:ic, iconFontFamily:"lucide", width:18, height:18, fill:[{type:"solid", color:"#FFFFFF"}]});
}
// hero photo (behind controls)
I(heroWrap, {type:"image", name:"store-hero-photo", x:0, y:0, width:375, height:200,
  imageSearchQuery:"noodle restaurant interior",
  imagePrompt:"Warm cozy interior of a small Chinese noodle restaurant, wooden tables, soft evening light, appetizing atmosphere"});

// ---- Logo overlapping hero bottom ----
const logoRow = I(sec, {type:"frame", name:"store-identity", width:"fill_container", layout:"vertical", padding:[0,24,0,24], alignItems:"start"});
const logoBand = I(logoRow, {type:"frame", name:"logo-band", width:"fill_container", layout:"horizontal", alignItems:"end", justifyContent:"space_between", padding:[0,0,8,0]});
const logoCard = I(logoBand, {type:"frame", name:"store-logo", width:60, height:60, cornerRadius:14, clipContent:true, fill:[{type:"solid", color:"#FFFFFF"}], stroke:{thickness:2, fill:[{type:"solid", color:"#FFFFFF"}]}, effects:[{type:"shadow", offsetX:0, offsetY:4, blur:12, spread:0, color:"#00000022"}], layout:"none"});
I(logoCard, {type:"image", name:"store-logo-image", x:0, y:0, width:60, height:60,
  imageSearchQuery:"bowl of noodles",
  imagePrompt:"Close-up of a steaming bowl of hand-pulled beef noodles in ceramic bowl, overhead shot, warm light"});

// ---- Store info ----
const info = I(sec, {type:"frame", name:"store-info", width:"fill_container", layout:"vertical", padding:[8,24,16,24], gap:8});
I(info, {type:"text", name:"store-name", content:"老陈家·手作牛肉面", fontSize:22, fontWeight:700, fill:[{type:"solid", color:"#0F172A"}], width:"fill_container", textGrowth:"fixed-width", lineHeight:1.3});
const metaRow = I(info, {type:"frame", name:"store-meta", layout:"horizontal", gap:8, alignItems:"center"});
I(metaRow, {type:"icon_font", iconFontName:"star", iconFontFamily:"lucide", width:14, height:14, fill:[{type:"solid", color:"#F59E0B"}]});
I(metaRow, {type:"text", name:"store-meta-text", content:"4.8 · 月售 3200+ · 30分钟 · 1.2km", fontSize:13, fontWeight:400, fill:[{type:"solid", color:"#64748B"}], width:"fit_content", textGrowth:"auto"});
const tagRow = I(info, {type:"frame", name:"promo-tags", layout:"horizontal", gap:8});
for (const t of ["满30减12","新客立减5"]) {
  const tag = I(tagRow, {type:"frame", name:"promo-tag", cornerRadius:4, fill:[{type:"solid", color:"#FEF2F2"}], layout:"horizontal", alignItems:"center", padding:[3,8]});
  I(tag, {type:"text", name:"promo-tag-text", content:t, fontSize:12, fontWeight:500, fill:[{type:"solid", color:"#DC2626"}], width:"fit_content", textGrowth:"auto"});
}
I(info, {type:"text", name:"store-announcement", content:"公告：本店牛肉每日现切，汤底熬制 8 小时，欢迎堂食与外带～", fontSize:12, fontWeight:400, fill:[{type:"solid", color:"#94A3B8"}], width:"fill_container", textGrowth:"fixed-width", lineHeight:1.5});

// ---- Category horizontal scroller ----
const catSec = I(sec, {type:"frame", name:"dish-categories", width:"fill_container", layout:"vertical", gap:12, padding:[4,0,4,0]});
const catRail = I(catSec, {type:"frame", name:"category-rail", width:"fill_container", height:44, layout:"horizontal", clipContent:true, padding:[0,0,0,24], gap:12, alignItems:"center"});
const cats = [
  {v:"rec", label:"商家推荐", active:true},
  {v:"noodle", label:"招牌面食"},
  {v:"side", label:"风味小食"},
  {v:"drink", label:"饮品"},
  {v:"combo", label:"超值套餐"},
];
for (const c of cats) {
  const chip = I(catRail, {type:"frame", name:"category-chip-"+c.v, cornerRadius:18, height:36, layout:"horizontal", alignItems:"center", padding:[0,16],
    fill:[{type:"solid", color: c.active ? "#DC2626" : "#FFFFFF"}],
    stroke:{thickness:1, fill:[{type:"solid", color: c.active ? "#DC2626" : "#E2E8F0"}]}});
  I(chip, {type:"text", name:"category-chip-text", content:c.label, fontSize:14, fontWeight: c.active ? 600 : 400,
    fill:[{type:"solid", color: c.active ? "#FFFFFF" : "#475569"}], width:"fit_content", textGrowth:"auto"});
}

// ---- Dish list (6 rows) ----
const dishSec = I(sec, {type:"frame", name:"dish-list", width:"fill_container", layout:"vertical", padding:[8,24,16,24], gap:16});
const dishes = [
  {n:"招牌红烧牛肉面", d:"牛腱现切，汤浓面弹，配溏心蛋", p:"¥28", o:"¥32", s:"月售 1260"},
  {n:"清炖羊肉面", d:"西北风味，汤清味鲜，暖胃首选", p:"¥32", s:"月售 856"},
  {n:"酸汤肥牛面", d:"金汤酸辣开胃，肥牛厚切", p:"¥26", s:"月售 742"},
  {n:"手拍黄瓜", d:"蒜香微辣，清爽解腻小菜", p:"¥8", s:"月售 980"},
  {n:"秘制卤蛋", d:"八小时老卤，入味弹嫩", p:"¥3", s:"月售 2100"},
  {n:"现熬酸梅汤", d:"生津解腻，冰镇更佳 300ml", p:"¥6", s:"月售 1500"},
];
for (const dme of dishes) {
  const row = I(dishSec, {type:"frame", name:"dish-row-"+dme.n, layout:"horizontal", width:"fill_container", gap:12, alignItems:"start"});
  const img = I(row, {type:"frame", name:"dish-photo-wrap", width:88, height:88, cornerRadius:10, clipContent:true, layout:"none"});
  I(img, {type:"image", name:"dish-photo", x:0, y:0, width:88, height:88,
    imageSearchQuery: dme.n + " dish",
    imagePrompt: dme.n + "，" + dme.d + "，placed in a ceramic bowl on a wooden table, overhead shot, warm appetizing light"});
  const body = I(row, {type:"frame", name:"dish-body", layout:"vertical", width:"fill_container", height:88, gap:3});
  I(body, {type:"text", name:"dish-name", content:dme.n, fontSize:15, fontWeight:600, fill:[{type:"solid", color:"#0F172A"}], width:"fill_container", textGrowth:"fixed-width", lineHeight:1.3});
  I(body, {type:"text", name:"dish-desc", content:dme.d, fontSize:12, fontWeight:400, fill:[{type:"solid", color:"#94A3B8"}], width:"fill_container", textGrowth:"fixed-width", lineHeight:1.4});
  I(body, {type:"text", name:"dish-sales", content:dme.s, fontSize:12, fontWeight:400, fill:[{type:"solid", color:"#CBD5E1"}], width:"fit_content", textGrowth:"auto"});
  const priceRow = I(body, {type:"frame", name:"dish-price-row", layout:"horizontal", width:"fill_container", justifyContent:"space_between", alignItems:"center"});
  const pr = I(priceRow, {type:"frame", name:"dish-price", layout:"horizontal", gap:6, alignItems:"end"});
  I(pr, {type:"text", name:"dish-price-now", content:dme.p, fontSize:18, fontWeight:700, fill:[{type:"solid", color:"#DC2626"}], width:"fit_content", textGrowth:"auto"});
  if (dme.o) I(pr, {type:"text", name:"dish-price-old", content:dme.o, fontSize:12, fontWeight:400, fill:[{type:"solid", color:"#CBD5E1"}], width:"fit_content", textGrowth:"auto"});
  const addBtn = I(priceRow, {type:"frame", name:"dish-add-button", width:32, height:32, cornerRadius:16,
    fill:[{type:"solid", color:"#DC2626"}],
    layout:"horizontal", alignItems:"center", justifyContent:"center",
    animations:[{trigger:"press", durationMs:150, easing:"emphasizedDecelerate", keyframes:[{offset:0, values:{scaleX:1, scaleY:1}},{offset:0.5, values:{scaleX:0.88, scaleY:0.88}},{offset:1, values:{scaleX:1, scaleY:1}}]}],
    events:{onTap:[{set:{"$app.cartCount":"$app.cartCount + 1"}},{toast:"已加入购物车"}]}});
  I(addBtn, {type:"icon_font", iconFontName:"plus", iconFontFamily:"lucide", width:16, height:16, fill:[{type:"solid", color:"#FFFFFF"}]});
}

// ---- Cart bar ----
const cartBarSec = I(sec, {type:"frame", name:"cart-bar-section", width:"fill_container", layout:"vertical", padding:[8,24,28,24]});
const cartBar = I(cartBarSec, {type:"frame", name:"cart-bar", role:"cart-bar", width:"fill_container", height:56, cornerRadius:28,
  fill:[{type:"solid", color:"#1E293B"}],
  effects:[{type:"shadow", offsetX:0, offsetY:6, blur:16, spread:0, color:"#0F172A33"}],
  layout:"horizontal", alignItems:"center", paddingLeft:64, paddingRight:6,
  justifyContent:"space_between"});
const cartDiscWrap = I(cartBar, {type:"frame", name:"cart-disc-wrap", width:0, height:0, layout:"none", clipContent:false});
const cartDisc = I(cartDiscWrap, {type:"frame", name:"cart-disc", x:-46, y:-24, width:48, height:48, cornerRadius:24,
  fill:[{type:"solid", color:"#DC2626"}],
  stroke:{thickness:4, fill:[{type:"solid", color:"#1E293B"}]},
  layout:"horizontal", alignItems:"center", justifyContent:"center"});
I(cartDisc, {type:"icon_font", iconFontName:"shopping-cart", iconFontFamily:"lucide", width:20, height:20, fill:[{type:"solid", color:"#FFFFFF"}]});
const cartCount = I(cartDisc, {type:"frame", name:"cart-count-badge", width:18, height:18, cornerRadius:9,
  fill:[{type:"solid", color:"#FBBF24"}], layout:"horizontal", alignItems:"center", justifyContent:"center"});
I(cartCount, {type:"text", name:"cart-count-text", content:"0", fontSize:11, fontWeight:700, fill:[{type:"solid", color:"#1E293B"}], width:"fit_content", textGrowth:"auto", bindings:{content:"\"\" + $app.cartCount"}});
const totalCol = I(cartBar, {type:"frame", name:"cart-total-col", layout:"vertical", gap:2});
const totRow = I(totalCol, {type:"frame", name:"cart-total-row", layout:"horizontal", gap:4, alignItems:"end"});
I(totRow, {type:"text", name:"cart-total", content:"¥0", fontSize:19, fontWeight:700, fill:[{type:"solid", color:"#FFFFFF"}], width:"fit_content", textGrowth:"auto", bindings:{content:"\"¥\" + $app.cartCount * 28"}});
I(totalCol, {type:"text", name:"cart-fee-hint", content:"另需配送费¥3", fontSize:11, fontWeight:400, fill:[{type:"solid", color:"#94A3B8"}], width:"fit_content", textGrowth:"auto"});
const checkoutBtn = I(cartBar, {type:"frame", name:"checkout-button", width:112, height:44, cornerRadius:22,
  fill:[{type:"solid", color:"#DC2626"}], layout:"horizontal", alignItems:"center", justifyContent:"center",
  events:{onTap:[{if:{expr:"$app.cartCount > 0", then:[{toast:"去结算"}], else:[{toast:"还差¥28起送，先加点好吃的吧"}]}}]}});
I(checkoutBtn, {type:"text", name:"checkout-button-text", content:"去结算", fontSize:16, fontWeight:600, fill:[{type:"solid", color:"#FFFFFF"}], width:"fit_content", textGrowth:"auto"});
