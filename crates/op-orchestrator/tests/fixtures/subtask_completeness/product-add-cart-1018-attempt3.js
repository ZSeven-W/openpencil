const sec = I(null, {type:"frame", name:"底部加购条", width:"fill_container", height:"fit_content", layout:"vertical", fill:[{type:"solid", color:"#FFFFFF"}], stroke:{thickness:{top:1}, fill:[{type:"solid", color:"#E7E5E4"}]}});

const row = I(sec, {type:"frame", name:"加购条内容", layout:"horizontal", width:"fill_container", alignItems:"center", padding:[12,24,12,24], gap:12});

const iconGroup = I(row, {type:"frame", name:"左侧图标按钮组", layout:"horizontal", gap:16, alignItems:"center"});
const icons = [
  {name:"客服按钮", icon:"headset"},
  {name:"店铺按钮", icon:"store"}
];
for (const it of icons) {
  const btn = I(iconGroup, {type:"frame", name:it.name, layout:"horizontal", width:44, height:44, justifyContent:"center", alignItems:"center", cornerRadius:12, fill:[{type:"solid", color:"#FFF7ED"}], stroke:{thickness:1, fill:[{type:"solid", color:"#F5F5F4"}]},
    animations:[{trigger:"press", durationMs:150, easing:"standard", keyframes:[{offset:0, values:{scaleX:1, scaleY:1}}, {offset:1, values:{scaleX:0.96, scaleY:0.96}}]}]});
  I(btn, {type:"icon_font", iconFontName:it.icon, iconFontFamily:"lucide", width:20, height:20, fill:"#57534E"});
}

const cta = I(row, {type:"frame", name:"加入购物车按钮", layout:"horizontal", width:"fill_container", height:52, justifyContent:"center", alignItems:"center", cornerRadius:12, fill:[{type:"linear_gradient", angle:90, stops:[{offset:0, color:"#F97316"}, {offset:1, color:"#EA580C"}]}], effects:[{type:"shadow", offsetX:0, offsetY:4, blur:12, spread:0, color:"#F9731633"}],
  events:{onTap:[{push:"\"/screen-1\""}]},
  animations:[{trigger:"press", durationMs:150, easing:"standard", keyframes:[{offset:0, values:{scaleX:1, scaleY:1}}, {offset:1, values:{scaleX:0.96, scaleY:0.96}}]}]});

const ctaInner = I(cta, {type:"frame", name:"按钮文案组", layout:"horizontal", gap:6, alignItems:"center"});
I(ctaInner, {type:"text", name:"按钮主文案", content:"加入购物车", fontFamily:"Plus Jakarta Sans", fontSize:16, fontWeight:600, fill:"#FFFFFF"});
I(ctaInner, {type:"text", name:"按钮摘要", content:"¥129 · 白色 / L", fontFamily:"Plus Jakarta Sans", fontSize:11, fontWeight:400, fill:"#FFEDD5"});

I(sec, {type:"frame", name:"底部安全区留白", width:"fill_container", height:12});