const sec = I(null, {type:"frame", name:"商家详情", screen:"/screen-1", width:"fill_container", height:"fit_content", layout:"vertical", fill:[{type:"solid",color:"#FFFFFF"}]});

// ---- 头图 ----
const hero = I(sec, {type:"frame", name:"hero-viewport", layout:"none", width:"fill_container", height:240, clipContent:true});
I(hero, {type:"image", name:"商家头图", width:375, height:240, imageSearchQuery:"chinese restaurant dishes table", imagePrompt:"Appetizing overhead view of a Chinese restaurant table filled with steaming dishes, bowls and side plates, warm natural light, rich colors, food photography", cornerRadius:0});
const scrim = I(hero, {type:"rectangle", name:"hero-scrim", width:375, height:120, y:120, fill:[{type:"linear_gradient", angle:180, stops:[{offset:0,color:"#00000000"},{offset:1,color:"#000000B3"}]}]});
const backBtn = I(hero, {type:"frame", name:"back-button", layout:"horizontal", width:36, height:36, x:16, y:16, cornerRadius:18, alignItems:"center", justifyContent:"center", fill:[{type:"solid",color:"#00000066"}], events:{onTap:[{pop:null}]}});
I(backBtn, {type:"icon_font", iconFontName:"chevron-left", iconFontFamily:"lucide", width:20, height:20, fill:[{type:"solid",color:"#FFFFFF"}]});
const favBtn = I(hero, {type:"frame", name:"fav-button", layout:"horizontal", width:36, height:36, x:323, y:16, cornerRadius:18, alignItems:"center", justifyContent:"center", fill:[{type:"solid",color:"#00000066"}]});
I(favBtn, {type:"icon_font", iconFontName:"heart", iconFontFamily:"lucide", width:18, height:18, fill:[{type:"solid",color:"#FFFFFF"}]});
const heroMeta = I(hero, {type:"frame", name:"hero-meta", layout:"vertical", width:343, x:16, y:186, gap:6});
I(heroMeta, {type:"text", name:"商家名", content:"巷口小馆 · 家常菜", fontSize:20, fontWeight:700, fill:[{type:"solid",color:"#FFFFFF"}]});
I(heroMeta, {type:"text", name:"起送提示", content:"起送 ¥20 · 约30分钟送达", fontSize:12, fontWeight:400, fill:[{type:"solid",color:"#FFFFFFCC"}]});

// ---- 菜品分类横滑 ----
const catWrap = I(sec, {type:"frame", name:"category-rail", layout:"horizontal", width:"fill_container", height:"fit_content", padding:[16,0,12,24], gap:10, clipContent:false});
const cats = ["招牌热销","家常小炒","汤羹主食","凉菜小吃","饮品甜品"];
for (let i=0;i<cats.length;i++){
  const active = i===0;
  I(catWrap, {type:"text", name:"分类-"+cats[i], content:cats[i], fontSize:14, fontWeight:active?600:400, fill:[{type:"solid",color:active?"#111111":"#8A8A8E"}]});
}

// ---- 菜品列表六条 ----
const list = I(sec, {type:"frame", name:"dish-list", layout:"vertical", width:"fill_container", padding:[4,16,0,16], gap:14});
const dishes = [
  {n:"招牌红烧肉", d:"肥而不腻，入口即化 · 月售 326", p:"¥38", img:"braised pork belly", ip:"Close-up of glossy braised pork belly cubes in rich brown sauce served in a ceramic bowl, warm appetizing light"},
  {n:"麻婆豆腐", d:"麻辣鲜香，下饭首选 · 月售 284", p:"¥22", img:"mapo tofu dish", ip:"Bowl of mapo tofu with red chili oil sauce and minced pork, sprinkled scallions, steaming hot, rustic table"},
  {n:"宫保鸡丁", d:"微辣带甜，花生香脆 · 月售 251", p:"¥28", img:"kung pao chicken", ip:"Wok-fried kung pao chicken with dried chilies and peanuts in a plate, glossy sauce, vibrant food photography"},
  {n:"番茄蛋汤", d:"清爽暖胃 · 月售 198", p:"¥12", img:"tomato egg soup", ip:"Clear bowl of tomato and egg drop soup with fresh scallion garnish, soft daylight, home-style cooking"},
  {n:"清炒时蔬", d:"当日鲜采绿叶菜 · 月售 165", p:"¥16", img:"stir fried greens", ip:"Plate of bright green stir-fried leafy vegetables with garlic, glossy and fresh, clean ceramic plate"},
  {n:"酸辣土豆丝", d:"爽脆开胃 · 月售 142", p:"¥14", img:"hot sour potato shreds", ip:"Plate of finely shredded hot and sour potato strips with red chili slivers, crisp texture, appetizing light"}
];
for (const d of dishes){
  const card = I(list, {type:"frame", name:"菜品-"+d.n, layout:"horizontal", width:"fill_container", gap:12, alignItems:"center", padding:[10,0], stroke:{thickness:{bottom:1}, fill:[{type:"solid",color:"#F0F0F2"}]}});
  I(card, {type:"image", name:"菜品图-"+d.n, width:84, height:84, cornerRadius:12, imageSearchQuery:d.img, imagePrompt:d.ip});
  const mid = I(card, {type:"frame", layout:"vertical", width:"fill_container", gap:6});
  I(mid, {type:"text", content:d.n, fontSize:15, fontWeight:600, fill:[{type:"solid",color:"#1C1C1E"}]});
  I(mid, {type:"text", content:d.d, fontSize:12, fontWeight:400, fill:[{type:"solid",color:"#8A8A8E"}]});
  I(mid, {type:"text", content:d.p, fontSize:16, fontWeight:700, fill:[{type:"solid",color:"#E5482B"}]});
  const add = I(card, {type:"frame", name:"加入购物车-"+d.n, layout:"horizontal", width:32, height:32, cornerRadius:16, alignItems:"center", justifyContent:"center", fill:[{type:"solid",color:"#E5482B"}], effects:[{type:"shadow",offsetX:0,offsetY:2,blur:6,spread:0,color:"#E5482B44"}], animations:[{trigger:"onTap",durationMs:150,easing:"standard",keyframes:[{offset:0,values:{scaleX:1,scaleY:1}},{offset:0.5,values:{scaleX:0.88,scaleY:0.88}},{offset:1,values:{scaleX:1,scaleY:1}}]}], events:{onTap:[{toast:"已加入购物车"}]}});
  I(add, {type:"icon_font", iconFontName:"plus", iconFontFamily:"lucide", width:18, height:18, fill:[{type:"solid",color:"#FFFFFF"}]});
}

// ---- 底部留白（供全局购物车条/底部导航在页面级叠加） ----
I(sec, {type:"frame", name:"bottom-spacer", layout:"vertical", width:"fill_container", height:64});
