const sec = I(null, {type:"frame", name:"用户评价", width:"fill_container", height:"fit_content", layout:"vertical", padding:[0,24], gap:16});

const head = I(sec, {type:"frame", name:"reviews-header", layout:"horizontal", width:"fill_container", justifyContent:"space_between", alignItems:"center"});
I(head, {type:"text", content:"用户评价 (1,236)", fontFamily:"SF Pro", fontSize:17, fontWeight:600, fill:[{type:"solid",color:"#111827"}], letterSpacing:-0.3});
const more = I(head, {type:"frame", name:"view-all", layout:"horizontal", gap:2, alignItems:"center"});
I(more, {type:"text", content:"查看全部", fontFamily:"SF Pro", fontSize:13, fontWeight:500, fill:[{type:"solid",color:"#2563EB"}]});
I(more, {type:"icon_font", iconFontName:"chevron-right", iconFontFamily:"lucide", width:14, height:14, fill:[{type:"solid",color:"#2563EB"}]});

const reviews = [
  {name:"小鹿哦", stars:5, text:"面料很软，透气不闷汗", tag:"云白 / L", photo:true},
  {name:"Ming_2023", stars:5, text:"版型正，洗后没缩水", tag:"雾灰 / M", photo:false},
  {name:"山与海", stars:4, text:"颜色和图片基本一致", tag:"云白 / S", photo:false}
];

let first = true;
for (const r of reviews) {
  const card = I(sec, {type:"frame", name:"review-card", layout:"vertical", width:"fill_container", padding:[14,16], gap:8, cornerRadius:12, fill:[{type:"solid",color:"#F9FAFB"}], stroke:{thickness:1, fill:[{type:"solid",color:"#F3F4F6"}]}});
  const top = I(card, {type:"frame", layout:"horizontal", width:"fill_container", gap:10, alignItems:"center"});
  I(top, {type:"ellipse", width:32, height:32, fill:[{type:"solid",color:"#DBEAFE"}]});
  const who = I(top, {type:"frame", layout:"vertical", width:"fill_container", gap:2});
  I(who, {type:"text", content:r.name, fontFamily:"SF Pro", fontSize:14, fontWeight:500, fill:[{type:"solid",color:"#111827"}]});
  const starsRow = I(who, {type:"frame", layout:"horizontal", gap:2});
  for (let s = 0; s < 5; s++) {
    I(starsRow, {type:"icon_font", iconFontName:"star", iconFontFamily:"lucide", width:12, height:12, fill:[{type:"solid",color: s < r.stars ? "#F59E0B" : "#E5E7EB"}]});
  }
  I(top, {type:"rectangle", name:"spec-tag", width:70, height:22, cornerRadius:8, fill:[{type:"solid",color:"#FFFFFF"}], stroke:{thickness:1, fill:[{type:"solid",color:"#E5E7EB"}]}});
  I(top.children ? top : top, {type:"text"});
  const tagWrap = I(top, {type:"frame", layout:"horizontal", alignItems:"center", justifyContent:"center", width:70, height:22, cornerRadius:8, fill:[{type:"solid",color:"#FFFFFF"}], stroke:{thickness:1, fill:[{type:"solid",color:"#E5E7EB"}]}});
  I(tagWrap, {type:"text", content:r.tag, fontFamily:"SF Pro", fontSize:11, fontWeight:400, fill:[{type:"solid",color:"#6B7280"}], textAlign:"center"});

  I(card, {type:"text", content:r.text, fontFamily:"SF Pro", fontSize:14, fontWeight:400, fill:[{type:"solid",color:"#374151"}], width:"fill_container", lineHeight:1.5});

  if (r.photo) {
    I(card, {type:"image", name:"review-photo", width:96, height:96, cornerRadius:8, imageSearchQuery:"white cotton t-shirt", imagePrompt:"白色纯棉圆领 T 恤平铺摄影，柔和自然光，简洁浅色背景，商品特写", clipContent:true});
  }
  first = false;
}
