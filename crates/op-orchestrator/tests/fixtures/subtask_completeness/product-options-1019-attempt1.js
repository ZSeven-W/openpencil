```javascript
const sec = I(null, {type:"frame", name:"商品页", screen:"/", width:"fill_container", height:"fit_content", layout:"vertical", fill:[]});

// ---- Carousel (full-bleed media, layout:none stack) ----
const carousel = I(sec, {type:"frame", name:"Hero Carousel", width:"fill_container", height:320, layout:"none", clipContent:true, fill:[{type:"solid", color:"#EDE9E2"}]});
I(carousel, {type:"image", name:"Product Hero", width:375, height:320, cornerRadius:0, imageSearchQuery:"beige hoodie sweatshirt", imagePrompt:"米白色宽松连帽卫衣，平铺在暖米色背景上的产品摄影，柔和自然光，居中构图，电商商品主图风格"});
I(carousel, {type:"frame", name:"Photo Count Badge", x:311, y:16, width:48, height:24, layout:"horizontal", alignItems:"center", justifyContent:"center", cornerRadius:999, fill:[{type:"solid", color:"#1C191799"}],
  children:[{type:"text", name:"Count Text", content:"1/4", fontFamily:"Plus Jakarta Sans", fontSize:12, fontWeight:500, fill:[{type:"solid", color:"#FFFFFF"}]}]});

// page indicator dots
const dots = I(carousel, {type:"frame", name:"Indicator Dots", x:0, y:296, width:375, height:12, layout:"horizontal", gap:6, alignItems:"center", justifyContent:"center", fill:[]});
for (let i=0;i<4;i++){
  I(dots, {type:"ellipse", name:"Dot "+(i+1), width:i===0?16:6, height:6, fill:[{type:"solid", color:i===0?"#F97316":"#FFFFFFB3"}]});
}

// ---- Price + product info rail ----
const rail = I(sec, {type:"frame", name:"Info Rail", width:"fill_container", height:"fit_content", layout:"vertical", gap:12, padding:[20,24,0,24], fill:[]});

const priceRow = I(rail, {type:"frame", name:"Price Row", width:"fill_container", height:"fit_content", layout:"horizontal", gap:8, alignItems:"end", fill:[]});
I(priceRow, {type:"text", name:"Price Main", content:"¥299", fontFamily:"Plus Jakarta Sans", fontSize:34, fontWeight:700, letterSpacing:-0.5, lineHeight:1.05, fill:[{type:"solid", color:"#EA580C"}],
  animations:[{trigger:"mount", durationMs:600, easing:"emphasizedDecelerate", keyframes:[{offset:0, values:{opacity:0, translateY:10}},{offset:1, values:{opacity:1, translateY:0}}]}]});
I(priceRow, {type:"text", name:"Price Original", content:"¥399", fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:400, lineHeight:1.4, fill:[{type:"solid", color:"#A8A29E"}]});
const tag = I(priceRow, {type:"frame", name:"Promo Tag", layout:"horizontal", width:"fit_content", height:22, padding:[3,8], alignItems:"center", justifyContent:"center", cornerRadius:6, fill:[{type:"solid", color:"#FFF7ED"}]});
I(tag, {type:"text", name:"Promo Text", content:"限时直降", fontFamily:"Plus Jakarta Sans", fontSize:11, fontWeight:500, fill:[{type:"solid", color:"#EA580C"}]});

I(rail, {type:"text", name:"Product Name", content:"云朵卫衣", fontFamily:"Plus Jakarta Sans", fontSize:22, fontWeight:600, letterSpacing:-0.3, lineHeight:1.2, width:"fill_container", textGrowth:"fixed-width", fill:[{type:"solid", color:"#1C1917"}]});
I(rail, {type:"text", name:"Product Subtitle", content:"云朵触感面料，宽松落肩版型，一穿就爱上", fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:400, lineHeight:1.5, width:"fill_container", textGrowth:"fixed-width", fill:[{type:"solid", color:"#57534E"}]});

// ---- Spec selection ----
const specState = {color:{type:"string", default:"雾白"}, size:{type:"string", default:"M"}};
const specSection = I(rail, {type:"frame", name:"Spec Section", width:"fill_container", height:"fit_content", layout:"vertical", gap:12, state:specState, fill:[]});

const colorGroup = I(specSection, {type:"frame", name:"Color Group", width:"fill_container", height:"fit_content", layout:"horizontal", gap:8, alignItems:"center", fill:[]});
I(colorGroup, {type:"text", name:"Color Label", content:"颜色", fontFamily:"Plus Jakarta Sans", fontSize:13, fontWeight:500, fill:[{type:"solid", color:"#A8A29E"}]});
const colorWrap = I(colorGroup, {type:"frame", name:"Color Chips", layout:"horizontal", width:"fit_content", height:"fit_content", gap:8, fill:[]});
for (const [v,active] of [["雾白",true],["燕麦",false],["深咖",false]]){
  const on = "$app.color == " + JSON.stringify(v);
  const chip = I(colorWrap, {type:"frame", name:"Color "+v, layout:"horizontal", width:"fit_content", height:34, padding:[0,16], alignItems:"center", justifyContent:"center", cornerRadius:999,
    fill:[{type:"solid", color:active?"#1C1917":"#F5F5F4"}],
    stroke:{thickness:1, fill:[{type:"solid", color:active?"#1C1917":"#E7E5E4"}]},
    bindings:{fill:on+" ? '#1C1917' : '#F5F5F4'"},
    transition:{property:"fill", durationMs:250, easing:"standard"},
    events:{onTap:[{set:{"$app.color":JSON.stringify(v)}}]}});
  I(chip, {type:"text", name:"Color Chip Label "+v, content:v, fontFamily:"Plus Jakarta Sans", fontSize:13, fontWeight:500,
    fill:[{type:"solid", color:active?"#FFFFFF":"#57534E"}],
    bindings:{fill:on+" ? '#FFFFFF' : '#57534E'"}});
}

const sizeGroup = I(specSection, {type:"frame", name:"Size Group", width:"fill_container", height:"fit_content", layout:"horizontal", gap:8, alignItems:"center", fill:[]});
I(sizeGroup, {type:"text", name:"Size Label", content:"尺码", fontFamily:"Plus Jakarta Sans", fontSize:13, fontWeight:500, fill:[{type:"solid", color:"#A8A29E"}]});
const sizeWrap = I(sizeGroup, {type:"frame", name:"Size Chips", layout:"horizontal", width:"fit_content", height:"fit_content", gap:8, fill:[]});
for (const v of ["S","M","L","XL"]){
  const active = v==="M";
  const on = "$app.size == " + JSON.stringify(v);
  const chip = I(sizeWrap, {type:"frame", name:"Size "+v, layout:"horizontal", width:44, height:34, alignItems:"center", justifyContent:"center", cornerRadius:999,
    fill:[{type:"solid", color:active?"#1C1917":"#F5F5F4"}],
    stroke:{thickness:1, fill:[{type:"solid", color:active?"#1C1917":"#E7E5E4"}]},
    bindings:{fill:on+" ? '#1C1917' : '#F5F5F4'"},
    transition:{property:"fill", durationMs:250, easing:"standard"},
    events:{onTap:[{set:{"$app.size":JSON.stringify(v)}}]}});
  I(chip, {type:"text", name:"Size Chip Label "+v, content:v, fontFamily:"Plus Jakarta Sans", fontSize:13, fontWeight:500,
    fill:[{type:"solid", color:active?"#FFFFFF":"#57534E"}],
    bindings:{fill:on+" ? '#FFFFFF' : '#57534E'"}});
}

// ---- Reviews ----
const reviews = I(sec, {type:"frame", name:"Reviews Section", width:"fill_container", height:"fit_content", layout:"vertical", gap:16, padding:[24,24,0,24], fill:[]});
const revHead = I(reviews, {type:"frame", name:"Reviews Header", width:"fill_container", height:"fit_content", layout:"horizontal", gap:8, alignItems:"center", fill:[]});
I(revHead, {type:"text", name:"Reviews Title", content:"用户评价", fontFamily:"Plus Jakarta Sans", fontSize:18, fontWeight:600, letterSpacing:-0.3, fill:[{type:"solid", color:"#1C1917"}]});
I(revHead, {type:"text", name:"Reviews Score", content:"4.9", fontFamily:"Plus Jakarta Sans", fontSize:18, fontWeight:700, fill:[{type:"solid", color:"#F97316"}]});
I(revHead, {type:"text", name:"Reviews Count", content:"326条", fontFamily:"Plus Jakarta Sans", fontSize:12, fontWeight:400, fill:[{type:"solid", color:"#A8A29E"}]});

const revData = [
  {name:"小鹿不吃草", text:"面料真的像云朵一样软，版型宽松但不显胖，雾白超百搭。"},
  {name:"阿澄", text:"燕麦色很温柔，洗了一次也没起球，细节走线很工整。"},
  {name:"Momo", text:"平时穿 M 这次也拿 M 刚好，里面套件衬衫也不紧绷。"}
];
for (const [i,r] of revData.entries()){
  const row = I(reviews, {type:"frame", name:"Review Row "+(i+1), width:"fill_container", height:"fit_content", layout:"horizontal", gap:12, alignItems:"start",
    padding:[16,16], cornerRadius:16, fill:[{type:"solid", color:"#FFFFFF"}],
    stroke:{thickness:1, fill:[{type:"solid", color:"#E7E5E4"}]}});
  I(row, {type:"ellipse", name:"Avatar "+(i+1), width:36, height:36, fill:[{type:"solid", color:["#FDE6CF","#E8E3D8","#DCE5DD"][i]}]});
  const col = I(row, {type:"frame", name:"Review Col "+(i+1), layout:"vertical", width:"fill_container", height:"fit_content", gap:6, fill:[]});
  const meta = I(col, {type:"frame", name:"Review Meta "+(i+1), layout:"horizontal", width:"fill_container", height:"fit_content", gap:8, alignItems:"center", fill:[]});
  I(meta, {type:"text", name:"Nickname "+(i+1), content:r.name, fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:600, fill:[{type:"solid", color:"#1C1917"}]});
  const stars = I(meta, {type:"frame", name:"Stars "+(i+1), layout:"horizontal", width:"fit_content", height:"fit_content", gap:2, fill:[]});
  for (let s=0;s<5;s++){
    I(stars, {type:"icon_font", name:"Star "+(s+1), iconFontName:"star", iconFontFamily:"lucide", width:12, height:12, fill:[{type:"solid", color:"#F97316"}]});
  }
  I(col, {type:"text", name:"Review Text "+(i+1), content:r.text, fontFamily:"Plus Jakarta Sans", fontSize:13, fontWeight:400, lineHeight:1.55, width:"fill_container", textGrowth:"fixed-width", fill:[{type:"solid", color:"#57534E"}]});
}

// ---- Bottom action bar ----
const bottom = I(sec, {type:"frame", name:"Bottom Action Bar", width:"fill_container", height:78, layout:"horizontal", gap:12, padding:[12,24], alignItems:"center", fill:[{type:"solid", color:"#FFFFFF"}],
  stroke:{thickness:{top:1}, fill:[{type:"solid", color:"#E7E5E4"}]}});
for (const [nm,ic] of [["客服","headphones"],["购物车","shopping-cart"]]){
  const btn = I(bottom, {type:"frame", name:"Icon Button "+nm, width:44, height:44, layout:"vertical", gap:2, alignItems:"center", justifyContent:"center", cornerRadius:12, fill:[]});
  I(btn, {type:"icon_font", name:"Icon "+nm, iconFontName:ic, iconFontFamily:"lucide", width:20, height:20, fill:[{type:"solid", color:"#57534E"}]});
}
const cartBtn = I(bottom, {type:"frame", name:"Add To Cart Button", layout:"horizontal", width:"fill_container", height:48, alignItems:"center", justifyContent:"center", cornerRadius:12,
  fill:[{type:"solid", color:"#FFF7ED"}], stroke:{thickness:1, fill:[{type:"solid", color:"#F97316"}]},
  events:{onTap:[{animate:{target:"cart-anim", property:"scaleX", from:1, to:0.95, durationMs:120, easing:"ease_out", fillMode:"forwards"}},
                  {animate:{target:"cart-anim", property:"scaleY", from:1, to:0.95, durationMs:120, easing:"ease_out", fillMode:"forwards"}},
                  {animate:{target:"cart-anim", property:"scaleX", from:0.95, to:1, durationMs:160, delayMs:120, easing:"ease_out", fillMode:"forwards"}},
                  {animate:{target:"cart-anim", property:"scaleY", from:0.95, to:1, durationMs:160, delayMs:120, easing:"ease_out", fillMode:"forwards"}},
                  {push:"\"/screen-1\""}]}});
I(cartBtn, {type:"text", name:"Add To Cart Label", content:"加入购物车", fontFamily:"Plus Jakarta Sans", fontSize:15, fontWeight:600, fill:[{type:"solid", color:"#EA580C"}]});
const buyBtn = I(bottom, {type:"frame", name:"Buy Now Button", layout:"horizontal", width:"fill_container", height:48, alignItems:"center", justifyContent:"center", cornerRadius:12,
  fill:[{type:"solid", color:"#F97316"}],
  events:{onTap:[{push:"\"/screen-1\""}]}});
I(buyBtn, {type:"text", name:"Buy Now Label", content:"立即购买", fontFamily:"Plus Jakarta Sans", fontSize:15, fontWeight:600, fill:[{type:"solid", color:"#FFFFFF"}]});
```