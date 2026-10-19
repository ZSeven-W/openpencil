```js
const sec = I(null, {type:"frame", name:"首页-商家列表", width:"fill_container", height:"fit_content", layout:"vertical", padding:[0,24,24,24], gap:16});

const head = I(sec, {type:"frame", name:"Section Header", layout:"vertical", width:"fill_container", height:"fit_content", gap:12});
I(head, {type:"text", name:"Section Title", content:"附近商家", fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:20, fontWeight:700, letterSpacing:-0.3, fill:[{type:"solid", color:"#21140F"}]});

const sortRow = I(head, {type:"frame", name:"Filter Row", layout:"horizontal", width:"fill_container", height:"fit_content", gap:8});
const filterFrame = I(sortRow, {type:"frame", name:"Filter Chip 综合排序", layout:"horizontal", alignItems:"center", justifyContent:"center", padding:[8,14], cornerRadius:8, fill:[{type:"solid", color:"#FF5A1F14"}], stroke:{thickness:1, fill:[{type:"solid", color:"#FF5A1F"}]}, events:{onTap:[{set:{"$app.sortBy":"'综合排序'"}}]}});
I(filterFrame, {type:"text", name:"Filter Label 综合排序", content:"综合排序", fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:13, fontWeight:500, fill:[{type:"solid", color:"#FF5A1F"}]});
I(filterFrame, {type:"icon_font", name:"Filter Caret", iconFontName:"chevron-down", iconFontFamily:"lucide", width:14, height:14, fill:[{type:"solid", color:"#FF5A1F"}]});
for (const label of ["销量","距离"]) {
  const chip = I(sortRow, {type:"frame", name:"Filter "+label, layout:"horizontal", alignItems:"center", justifyContent:"center", padding:[8,14], cornerRadius:8, fill:[{type:"solid", color:"#FFFFFF"}], stroke:{thickness:1, fill:[{type:"solid", color:"#EAD8C8"}]}, events:{onTap:[{set:{"$app.sortBy":JSON.stringify(label)}}]}});
  I(chip, {type:"text", name:"Filter Label "+label, content:label, fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:13, fontWeight:500, fill:[{type:"solid", color:"#7A5B48"}]});
}

const list = I(sec, {type:"frame", name:"Store List", layout:"vertical", width:"fill_container", height:"fit_content", gap:12});

const stores = [
  {id:"store-1", name:"老王记烧腊饭堂", q:"roast meat rice dish", p:"Cantonese roast pork and char siu over rice in a ceramic bowl, overhead warm natural light, appetizing food photography", tags:["免配送费","30分钟"], first:true},
  {id:"store-2", name:"川香麻辣香锅", q:"spicy mala hotpot", p:"Sichuan mala stir-fry hotpot with chili peppers and mixed ingredients in a pan, warm tones, appetizing food photography", tags:[], first:false},
  {id:"store-3", name:"面屋一灯拉面", q:"japanese ramen bowl", p:"Japanese tonkotsu ramen with chashu and soft egg in a ceramic bowl, warm side light, appetizing food photography", tags:[], first:false},
  {id:"store-4", name:"轻食沙拉实验室", q:"fresh salad bowl", p:"Fresh vegetable salad bowl with avocado and greens, bright daylight, clean appetizing food photography", tags:[], first:false},
  {id:"store-5", name:"粤味砂锅粥铺", q:"claypot congee", p:"Cantonese claypot congee rice porridge in a bubbling clay pot, warm cozy light, appetizing food photography", tags:[], first:false}
];

let i = 0;
for (const s of stores) {
  const delay = i * 60; i++;
  const card = I(list, {type:"frame", name:"Store Card "+s.name, layout:"horizontal", width:"fill_container", height:"fit_content", gap:12, padding:12, cornerRadius:14, fill:[{type:"solid", color:"#FFFFFF"}], stroke:{thickness:1, fill:[{type:"solid", color:"#F3E7DC"}]}, clipContent:true,
    events:{onTap:[{set:{"$app.storeId":JSON.stringify(s.id)}},{push:"\"/screen-1\""}]}},
    );
  const imgWrap = I(card, {type:"frame", name:"Store Photo Wrap "+s.id, width:80, height:80, cornerRadius:10, clipContent:true, layout:"none"});
  I(imgWrap, {type:"image", name:"Store Photo "+s.id, width:80, height:80, cornerRadius:10, imageSearchQuery:s.q, imagePrompt:s.p});
  const info = I(card, {type:"frame", name:"Store Info "+s.id, layout:"vertical", width:"fill_container", height:"fit_content", gap:6});
  I(info, {type:"text", name:"Store Name "+s.id, content:s.name, fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:16, fontWeight:600, letterSpacing:-0.3, fill:[{type:"solid", color:"#21140F"}]});
  if (s.tags.length > 0) {
    const tagRow = I(info, {type:"frame", name:"Store Tags "+s.id, layout:"horizontal", width:"fit_content", height:"fit_content", gap:6});
    for (const t of s.tags) {
      const tag = I(tagRow, {type:"frame", name:"Tag "+t, layout:"horizontal", alignItems:"center", justifyContent:"center", padding:[3,8], cornerRadius:4, fill:[{type:"solid", color:"#FFF0E3"}]});
      I(tag, {type:"text", name:"Tag Label "+t, content:t, fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:11, fontWeight:500, fill:[{type:"solid", color:"#FF5A1F"}]});
    }
  } else {
    I(info, {type:"text", name:"Store CTA "+s.id, content:"进入商家", fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:12, fontWeight:500, fill:[{type:"solid", color:"#A8917F"}]});
  }
  const metaRow = I(info, {type:"frame", name:"Store Meta "+s.id, layout:"horizontal", width:"fill_container", height:"fit_content", gap:4, alignItems:"center"});
  I(metaRow, {type:"text", name:"Store Meta Text "+s.id, content:"查看菜单", fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:12, fontWeight:400, fill:[{type:"solid", color:"#A8917F"}]});
  I(metaRow, {type:"icon_font", name:"Store Chevron "+s.id, iconFontName:"chevron-right", iconFontFamily:"lucide", width:14, height:14, fill:[{type:"solid", color:"#D7C6B8"}]});
}
```