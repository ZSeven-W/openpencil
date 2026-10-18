const sec = I(null, {type:"frame", name:"商品大图轮播区", width:"fill_container", height:"fit_content", layout:"vertical", clipContent:true});

const viewport = I(sec, {type:"frame", name:"轮播视口", width:"fill_container", height:340, layout:"none", clipContent:true});

const strip = I(viewport, {type:"frame", name:"轮播条", x:0, y:0, width:1875, height:340, layout:"horizontal", gap:0, clipContent:false});

const slides = [
  {q:"white ceramic mug", p:"雾白陶瓷马克杯静置在浅色亚麻桌布上，柔和自然侧光，极简产品摄影，暖调背景"},
  {q:"ceramic mug top view", p:"雾白陶瓷马克杯俯拍特写，杯口圆润釉面细节，极简浅灰背景，柔和阴影产品摄影"},
  {q:"mug holding coffee", p:"雾白陶瓷马克杯盛着拿铁咖啡放在木桌上，温暖晨光，生活方式产品摄影"},
  {q:"mug in hands", p:"双手捧着雾白陶瓷马克杯，近景特写，温暖柔和光线，生活方式摄影"},
  {q:"ceramic mug detail", p:"雾白陶瓷马克杯侧面釉面纹理细节特写，浅暖灰背景，极简产品摄影"}
];
let i = 0;
for (const s of slides) {
  I(strip, {type:"image", name:"轮播图-" + (i+1), width:375, height:340, imageSearchQuery:s.q, imagePrompt:s.p, cornerRadius:0});
  i++;
}

const controls = I(viewport, {type:"frame", name:"悬浮控件", x:0, y:0, width:375, height:340, layout:"none"});

const backBtn = I(controls, {type:"frame", name:"返回按钮", x:16, y:16, width:40, height:40, cornerRadius:20, layout:"horizontal", alignItems:"center", justifyContent:"center", fill:[{type:"solid", color:"#FFFFFF66"}], stroke:{thickness:1, fill:[{type:"solid", color:"#FFFFFF99"}]}, events:{onTap:[{pop:null}]}});
I(backBtn, {type:"icon_font", name:"返回图标", iconFontName:"chevron-left", iconFontFamily:"lucide", width:20, height:20, fill:[{type:"solid", color:"#1C1917"}]});

const favBtn = I(controls, {type:"frame", name:"收藏按钮", x:319, y:16, width:40, height:40, cornerRadius:20, layout:"horizontal", alignItems:"center", justifyContent:"center", fill:[{type:"solid", color:"#FFFFFF66"}], stroke:{thickness:1, fill:[{type:"solid", color:"#FFFFFF99"}]}});
I(favBtn, {type:"icon_font", name:"收藏图标", iconFontName:"heart", iconFontFamily:"lucide", width:20, height:20, fill:[{type:"solid", color:"#1C1917"}]});

const dots = I(controls, {type:"frame", name:"圆点指示器", x:0, y:316, width:375, height:8, layout:"horizontal", gap:6, alignItems:"center", justifyContent:"center"});
for (let d = 0; d < 5; d++) {
  I(dots, {type:"ellipse", name:"指示点-" + (d+1), width: d===0 ? 16 : 6, height:6, fill:[{type:"solid", color: d===0 ? "#F97316" : "#FFFFFF99"}]});
}