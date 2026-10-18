const sec = I(null, {type:"frame", name:"用户评价", width:"fill_container", height:"fit_content", layout:"vertical", padding:[0,24], gap:16});

const head = I(sec, {type:"frame", name:"reviews-header", layout:"horizontal", width:"fill_container", height:"fit_content", alignItems:"center", justifyContent:"space_between"});
const titleWrap = I(head, {type:"frame", name:"reviews-title", layout:"horizontal", width:"fit_content", height:"fit_content", gap:6, alignItems:"center"});
I(titleWrap, {type:"text", name:"reviews-title-text", content:"用户评价 (1,236)", fontFamily:"SF Pro", fontSize:20, fontWeight:600, letterSpacing:-0.3, lineHeight:1.2, fill:[{type:"solid", color:"#111827"}], width:"fit_content", height:"fit_content", textGrowth:"auto"});
I(titleWrap, {type:"text", name:"rating-chip", content:"4.8", fontFamily:"SF Pro", fontSize:12, fontWeight:500, lineHeight:1.4, fill:[{type:"solid", color:"#2563EB"}], width:"fit_content", height:"fit_content", textGrowth:"auto"});
const seeAll = I(head, {type:"frame", name:"see-all-link", layout:"horizontal", width:"fit_content", height:"fit_content", gap:2, alignItems:"center", cornerRadius:8, events:{onTap:[{toast:"'查看全部评价'"}]}});
I(seeAll, {type:"text", name:"see-all-text", content:"查看全部", fontFamily:"SF Pro", fontSize:13, fontWeight:500, lineHeight:1.4, fill:[{type:"solid", color:"#6B7280"}], width:"fit_content", height:"fit_content", textGrowth:"auto"});
I(seeAll, {type:"icon_font", name:"see-all-icon", iconFontName:"chevron-right", iconFontFamily:"lucide", width:14, height:14, fill:[{type:"solid", color:"#9CA3AF"}]});

const list = I(sec, {type:"frame", name:"review-list", layout:"vertical", width:"fill_container", height:"fit_content", gap:0, fill:[{type:"solid", color:"#FFFFFF"}], stroke:{thickness:1, fill:[{type:"solid", color:"#F3F4F6"}]}, cornerRadius:14, clipContent:true});

const reviews = [
  {name:"林***溪", stars:5, text:"面料软糯透气，版型正，很满意", spec:"云白 / L", img:true},
  {name:"山**月", stars:5, text:"洗了几次没变形，值得回购", spec:"雾灰 / M", img:false},
  {name:"K***o", stars:4, text:"略微宽松，喜欢这种廓形", spec:"云白 / XL", img:false}
];

let first = true;
for (const r of reviews) {
  const row = I(list, {type:"frame", name:"review-item-" + r.name, layout:"vertical", width:"fill_container", height:"fit_content", padding:[14,16], gap:10, stroke:{thickness:{top: first ? 0 : 1, right:0, bottom:0, left:0}, fill:[{type:"solid", color:"#F3F4F6"}]}});
  const top = I(row, {type:"frame", name:"review-top", layout:"horizontal", width:"fill_container", height:"fit_content", gap:10, alignItems:"center"});
  const avatar = I(top, {type:"frame", name:"review-avatar", width:34, height:34, cornerRadius:17, layout:"horizontal", alignItems:"center", justifyContent:"center", fill:[{type:"solid", color:"#DBEAFE"}]});
  I(avatar, {type:"text", name:"avatar-letter", content:r.name.charAt(0), fontFamily:"SF Pro", fontSize:14, fontWeight:600, lineHeight:1.2, fill:[{type:"solid", color:"#2563EB"}], width:"fit_content", height:"fit_content", textGrowth:"auto"});
  const nameStars = I(top, {type:"frame", name:"name-stars", layout:"vertical", width:"fill_container", height:"fit_content", gap:3});
  I(nameStars, {type:"text", name:"reviewer-name", content:r.name, fontFamily:"SF Pro", fontSize:13, fontWeight:500, lineHeight:1.4, fill:[{type:"solid", color:"#111827"}], width:"fit_content", height:"fit_content", textGrowth:"auto"});
  const starsRow = I(nameStars, {type:"frame", name:"stars-row", layout:"horizontal", width:"fit_content", height:"fit_content", gap:2, alignItems:"center"});
  for (let s = 0; s < 5; s++) {
    I(starsRow, {type:"icon_font", name:"star-" + s, iconFontName:"star", iconFontFamily:"lucide", width:12, height:12, fill:[{type:"solid", color: s < r.stars ? "#F59E0B" : "#E5E7EB"}]});
  }
  I(top, {type:"text", name:"spec-tag", content:r.spec, fontFamily:"SF Pro", fontSize:11, fontWeight:500, lineHeight:1.4, fill:[{type:"solid", color:"#6B7280"}], width:"fit_content", height:"fit_content", textGrowth:"auto"});
  const body = I(row, {type:"frame", name:"review-body", layout:"horizontal", width:"fill_container", height:"fit_content", gap:12, alignItems:"start"});
  I(body, {type:"text", name:"review-text", content:r.text, fontFamily:"SF Pro", fontSize:15, fontWeight:400, lineHeight:1.5, fill:[{type:"solid", color:"#374151"}], width:"fill_container", height:"fit_content", textGrowth:"fixed-width"});
  if (r.img) {
    const thumb = I(body, {type:"frame", name:"review-thumb-wrap", width:56, height:56, cornerRadius:8, clipContent:true});
    I(thumb, {type:"image", name:"review-thumb", width:56, height:56, cornerRadius:8, imageSearchQuery:"white cotton t-shirt", imagePrompt:"白色纯棉圆领 T 恤平铺特写照片，柔和自然光，简洁浅色背景"});
  }
  first = false;
}

I(sec, {type:"rectangle", name:"reviews-end-rule", width:"fill_container", height:1, fill:[{type:"solid", color:"#FFFFFF00"}]});
