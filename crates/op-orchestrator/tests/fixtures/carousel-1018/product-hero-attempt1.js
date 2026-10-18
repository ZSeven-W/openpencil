const sec = I(null, {type:"frame", name:"商品大图轮播区", width:"fill_container", height:"fit_content", layout:"vertical", clipContent:true});

const hero = I(sec, {type:"frame", name:"轮播视口", width:"fill_container", height:340, layout:"none", clipContent:true});

I(hero, {type:"image", name:"轮播图-1-马克杯主图", x:0, y:0, width:375, height:340, imageSearchQuery:"white ceramic mug product", imagePrompt:"雾白陶瓷马克杯产品摄影：一只雾白色陶瓷马克杯置于柔和浅灰背景中，柔和自然光，简洁电商产品照，杯身有细腻质感细节，画面居中留有呼吸空间", cornerRadius:0});

const backBtn = I(hero, {type:"frame", name:"返回按钮", x:16, y:16, width:44, height:44, cornerRadius:22, layout:"horizontal", alignItems:"center", justifyContent:"center", fill:[{type:"solid", color:"#FFFFFF66"}], stroke:{thickness:1, fill:[{type:"solid", color:"#FFFFFF99"}]}, effects:[{type:"shadow", offsetX:0, offsetY:2, blur:8, spread:0, color:"#0000001A"}]});
I(backBtn, {type:"icon_font", name:"返回图标", iconFontName:"chevron-left", iconFontFamily:"lucide", width:20, height:20, fill:[{type:"solid", color:"#1C1917"}]});

const favBtn = I(hero, {type:"frame", name:"收藏按钮", x:315, y:16, width:44, height:44, cornerRadius:22, layout:"horizontal", alignItems:"center", justifyContent:"center", fill:[{type:"solid", color:"#FFFFFF66"}], stroke:{thickness:1, fill:[{type:"solid", color:"#FFFFFF99"}]}, effects:[{type:"shadow", offsetX:0, offsetY:2, blur:8, spread:0, color:"#0000001A"}]});
I(favBtn, {type:"icon_font", name:"收藏图标", iconFontName:"heart", iconFontFamily:"lucide", width:20, height:20, fill:[{type:"solid", color:"#1C1917"}]});

const dots = I(hero, {type:"frame", name:"指示器", x:0, y:316, width:375, height:10, layout:"horizontal", gap:8, alignItems:"center", justifyContent:"center"});
const dotData = [true, false, false, false, false];
for (const active of dotData) {
  I(dots, {type:"ellipse", name: active ? "指示点-当前" : "指示点", width: active ? 18 : 6, height:6, fill:[{type:"solid", color: active ? "#FFFFFF" : "#FFFFFF80"}]});
}