const sec = I(null, {type:"frame", name:"底部加购条", width:"fill_container", height:"fit_content", layout:"vertical", padding:[10,24,16,24], fill:[{type:"solid", color:"#FFFFFF"}], stroke:{thickness:{top:1}, fill:[{type:"solid", color:"#F5F5F4"}]}});

const row = I(sec, {type:"frame", name:"加购条内容行", width:"fill_container", height:"fit_content", layout:"horizontal", gap:12, alignItems:"center", justifyContent:"space_between"});

const iconGroup = I(row, {type:"frame", name:"客服店铺按钮组", width:"fit_content", height:"fit_content", layout:"horizontal", gap:4, alignItems:"center"});

const svcBtn = I(iconGroup, {type:"frame", name:"客服按钮", width:48, height:48, layout:"vertical", alignItems:"center", justifyContent:"center", gap:2, cornerRadius:12, events:{onTap:[{toast:"'正在为您接入客服…'"}]}});
I(svcBtn, {type:"icon_font", name:"客服图标", iconFontName:"message-circle", iconFontFamily:"lucide", width:20, height:20, fill:[{type:"solid", color:"#57534E"}]});
I(svcBtn, {type:"text", name:"客服文字", content:"客服", fontFamily:"Plus Jakarta Sans", fontSize:11, fontWeight:500, fill:[{type:"solid", color:"#A8A29E"}], textAlign:"center", textGrowth:"fixed-width", width:40});

const shopBtn = I(iconGroup, {type:"frame", name:"店铺按钮", width:48, height:48, layout:"vertical", alignItems:"center", justifyContent:"center", gap:2, cornerRadius:12, events:{onTap:[{toast:"'进入店铺…'"}]}});
I(shopBtn, {type:"icon_font", name:"店铺图标", iconFontName:"store", iconFontFamily:"lucide", width:20, height:20, fill:[{type:"solid", color:"#57534E"}]});
I(shopBtn, {type:"text", name:"店铺文字", content:"店铺", fontFamily:"Plus Jakarta Sans", fontSize:11, fontWeight:500, fill:[{type:"solid", color:"#A8A29E"}], textAlign:"center", textGrowth:"fixed-width", width:40});

const cta = I(row, {type:"frame", name:"加入购物车按钮", width:"fill_container", height:50, layout:"horizontal", alignItems:"center", justifyContent:"center", gap:8, cornerRadius:12, fill:[{type:"solid", color:"#F97316"}], effects:[{type:"shadow", offsetX:0, offsetY:4, blur:12, spread:0, color:"#F9731633"}], events:{onTap:[{push:"\"/screen-1\""}]}});

const ctaTextCol = I(cta, {type:"frame", name:"按钮文案组", width:"fit_content", height:"fit_content", layout:"vertical", alignItems:"center", gap:1});
I(ctaTextCol, {type:"text", name:"按钮主文案", content:"加入购物车", fontFamily:"Plus Jakarta Sans", fontSize:16, fontWeight:600, fill:[{type:"solid", color:"#FFFFFF"}], width:"fit_content", textGrowth:"auto"});
I(ctaTextCol, {type:"text", name:"按钮规格摘要", content:"¥129 · 白色 · L", fontFamily:"Plus Jakarta Sans", fontSize:11, fontWeight:400, fill:[{type:"solid", color:"#FFFFFF"}], opacity:0.75, width:"fit_content", textGrowth:"auto"});

I(cta, {type:"icon_font", name:"购物车图标", iconFontName:"shopping-cart", iconFontFamily:"lucide", width:18, height:18, fill:[{type:"solid", color:"#FFFFFF"}]});