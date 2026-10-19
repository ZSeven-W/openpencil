const sec = I(null, {type:"frame", name:"首页-商家列表", width:"fill_container", height:"fit_content", layout:"vertical", padding:[0,24], gap:14});

const head = I(sec, {type:"frame", name:"列表头部", layout:"vertical", width:"fill_container", gap:10});
I(head, {type:"text", name:"附近商家标题", content:"附近商家", fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:18, fontWeight:700, fill:"#21140F", letterSpacing:-0.3});

const filterRow = I(head, {type:"frame", name:"筛选行", layout:"horizontal", width:"fill_container", gap:8});
for (const [i, f] of [["综合排序", true],["销量", false],["距离", false]].entries()) {
  const chip = I(filterRow, {type:"frame", name:"筛选-" + f[0], layout:"horizontal", height:28, padding:[0,12], cornerRadius:8, alignItems:"center", fill: f[1] ? "#FF5A1F14" : "transparent", stroke: f[1] ? {thickness:0} : {thickness:1, fill:[{type:"solid", color:"#F3E7DC"}]}});
  I(chip, {type:"text", name:"筛选文字-" + f[0], content:f[0], fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:12, fontWeight:500, fill: f[1] ? "#FF5A1F" : "#7A5B48"});
  if (i === 0) I(chip, {type:"icon_font", name:"排序箭头", iconFontFamily:"lucide", iconFontName:"chevron-down", width:14, height:14, fill:"#FF5A1F"});
}

const list = I(sec, {type:"frame", name:"商家卡片列表", layout:"vertical", width:"fill_container", gap:12});

const stores = [
  {id:"store-1", name:"老王记烧腊饭堂", sales:"月售 999+", min:"¥20 起送", time:"30分钟 · 1.2km", fee:"免配送费", q:"roast pork rice", p:"广式烧腊快餐门店招牌叉烧饭特写，暖光美食摄影，俯拍浅景深"},
  {id:"store-2", name:"川香麻辣香锅", sales:"月售 680+", min:"¥25 起送", time:"35分钟 · 0.8km", fee:"配送费 ¥3", q:"spicy hot pot dish", p:"麻辣香锅特写，红油辣椒配藕片虾滑，暖色美食摄影，浅景深"},
  {id:"store-3", name:"面屋一灯拉面", sales:"月售 520+", min:"¥30 起送", time:"40分钟 · 1.6km", fee:"配送费 ¥4", q:"japanese ramen bowl", p:"日式豚骨拉面碗，溏心蛋叉烧葱花特写，暖光美食摄影，俯拍"},
  {id:"store-4", name:"轻食沙拉实验室", sales:"月售 410+", min:"¥35 起送", time:"32分钟 · 2.1km", fee:"免配送费", q:"fresh salad bowl", p:"牛油果鸡胸轻食沙拉碗，明亮新鲜美食摄影，俯拍浅景深"},
  {id:"store-5", name:"粤味砂锅粥铺", sales:"月售 760+", min:"¥22 起送", time:"28分钟 · 1.0km", fee:"配送费 ¥2.5", q:"claypot congee", p:"砂锅海鲜粥特写，热气腾腾的广式砂锅粥，暖光美食摄影"}
];

stores.forEach((s, i) => {
  const card = I(list, {type:"frame", name:"商家卡-" + s.name, layout:"horizontal", width:"fill_container", padding:12, gap:12, cornerRadius:14, fill:"#FFFFFF", stroke:{thickness:1, fill:[{type:"solid", color:"#F3E7DC"}]}, effects:[{type:"shadow", offsetX:0, offsetY:2, blur:8, spread:0, color:"#21140F0A"}], alignItems:"center",
    animations:[{trigger:"inView", durationMs:400, easing:"emphasizedDecelerate", delayMs:i*60, keyframes:[{offset:0, values:{opacity:0, translateY:16}}, {offset:1, values:{opacity:1, translateY:0}}]}],
    events:{onTap:[{set:{"$app.selectedStore":JSON.stringify(s.name)}}, {push:"\"/screen-1\""}]}});
  I(card, {type:"image", name:"门店图-" + s.name, width:72, height:72, cornerRadius:10, imageSearchQuery:s.q, imagePrompt:s.p});
  const info = I(card, {type:"frame", name:"商家信息-" + s.name, layout:"vertical", width:"fill_container", gap:5});
  I(info, {type:"text", name:"店名-" + s.name, content:s.name, fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:16, fontWeight:600, fill:"#21140F", width:"fill_container"});
  const meta1 = I(info, {type:"frame", name:"销售行-" + s.name, layout:"horizontal", gap:6, alignItems:"center"});
  I(meta1, {type:"icon_font", name:"销量图标-" + s.name, iconFontFamily:"lucide", iconFontName:"flame", width:14, height:14, fill:"#F59E0B"});
  I(meta1, {type:"text", name:"销量-" + s.name, content:s.sales, fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:12, fontWeight:400, fill:"#7A5B48"});
  if (s.id === "store-1") {
    const badge = I(meta1, {type:"frame", name:"免配送费标签", layout:"horizontal", height:18, padding:[0,6], cornerRadius:8, alignItems:"center", fill:"#FF5A1F14"});
    I(badge, {type:"text", name:"免配送费文字", content:"免配送费", fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:10, fontWeight:600, fill:"#FF5A1F"});
  }
  I(info, {type:"text", name:"配送信息-" + s.name, content:s.min + " · " + s.time, fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:12, fontWeight:400, fill:"#A8917F", width:"fill_container"});
  if (s.id !== "store-1") {
    const feeRow = I(info, {type:"frame", name:"配送费行-" + s.name, layout:"horizontal", gap:6, alignItems:"center"});
    const fb = I(feeRow, {type:"frame", name:"配送费标签-" + s.name, layout:"horizontal", height:18, padding:[0,6], cornerRadius:8, alignItems:"center", fill:"#FFF0E3"});
    I(fb, {type:"text", name:"配送费文字-" + s.name, content:s.fee, fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:10, fontWeight:500, fill:"#7A5B48"});
  }
  I(card, {type:"icon_font", name:"进入箭头-" + s.name, iconFontFamily:"lucide", iconFontName:"chevron-right", width:18, height:18, fill:"#D7C6B8"});
});