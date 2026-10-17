const sec = I(null, {type:"frame", name:"Hero with Title & Meta", width:"fill_container", height:330, layout:"none", clipContent:true});

// Back button (top-left overlay)
const back = I(sec, {type:"frame", name:"back-button", x:20, y:20, width:40, height:40, layout:"horizontal", justifyContent:"center", alignItems:"center", cornerRadius:9999, fill:[{type:"solid", color:"#FFFFFF"}], stroke:{thickness:1, fill:[{type:"solid", color:"#E5E5E5"}]}});
I(back, {type:"icon_font", iconFontName:"arrow-left", iconFontFamily:"lucide", width:20, height:20, fill:"#1A1A1A"});

// Bottom gradient scrim (transparent to dark)
I(sec, {type:"rectangle", name:"bottom-scrim", x:0, y:150, width:375, height:180, fill:[{type:"linear_gradient", angle:180, stops:[{offset:0, color:"#00000000"}, {offset:0.55, color:"#00000080"}, {offset:1, color:"#000000D9"}]}]});

// Top scrim for back button legibility
I(sec, {type:"rectangle", name:"top-scrim", x:0, y:0, width:375, height:90, fill:[{type:"linear_gradient", angle:180, stops:[{offset:0, color:"#00000033"}, {offset:1, color:"#00000000"}]}]});

// Overlay copy block
const copy = I(sec, {type:"frame", name:"hero-copy", x:20, y:214, width:335, layout:"vertical", gap:12});
const metaRow = I(copy, {type:"text", name:"workout-meta", content:"45 min  ·  12 exercises  ·  320 kcal", fontFamily:"DM Sans", fontSize:13, fontWeight:500, fill:"#FFFFFFCC", letterSpacing:0.3});
I(copy, {type:"text", name:"workout-title", content:"Full Body Strength", fontFamily:"Bricolage Grotesque", fontSize:30, fontWeight:700, fill:"#FFFFFF", letterSpacing:-0.5, lineHeight:1.05});
const badges = I(copy, {type:"frame", name:"difficulty-badges", layout:"horizontal", gap:8});
const levels = [{label:"Beginner", active:false},{label:"Intermediate", active:true}];
for (const lv of levels) {
  I(badges, {type:"frame", name:lv.active?"badge-active":"badge-inactive", layout:"horizontal", justifyContent:"center", alignItems:"center", padding:[6,14], cornerRadius:9999, fill:[{type:"solid", color: lv.active ? "#FF6B6B" : "#FFFFFF26"}], stroke:{thickness:1, fill:[{type:"solid", color: lv.active ? "#FF6B6B" : "#FFFFFF40"}]}});
  // text inserted into badge frame
}
// insert badge labels into the two badge frames
// Protagonist hero photo last (bottom of stack)
I(sec, {type:"image", name:"hero-image", x:0, y:0, width:375, height:330, imageSearchQuery:"full body strength workout", imagePrompt:"Athletic woman performing a full body strength training workout with dumbbells in a bright modern gym, mid-exercise, natural light, dynamic composition, photo fills frame edge to edge"});
