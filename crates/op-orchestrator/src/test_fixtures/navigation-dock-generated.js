const root = I(null, {type:"frame", name:"地图与悬浮导航", layout:"none", width:"fill_container", height:734, padding:0, clipContent:true});

// overlay children FIRST (topmost), map LAST

// Brand chip top-right area (subtle overlay, editable label)
const brand = I(root, {type:"frame", name:"brand-chip", layout:"horizontal", x:16, y:16, width:"fit_content", height:32, padding:[6,12], alignItems:"center", justifyContent:"center", cornerRadius:999, fill:[{type:"solid", color:"#0F1B2D99"}]});
I(brand, {type:"text", name:"brand-label", content:"沿途", fontFamily:"Outfit", fontSize:13, fontWeight:600, fill:[{type:"solid", color:"#F8FAFC"}]});

// Direction card
const dir = I(root, {type:"frame", name:"direction-card", layout:"horizontal", x:16, y:60, width:220, height:"fit_content", padding:[14,16], gap:12, alignItems:"center", cornerRadius:16, fill:[{type:"solid", color:"#1C2B42E6"}], effects:[{type:"shadow", offsetX:0, offsetY:6, blur:16, spread:0, color:"#0F1B2D59"}]});
const dirIcon = I(dir, {type:"frame", name:"turn-icon-holder", layout:"horizontal", width:40, height:40, cornerRadius:20, alignItems:"center", justifyContent:"center", fill:[{type:"solid", color:"#F97316"}]});
I(dirIcon, {type:"icon_font", name:"turn-left-icon", iconFontName:"corner-up-left", iconFontFamily:"lucide", width:22, height:22, fill:[{type:"solid", color:"#0F1B2D"}]});
const dirText = I(dir, {type:"frame", name:"turn-text-col", layout:"vertical", width:"fill_container", gap:2});
I(dirText, {type:"text", name:"turn-distance-label", content:"前方200米", fontFamily:"Outfit", fontSize:12, fontWeight:400, fill:[{type:"solid", color:"#94A3B8"}]});
I(dirText, {type:"text", name:"turn-action-label", content:"右转", fontFamily:"Outfit", fontSize:18, fontWeight:600, fill:[{type:"solid", color:"#F8FAFC"}]});

// Right-side floating controls (layer + locate), below direction card
const ctrls = [
  {name:"layer-button", icon:"layers", label:"图层", active:true},
  {name:"locate-button", icon:"navigation", label:"定位", active:false}
];
for (const c of ctrls) {
  const btn = I(root, {type:"frame", name:c.name, layout:"horizontal", x:319, y:200 + (c.active ? 0 : 52), width:40, height:40, alignItems:"center", justifyContent:"center", cornerRadius:20, fill:[{type:"solid", color: c.active ? "#F97316" : "#1C2B42E6"}], stroke:{thickness:1, fill:[{type:"solid", color: c.active ? "#F97316" : "#2A3A55"}]}, effects:[{type:"shadow", offsetX:0, offsetY:4, blur:12, spread:0, color:"#0F1B2D59"}]});
  I(btn, {type:"icon_font", name:c.name+"-icon", iconFontName:c.icon, iconFontFamily:"lucide", width:20, height:20, fill:[{type:"solid", color: c.active ? "#0F1B2D" : "#94A3B8"}]});
}

// Bottom summary dock
const dock = I(root, {type:"frame", name:"bottom-nav-dock", layout:"vertical", x:16, y:646, width:343, height:"fit_content", padding:[16,16,16,16], gap:14, cornerRadius:20, fill:[{type:"solid", color:"#1C2B42F2"}], stroke:{thickness:1, fill:[{type:"solid", color:"#2A3A55"}]}, effects:[{type:"shadow", offsetX:0, offsetY:-4, blur:20, spread:0, color:"#0F1B2D40"}]});
const statsRow = I(dock, {type:"frame", name:"trip-stats-row", layout:"horizontal", width:"fill_container", height:"fit_content", justifyContent:"space_between", alignItems:"end"});
const stat1 = I(statsRow, {type:"frame", name:"distance-stat", layout:"vertical", width:"fit_content", gap:2});
I(stat1, {type:"text", name:"distance-value", content:"剩余3.2公里", fontFamily:"Outfit", fontSize:20, fontWeight:700, fill:[{type:"solid", color:"#F8FAFC"}]});
I(stat1, {type:"text", name:"distance-label", content:"剩余里程", fontFamily:"Outfit", fontSize:12, fontWeight:400, fill:[{type:"solid", color:"#94A3B8"}]});
const stat2 = I(statsRow, {type:"frame", name:"time-stat", layout:"vertical", width:"fit_content", gap:2});
I(stat2, {type:"text", name:"time-value", content:"预计12分钟", fontFamily:"Outfit", fontSize:20, fontWeight:700, fill:[{type:"solid", color:"#F8FAFC"}]});
I(stat2, {type:"text", name:"time-label", content:"预计时间", fontFamily:"Outfit", fontSize:12, fontWeight:400, fill:[{type:"solid", color:"#94A3B8"}]});
const endBtn = I(dock, {type:"frame", name:"end-navigation-button", layout:"horizontal", width:"fill_container", height:48, padding:[12,24], alignItems:"center", justifyContent:"center", cornerRadius:12, fill:[{type:"solid", color:"#F97316"}]});
I(endBtn, {type:"text", name:"end-navigation-label", content:"结束导航", fontFamily:"Outfit", fontSize:16, fontWeight:600, fill:[{type:"solid", color:"#0F1B2D"}]});

// Map image LAST (paints beneath overlays)
I(root, {type:"image", name:"Map", width:375, height:734, imageSearchQuery:"map-template:running", imagePrompt:"Dark navy blue cycling navigation map of a city street grid with a warm orange highlighted route, starting point and destination markers, light street lines, no people, top-down view"});
