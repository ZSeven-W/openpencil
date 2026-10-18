const sec = I(null, {type:"frame", name:"商家详情", width:"fill_container", height:"fit_content", layout:"vertical", fill:[]});

// ---------- Full-bleed hero ----------
const hero = I(sec, {type:"frame", name:"Store Hero", width:"fill_container", height:220, layout:"none", clipContent:true});
I(hero, {type:"image", name:"Store Hero Photo", width:375, height:220,
  imageSearchQuery:"noodle restaurant storefront",
  imagePrompt:"A cozy Chinese noodle restaurant interior with warm evening lighting, steaming bowls visible on a counter, inviting and appetizing atmosphere"});
I(hero, {type:"rectangle", name:"Hero Scrim", x:0, y:120, width:375, height:100,
  fill:[{type:"linear_gradient", angle:180, stops:[{offset:0, color:"#00000000"},{offset:1, color:"#0F172A99"}]}]});
// floating controls (topmost)
const backBtn = I(hero, {type:"frame", name:"Back Button", x:16, y:16, width:36, height:36, cornerRadius:18, layout:"horizontal", alignItems:"center", justifyContent:"center", fill:[{type:"solid", color:"#FFFFFFCC"}],
  events:{onTap:[{pop:null}]}});
I(backBtn, {type:"icon_font", iconFontName:"chevron-left", iconFontFamily:"lucide", width:20, height:20, fill:[{type:"solid", color:"#0F172A"}]});
const favBtn = I(hero, {type:"frame", name:"Favorite Button", x:323, y:16, width:36, height:36, cornerRadius:18, layout:"horizontal", alignItems:"center", justifyContent:"center", fill:[{type:"solid", color:"#FFFFFFCC"}]});
I(favBtn, {type:"icon_font", iconFontName:"heart", iconFontFamily:"lucide", width:18, height:18, fill:[{type:"solid", color:"#EF4444"}]});
// store logo overlapping bottom edge
I(hero, {type:"frame", name:"Store Logo", x:24, y:180, width:56, height:56, cornerRadius:12, layout:"horizontal", alignItems:"center", justifyContent:"center",
  fill:[{type:"solid", color:"#F59E0B"}], stroke:{thickness:2, fill:[{type:"solid", color:"#FFFFFF"}]}});
I(hero.children === undefined ? hero : hero, {type:"text", x:24, y:180, width:56, height:56, content:"面", fontFamily:"Noto Sans SC", fontSize:24, fontWeight:700, fill:[{type:"solid", color:"#FFFFFF"}], textAlign:"center"});

// ---------- Info block ----------
const info = I(sec, {type:"frame", name:"Store Info", width:"fill_container", height:"fit_content", layout:"vertical", padding:[20,24,0,24], gap:6});
I(info, {type:"frame", name:"Info Spacer Left", width:"fill_container", height:0, layout:"vertical", fill:[], opacity:0});
const nameRow = I(info, {type:"frame", name:"Store Name Row", width:"fill_container", height:"fit_content", layout:"horizontal", alignItems:"center", gap:8, padding:[0,0,0,0]});
I(nameRow, {type:"text", name:"Store Name", content:"老张手擀面 · 江南西店", width:"fit_content", textGrowth:"auto", fontFamily:"Noto Sans SC", fontSize:21, fontWeight:700, lineHeight:1.3, fill:[{type:"solid", color:"#0F172A"}]});
I(nameRow, {type:"icon_font", iconFontName:"chevron-right", iconFontFamily:"lucide", width:18, height:18, fill:[{type:"solid", color:"#94A3B8"}]});
I(info, {type:"text", name:"Store Meta", content:"★ 4.8 · 月售 3200+ · 30分钟 · 1.2km", width:"fill_container", textGrowth:"fixed-width", fontFamily:"Inter", fontSize:13, lineHeight:1.4, fill:[{type:"solid", color:"#475569"}]});
const tagRow = I(info, {type:"frame", name:"Promo Tags", width:"fill_container", height:"fit_content", layout:"horizontal", gap:8, padding:[4,0,0,0]});
for (const t of ["满30减12","新客立减5"]) {
  const tag = I(tagRow, {type:"frame", name:"Promo Tag", height:22, cornerRadius:4, layout:"horizontal", alignItems:"center", padding:[0,8], fill:[{type:"solid", color:"#FEF2F2"}]});
  I(tag, {type:"text", content:t, width:"fit_content", textGrowth:"auto", fontFamily:"Noto Sans SC", fontSize:12, fontWeight:500, fill:[{type:"solid", color:"#EF4444"}]});
}
I(info, {type:"text", name:"Announcement", content:"公告：本店招牌牛肉面每日限量 80 碗，售完即止", width:"fill_container", textGrowth:"fixed-width", fontFamily:"Noto Sans SC", fontSize:12, lineHeight:1.5, fill:[{type:"solid", color:"#94A3B8"}], padding:[4,0,0,0]});

