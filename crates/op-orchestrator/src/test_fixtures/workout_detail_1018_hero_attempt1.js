const sec = I(null, {type:"frame", name:"Hero with Title & Meta", width:"fill_container", height:"fit_content", layout:"vertical"});

const hero = I(sec, {type:"frame", name:"Hero Viewport", width:"fill_container", height:330, layout:"none", clipContent:true});

const title = I(hero, {type:"text", name:"Workout Title", content:"Full Body Strength", fontFamily:"Bricolage Grotesque", fontSize:28, fontWeight:700, fill:[{type:"solid", color:"#FFFFFF"}], letterSpacing:-0.5, lineHeight:1.1, x:24, y:226, width:300, textGrowth:"fixed-width"});

const meta = I(hero, {type:"text", name:"Meta Row", content:"45 min · 12 exercises · 320 kcal", fontFamily:"DM Sans", fontSize:13, fontWeight:500, fill:[{type:"solid", color:"#FFFFFFE6"}], letterSpacing:0.3, lineHeight:1.4, x:24, y:262, width:300, textGrowth:"fixed-width"});

const chips = I(hero, {type:"frame", name:"Difficulty Chips", layout:"horizontal", gap:8, x:24, y:286});
const chipData = [
  {label:"Beginner", active:false},
  {label:"Intermediate", active:true}
];
for (const c of chipData) {
  const chip = I(chips, {type:"frame", name:(c.active ? "Chip Active" : "Chip") + " " + c.label, layout:"horizontal", alignItems:"center", justifyContent:"center", padding:[6,14], cornerRadius:999,
    fill: c.active ? [{type:"solid", color:"#FF6B6B"}] : [{type:"solid", color:"#FFFFFF2E"}],
    stroke: c.active ? {thickness:0, fill:[]} : {thickness:1, fill:[{type:"solid", color:"#FFFFFF66"}]}});
  I(chip, {type:"text", content:c.label, fontFamily:"DM Sans", fontSize:11, fontWeight:600, letterSpacing:0.5, fill:[{type:"solid", color:"#FFFFFF"}]});
}

const scrim = I(hero, {type:"rectangle", name:"Bottom Scrim", width:375, height:200, y:130, x:0,
  fill:[{type:"linear_gradient", angle:180, stops:[{offset:0, color:"#00000000"}, {offset:1, color:"#0F0A0ACF"}]}]});

const photo = I(hero, {type:"image", name:"Hero Workout Photo", width:375, height:330, x:0, y:0,
  imageSearchQuery:"full body workout dumbbells",
  imagePrompt:"A woman in athletic wear performing a full-body strength training workout with dumbbells in a bright modern gym, mid-exercise, natural window light, energetic mood"});

const back = I(hero, {type:"frame", name:"Back Button", layout:"horizontal", alignItems:"center", justifyContent:"center", width:44, height:44, x:16, y:16, cornerRadius:22,
  fill:[{type:"solid", color:"#0F0A0A66"}]});
I(back, {type:"icon_font", iconFontName:"chevron-left", iconFontFamily:"lucide", width:22, height:22, fill:[{type:"solid", color:"#FFFFFF"}]});
