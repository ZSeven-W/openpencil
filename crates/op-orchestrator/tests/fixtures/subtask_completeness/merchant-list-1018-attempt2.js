const sec = I(null, {type:"frame", name:"商家列表", width:"fill_container", height:"fit_content", layout:"vertical", padding:[0,24,24,24], gap:14});

const head = I(sec, {type:"frame", name:"列表头", layout:"horizontal", width:"fill_container", height:"fit_content", justifyContent:"space_between", alignItems:"center"});
I(head, {type:"text", name:"区块标题", content:"附近商家", fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:18, fontWeight:700, fill:[{type:"solid", color:"#21140F"}], letterSpacing:-0.3, lineHeight:1.2});

const tabs = I(head, {type:"frame", name:"筛选行", layout:"horizontal", width:"fit_content", height:"fit_content", gap:8});
const tabData = [
  {label:"综合排序", active:true},
  {label:"销量", active:false},
  {label:"距离", active:false}
];
for (const t of tabData) {
  const chip = I(tabs, {type:"frame", name:"筛选-"+t.label, layout:"horizontal", width:"fit_content", height:32, padding:[0,12], alignItems:"center", justifyContent:"center", cornerRadius:8,
    fill: t.active ? [{type:"solid", color:"#FFF0E3"}] : [],
    stroke: t.active ? {thickness:0, fill:[]} : {thickness:1, fill:[{type:"solid", color:"#F3E7DC"}]}});
  I(chip, {type:"text", content:t.label, fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:12, fontWeight:500, fill:[{type:"solid", color: t.active ? "#FF5A1F" : "#7A5B48"}]});
}

const stores = [
  {name:"老陈家 · 热汤面", img:"noodle-soup-bowl", prompt:"Steaming hot noodle soup in a ceramic bowl with scallions and soft-boiled egg, warm side light, overhead crop, appetizing food photography",
    meta:"★ 4.8 · 月售 3200+", meta2:"30分钟 · 1.2km", cost:"起送 ¥20 · 配送 ¥3", tags:["满 25 减 8","新客立减 5"]},
  {name:"咔滋炸鸡工坊", img:"fried-chicken-basket", prompt:"Golden crispy fried chicken pieces in a paper basket with dipping sauce, warm light, close-up food photography",
    meta:"★ 4.7 · 月售 2100+", meta2:"35分钟 · 0.8km", cost:"起送 ¥25 · 配送 ¥4", tags:["满 30 减 10","第二份半价"]},
  {name:"川香麻辣拌", img:"spicy-noodle-bowl", prompt:"Sichuan spicy mixed noodles with chili oil and vegetables in a clay bowl, warm overhead food photography",
    meta:"★ 4.6 · 月售 1800+", meta2:"28分钟 · 1.5km", cost:"起送 ¥15 · 配送 ¥2", tags:["满 20 减 6"]},
  {name:"鲜果轻食沙拉", img:"fresh-salad-bowl", prompt:"Colorful fresh salad bowl with avocado, cherry tomatoes and greens, natural daylight, overhead food photography",
    meta:"★ 4.9 · 月售 960+", meta2:"32分钟 · 0.6km", cost:"起送 ¥30 · 配送 ¥5", tags:["低卡专区"]},
  {name:"稻香盖饭食堂", img:"rice-bowl-pork", prompt:"Japanese-style rice bowl topped with glazed pork and egg, warm restaurant light, close-up food photography",
    meta:"★ 4.8 · 月售 1500+", meta2:"26分钟 · 1.0km", cost:"起送 ¥18 · 配送 ¥3", tags:["午市特价","满 28 减 9"]}
];

const list = I(sec, {type:"frame", name:"商家卡片列", layout:"vertical", width:"fill_container", height:"fit_content", gap:14});

stores.forEach((s, idx) => {
  const card = I(list, {type:"frame", name:"商家卡-"+s.name, layout:"horizontal", width:"fill_container", height:"fit_content", gap:12, alignItems:"start", padding:12, cornerRadius:14,
    fill:[{type:"solid", color:"#FFFFFF"}], stroke:{thickness:1, fill:[{type:"solid", color:"#F3E7DC"}]}, clipContent:true,
    events:{onTap:[{push:"\"/screen-1\"\""}]},
    animations:[{trigger:"inView", durationMs:400, delayMs:idx*60, easing:"emphasizedDecelerate", keyframes:[{offset:0, values:{opacity:0, translateY:16}},{offset:1, values:{opacity:1, translateY:0}}]}]});

  I(card, {type:"image", name:"商家图-"+s.name, width:88, height:88, cornerRadius:12,
    imageSearchQuery:s.img, imagePrompt:s.prompt});

  const info = I(card, {type:"frame", name:"商家信息", layout:"vertical", width:"fill_container", height:"fit_content", gap:5});
  I(info, {type:"text", content:s.name, fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:16, fontWeight:600, fill:[{type:"solid", color:"#21140F"}], width:"fill_container", textGrowth:"fixed-width", lineHeight:1.2});
  I(info, {type:"text", content:s.meta + " · " + s.meta2, fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:12, fontWeight:400, fill:[{type:"solid", color:"#7A5B48"}], width:"fill_container", textGrowth:"fixed-width", lineHeight:1.4});
  I(info, {type:"text", content:s.cost, fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:11, fontWeight:400, fill:[{type:"solid", color:"#A8917F"}], width:"fill_container", textGrowth:"fixed-width", lineHeight:1.4});

  const tagRow = I(info, {type:"frame", name:"活动标签", layout:"horizontal", width:"fill_container", height:"fit_content", gap:6});
  s.tags.forEach((tg, i) => {
    const badge = I(tagRow, {type:"frame", name:"标签-"+tg, layout:"horizontal", width:"fit_content", height:20, padding:[0,7], alignItems:"center", justifyContent:"center", cornerRadius:4,
      fill:[{type:"solid", color:"#FFF0E3"}]});
    I(badge, {type:"text", content:tg, fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:10, fontWeight:500, fill:[{type:"solid", color:"#FF5A1F"}]});
  });
});