// ---------- Category tabs (horizontal scroll) ----------
const catSec = I(sec, {type:"frame", name:"Menu Categories", width:"fill_container", height:"fit_content", layout:"vertical", gap:12, padding:[20,0,0,0]});
const catViewport = I(catSec, {type:"frame", name:"Category Scroll Viewport", width:"fill_container", height:"fit_content", layout:"horizontal", clipContent:true, padding:[0,0,0,24]});
const catRow = I(catViewport, {type:"frame", name:"Category Chip Row", width:"fit_content", height:"fit_content", layout:"horizontal", gap:12});
const cats = [{n:"招牌推荐",a:true},{n:"手工面类",a:false},{n:"卤味小食",a:false},{n:"清爽凉菜",a:false},{n:"饮品甜汤",a:false}];
for (const c of cats) {
  const chip = I(catRow, {type:"frame", name:"Category Chip", height:36, cornerRadius:18, layout:"horizontal", alignItems:"center", padding:[0,16],
    fill: c.a ? [{type:"solid", color:"#0F172A"}] : [{type:"solid", color:"#FFFFFF"}],
    stroke: c.a ? {thickness:1, fill:[{type:"solid", color:"#0F172A"}]} : {thickness:1, fill:[{type:"solid", color:"#E2E8F0"}]});
  I(chip, {type:"text", content:c.n, width:"fit_content", textGrowth:"auto", fontFamily:"Noto Sans SC", fontSize:14, fontWeight:c.a?600:500, fill:[{type:"solid", color:c.a?"#FFFFFF":"#475569"}]});
}

// ---------- Dish list: 6 rows ----------
const listSec = I(sec, {type:"frame", name:"Dish List", width:"fill_container", height:"fit_content", layout:"vertical", padding:[16,24,0,24], gap:0});
const dishes = [
  {n:"招牌牛肉手擀面", d:"牛骨浓汤熬制 6 小时，配卤蛋与青菜", p:26, o:32, s:"月售 1260", q:"beef noodle bowl"},
  {n:"酸汤肥牛面", d:"金汤酸辣开胃，肥牛鲜嫩大片", p:24, o:null, s:"月售 980", q:"sour soup noodles"},
  {n:"葱油拌面", d:"小碗精拌，葱油现熬喷香", p:15, o:null, s:"月售 866", q:"scallion oil noodles"},
  {n:"秘制卤鸡腿", d:"老卤慢煨，入口脱骨", p:8, o:null, s:"月售 720", q:"braised chicken leg"},
  {n:"凉拌黄瓜", d:"蒜末香醋现拌，清脆爽口", p:6, o:null, s:"月售 640", q:"smashed cucumber salad"},
  {n:"冰镇酸梅汤", d:"古法熬煮，饭后解腻", p:5, o:null, s:"月售 1500", q:"iced plum drink"}
];
for (const [i, d] of dishes.entries()) {
  const row = I(listSec, {type:"frame", name:"Dish Row " + (i+1), width:"fill_container", height:"fit_content", layout:"horizontal", gap:12, alignItems:"start", padding:[12,0],
    stroke: i < 5 ? {thickness:{bottom:1}, fill:[{type:"solid", color:"#F1F5F9"}]} : {thickness:0, fill:[]},
    animations:[{trigger:"inView", durationMs:400, delayMs: i*60, easing:"emphasizedDecelerate", keyframes:[{offset:0, values:{opacity:0, translateY:16}},{offset:1, values:{opacity:1, translateY:0}}]}]});
  I(row, {type:"image", name:"Dish Photo " + (i+1), width:92, height:92, cornerRadius:10,
    imageSearchQuery: d.q + " dish", imagePrompt: d.n + " — appetizing overhead food photography of " + d.q + ", warm natural light, served on a simple ceramic dish"});
  const mid = I(row, {type:"frame", name:"Dish Info " + (i+1), width:"fill_container", height:"fit_content", layout:"vertical", gap:4});
  I(mid, {type:"text", content:d.n, width:"fill_container", textGrowth:"fixed-width", fontFamily:"Noto Sans SC", fontSize:15, fontWeight:600, lineHeight:1.35, fill:[{type:"solid", color:"#0F172A"}]});
  I(mid, {type:"text", content:d.d, width:"fill_container", textGrowth:"fixed-width", fontFamily:"Noto Sans SC", fontSize:12, lineHeight:1.45, fill:[{type:"solid", color:"#94A3B8"}]});
  I(mid, {type:"text", content:d.s, width:"fill_container", textGrowth:"fixed-width", fontFamily:"Inter", fontSize:12, lineHeight:1.4, fill:[{type:"solid", color:"#94A3B8"}]});
  const priceRow = I(mid, {type:"frame", name:"Price Row " + (i+1), width:"fill_container", height:"fit_content", layout:"horizontal", alignItems:"end", gap:6, padding:[2,0,0,0]});
  if (d.o) I(priceRow, {type:"text", content:"¥" + d.o, width:"fit_content", textGrowth:"auto", fontFamily:"Inter", fontSize:12, fill:[{type:"solid", color:"#94A3B8"}]});
  I(priceRow, {type:"text", content:"¥" + d.p, width:"fit_content", textGrowth:"auto", fontFamily:"Inter", fontSize:18, fontWeight:700, fill:[{type:"solid", color:"#0F172A"}]});
  const addWrap = I(row, {type:"frame", name:"Add Button Slot " + (i+1), width:44, height:44, layout:"horizontal", alignItems:"end", justifyContent:"end"});
  const addBtn = I(addWrap, {type:"frame", name:"Add Button " + (i+1), width:30, height:30, cornerRadius:15, layout:"horizontal", alignItems:"center", justifyContent:"center",
    fill:[{type:"solid", color:"#F59E0B"}],
    animations:[{trigger:"press", durationMs:150, easing:"standard", keyframes:[{offset:0, values:{scaleX:1, scaleY:1}},{offset:0.5, values:{scaleX:0.88, scaleY:0.88}},{offset:1, values:{scaleX:1, scaleY:1}}]}]});
  I(addBtn, {type:"icon_font", iconFontName:"plus", iconFontFamily:"lucide", width:16, height:16, fill:[{type:"solid", color:"#FFFFFF"}]});
}

// ---------- Cart bar (floating pill) ----------
const cartSec = I(sec, {type:"frame", name:"Cart Bar Section", width:"fill_container", height:96, layout:"none", padding:0});
const bar = I(cartSec, {type:"frame", name:"Cart Bar", x:12, y:20, width:351, height:56, layout:"none"});
I(bar, {type:"rectangle", name:"Cart Pill", x:24, y:6, width:327, height:50, cornerRadius:25, fill:[{type:"solid", color:"#0F172A"}]});
const disc = I(bar, {type:"frame", name:"Cart Disc", x:4, y:4, width:48, height:48, cornerRadius:24, layout:"horizontal", alignItems:"center", justifyContent:"center", fill:[{type:"solid", color:"#F59E0B"}], stroke:{thickness:4, fill:[{type:"solid", color:"#FFFFFF"}]}});
I(disc, {type:"icon_font", iconFontName:"shopping-cart", iconFontFamily:"lucide", width:20, height:20, fill:[{type:"solid", color:"#FFFFFF"}]});
const badge = I(bar, {type:"frame", name:"Cart Count Badge", x:40, y:0, width:18, height:18, cornerRadius:9, layout:"horizontal", alignItems:"center", justifyContent:"center", fill:[{type:"solid", color:"#EF4444"}]});
I(badge, {type:"text", content:"2", width:"fit_content", textGrowth:"auto", fontFamily:"Inter", fontSize:11, fontWeight:600, fill:[{type:"solid", color:"#FFFFFF"}]});
I(bar, {type:"text", name:"Cart Total", x:64, y:0, width:120, height:56, content:"¥34", fontFamily:"Inter", fontSize:19, fontWeight:700, fill:[{type:"solid", color:"#FFFFFF"}], textAlignVertical:"middle"});
I(bar, {type:"text", name:"Delivery Fee Note", x:64, y:38, width:150, height:16, content:"另需配送费 ¥3", fontFamily:"Noto Sans SC", fontSize:11, fill:[{type:"solid", color:"#94A3B8"}]});
const checkout = I(bar, {type:"frame", name:"Checkout CTA", x:247, y:10, width:96, height:42, cornerRadius:21, layout:"horizontal", alignItems:"center", justifyContent:"center", fill:[{type:"solid", color:"#F59E0B"}]});
I(checkout, {type:"text", content:"去结算", width:"fit_content", textGrowth:"auto", fontFamily:"Noto Sans SC", fontSize:15, fontWeight:600, fill:[{type:"solid", color:"#FFFFFF"}]});
