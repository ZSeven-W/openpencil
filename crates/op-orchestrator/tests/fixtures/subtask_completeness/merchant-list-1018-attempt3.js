const sec = I(null, {type:"frame", name:"商家列表", width:"fill_container", height:"fit_content", layout:"vertical", padding:[0,24], gap:14});

const head = I(sec, {type:"frame", name:"list-header", layout:"horizontal", width:"fill_container", justifyContent:"space_between", alignItems:"center"});
I(head, {type:"text", name:"list-title", content:"附近商家", fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:20, fontWeight:700, fill:[{type:"solid", color:"#21140F"}], letterSpacing:-0.3});
const tabsRow = I(head, {type:"frame", name:"filter-tabs", layout:"horizontal", gap:6, alignItems:"center"});
const tabs = [{label:"综合排序", active:true},{label:"销量", active:false},{label:"距离", active:false}];
for (const t of tabs) {
  const chip = I(tabsRow, {type:"frame", name:"filter-tab", layout:"horizontal", padding:[6,10], cornerRadius:8, alignItems:"center", fill: t.active ? [{type:"solid", color:"#FF5A1F14"}] : [{type:"solid", color:"#FFFFFF"}], stroke:{thickness:1, fill:[{type:"solid", color: t.active ? "#FF5A1F" : "#F3E7DC"}]}});
  I(chip, {type:"text", content:t.label, fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:12, fontWeight: t.active ? 600 : 500, fill:[{type:"solid", color: t.active ? "#FF5A1F" : "#7A5B48"}]});
}

const stores = [
  {name:"老陈家 · 热汤面", img:"noodle soup bowl", prompt:"A steaming bowl of hot noodle soup with scallions in warm light, top-down food photography on a wooden table", sales:"月售 1200+", time:"30分钟", fee:"起送 ¥15 · 配送 ¥3", tags:["满25减8","新客立减"]},
  {name:"咔滋炸鸡工坊", img:"fried chicken plate", prompt:"Golden crispy fried chicken pieces piled on a plate with warm studio lighting, appetizing close-up food photography", sales:"月售 860+", time:"35分钟", fee:"起送 ¥20 · 配送 ¥2", tags:["满30减6","折扣菜"]},
  {name:"鲜禾轻食盖饭", img:"rice bowl meal", prompt:"A colorful rice bowl with grilled salmon and vegetables in warm natural light, clean food photography on ceramic plate", sales:"月售 640+", time:"28分钟", fee:"起送 ¥18 · 配送 ¥3", tags:["低卡轻食"]},
  {name:"甜时奶茶铺", img:"milk tea cups", prompt:"Two cups of creamy milk tea with tapioca pearls on a wooden counter, warm cozy beverage photography", sales:"月售 990+", time:"25分钟", fee:"起送 ¥12 · 免配送费", tags:["第二杯半价"]},
  {name:"深夜食堂小馆", img:"late night ramen", prompt:"A cozy bowl of late-night ramen with soft egg under moody warm restaurant lighting, steam rising, food photography", sales:"月售 430+", time:"40分钟", fee:"起送 ¥25 · 配送 ¥4", tags:["夜宵特惠"]}
];

let delay = 0;
for (const s of stores) {
  const card = I(sec, {type:"frame", name:"store-card", layout:"horizontal", width:"fill_container", padding:14, gap:12, cornerRadius:14, fill:[{type:"solid", color:"#FFFFFF"}], stroke:{thickness:1, fill:[{type:"solid", color:"#F3E7DC"}]}, effects:[{type:"shadow", offsetX:0, offsetY:2, blur:8, spread:0, color:"#21140F0D"}],
    animations:[{trigger:"inView", durationMs:400, delayMs:delay, easing:"emphasizedDecelerate", keyframes:[{offset:0, values:{opacity:0, translateY:14}},{offset:1, values:{opacity:1, translateY:0}}]}],
    events:{onTap:[{push:"\"/screen-1\""}]}});
  delay += 60;
  I(card, {type:"image", name:"store-photo", width:88, height:88, cornerRadius:10, imageSearchQuery:s.img, imagePrompt:s.prompt});
  const info = I(card, {type:"frame", name:"store-info", layout:"vertical", width:"fill_container", gap:5});
  I(info, {type:"text", content:s.name, fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:15, fontWeight:600, fill:[{type:"solid", color:"#21140F"}], letterSpacing:-0.2});
  I(info, {type:"text", content:s.sales + " · " + s.time, fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:12, fontWeight:400, fill:[{type:"solid", color:"#7A5B48"}]});
  I(info, {type:"text", content:s.fee, fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:12, fontWeight:400, fill:[{type:"solid", color:"#A8917F"}]});
  const tagRow = I(info, {type:"frame", name:"store-tags", layout:"horizontal", gap:6});
  for (const tg of s.tags) {
    const badge = I(tagRow, {type:"frame", layout:"horizontal", padding:[3,6], cornerRadius:6, fill:[{type:"solid", color:"#FFF0E3"}]});
    I(badge, {type:"text", content:tg, fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:10, fontWeight:600, fill:[{type:"solid", color:"#FF5A1F"}]});
  }
}
