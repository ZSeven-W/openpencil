const sec = I(null, {type:"frame", name:"Hero with Title & Meta", width:"fill_container", height:330, layout:"none", clipContent:true});

const hero = I(sec, {type:"image", name:"workout-hero-image", width:375, height:330, imageSearchQuery:"dumbbell workout gym", imagePrompt:"A person mid-workout in a bright modern gym holding a dumbbell, energetic full-body strength training scene, natural light, confident crop"});

I(sec, {type:"rectangle", name:"hero-bottom-scrim", width:375, height:330, fill:[{type:"linear_gradient", angle:180, stops:[{offset:0, color:"#00000000"},{offset:0.45, color:"#00000033"},{offset:1, color:"#000000CC"}]}]});

const back = I(sec, {type:"frame", name:"back-button", x:20, y:18, width:40, height:40, cornerRadius:20, fill:[{type:"solid", color:"#FFFFFFCC"}], layout:"horizontal", alignItems:"center", justifyContent:"center"});
I(back, {type:"icon_font", name:"back-icon", iconFontName:"chevron-left", iconFontFamily:"lucide", width:20, height:20, fill:[{type:"solid", color:"#1A1A1A"}]});

const copy = I(sec, {type:"frame", name:"hero-copy", x:24, y:170, width:327, height:140, layout:"vertical", gap:10});
I(copy, {type:"text", name:"workout-title", content:"Full Body Strength", fontFamily:"Bricolage Grotesque", fontSize:28, fontWeight:700, fill:[{type:"solid", color:"#FFFFFF"}], width:327, textGrowth:"fixed-width", lineHeight:1.1, letterSpacing:-0.5});

const meta = I(copy, {type:"frame", name:"meta-row", layout:"horizontal", width:"fit_content", height:"fit_content", gap:6, alignItems:"center"});
const metas = ["45 min","12 exercises","320 kcal"];
metas.forEach((m, i) => {
  if (i > 0) I(meta, {type:"text", name:"meta-dot-"+i, content:"·", fontFamily:"DM Sans", fontSize:13, fontWeight:400, fill:[{type:"solid", color:"#FFFFFF99"}]});
  I(meta, {type:"text", name:"meta-"+i, content:m, fontFamily:"DM Sans", fontSize:13, fontWeight:500, fill:[{type:"solid", color:"#FFFFFFE6"}]});
});

const chips = I(copy, {type:"frame", name:"difficulty-chips", layout:"horizontal", width:"fit_content", height:"fit_content", gap:8});
const levels = [{label:"Beginner", active:false},{label:"Intermediate", active:true}];
for (const lv of levels) {
  const chip = I(chips, {type:"frame", name:"chip-"+lv.label, layout:"horizontal", padding:[6,14], cornerRadius:999, alignItems:"center", justifyContent:"center", fill: lv.active ? [{type:"solid", color:"#FF6B6B"}] : [{type:"solid", color:"#FFFFFF1A"}], stroke: lv.active ? undefined : {thickness:1, fill:[{type:"solid", color:"#FFFFFF4D"}]}});
  I(chip, {type:"text", name:"chip-label-"+lv.label, content:lv.label, fontFamily:"DM Sans", fontSize:12, fontWeight:500, letterSpacing:0.5, fill:[{type:"solid", color: lv.active ? "#FFFFFF" : "#FFFFFFCC"}]});
}
