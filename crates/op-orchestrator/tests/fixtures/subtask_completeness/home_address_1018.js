const sec = I(null, {type:"frame", name:"首页地址栏与搜索", width:"fill_container", height:"fit_content", layout:"vertical", padding:[8,24,12,24], gap:14});

const addrRow = I(sec, {type:"frame", name:"地址行", layout:"horizontal", width:"fill_container", alignItems:"center", gap:6});
I(addrRow, {type:"icon_font", name:"定位图标", iconFontName:"map-pin", iconFontFamily:"lucide", width:18, height:18, fill:"#FF5A1F"});
I(addrRow, {type:"text", content:"天府二街 66 号", fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:16, fontWeight:700, fill:"#21140F", letterSpacing:-0.3});
I(addrRow, {type:"icon_font", name:"下拉箭头", iconFontName:"chevron-down", iconFontFamily:"lucide", width:16, height:16, fill:"#7A5B48"});
const spacer = I(addrRow, {type:"frame", name:"弹性占位", layout:"horizontal", width:"fill_container", height:1});
const bellWrap = I(addrRow, {type:"frame", name:"铃铛容器", layout:"none", width:26, height:26});
I(bellWrap, {type:"icon_font", name:"消息铃铛", iconFontName:"bell", iconFontFamily:"lucide", width:22, height:22, fill:"#21140F", x:2, y:2});
I(bellWrap, {type:"ellipse", name:"红点", width:8, height:8, fill:"#FF5A1F", x:16, y:1});

const search = I(sec, {type:"text_input", name:"搜索框", width:"fill_container", height:46, placeholder:"搜索商家、菜品", value:"", leadingIcon:"search", trailingIcon:"history", fill:"#FFFFFF", stroke:{thickness:1, fill:"#EAD8C8"}, cornerRadius:12, effects:[{type:"shadow", offsetX:0, offsetY:2, blur:6, spread:0, color:"#21140F0A"}]});
