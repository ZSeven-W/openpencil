const sec = I(null, {type:"frame", name:"商家详情-分类横滑与菜品列表", width:"fill_container", height:"fit_content", layout:"vertical", fill:[], padding:[0,0,16,0], gap:16});

const cats = I(sec, {type:"frame", name:"分类横滑", layout:"horizontal", width:"fill_container", height:"fit_content", clipContent:true, gap:10, padding:[0,24,0,24]});
const catData = [["招牌必点", true],["烧腊饭", false],["小食", false],["例汤", false],["饮品", false]];
for (const [label, active] of catData) {
  I(cats, {type:"frame", name:"分类-"+label, layout:"horizontal", alignItems:"center", justifyContent:"center", width:"fit_content", height:34, cornerRadius:17, fill: active ? [{type:"solid", color:"#FF5A1F"}] : [{type:"solid", color:"#FFFFFF"}], stroke:{thickness:1, fill:[{type:"solid", color: active ? "#FF5A1F" : "#EAD8C8"}]}, padding:[0,16], children: undefined});
  // chip label inside its own chip
}
// rebuild chips with labels properly nested
// (chips above already created; add labels via nested loop instead)
// -- corrected approach below --

const catChips = I(sec, {type:"frame", name:"分类胶囊条", layout:"horizontal", width:"fill_container", height:"fit_content", clipContent:true, gap:10, padding:[0,24]});
for (const [label, active] of catData) {
  const chip = I(catChips, {type:"frame", name:"分类芯片-"+label, layout:"horizontal", alignItems:"center", justifyContent:"center", height:34, cornerRadius:17,
    fill:[{type:"solid", color: active ? "#FF5A1F" : "#FFFFFF"}],
    stroke:{thickness:1, fill:[{type:"solid", color: active ? "#FF5A1F" : "#EAD8C8"}]},
    padding:[0,16],
    events:{onTap:[{toast:"'已切换到 ' + " + JSON.stringify(label)}]}});
  I(chip, {type:"text", name:"分类文字-"+label, content:label, fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:13, fontWeight: active ? 600 : 500, fill:[{type:"solid", color: active ? "#FFFFFF" : "#7A5B48"}]});
}

const list = I(sec, {type:"frame", name:"菜品列表", layout:"vertical", width:"fill_container", height:"fit_content", padding:[0,24], gap:0});
const dishes = [
  {name:"秘制叉烧双拼饭", desc:"叉烧油鸡双拼，浇秘制烧汁", sales:"月售312", price:"26", sig:true, img:"roast pork rice plate"},
  {name:"豉油鸡饭", desc:"手撕豉油鸡，皮滑肉嫩", sales:"月售208", price:"22", img:"soy sauce chicken rice"},
  {name:"烧鸭例牌", desc:"炭火烧鸭，皮脆多汁", sales:"月售156", price:"32", img:"roast duck sliced"},
  {name:"例汤", desc:"今日例汤，老火慢炖", sales:"月售421", price:"6", img:"chinese soup bowl"},
  {name:"白灼菜心", desc:"清爽菜心，蒜蓉豉油", sales:"月售189", price:"9", img:"blanched vegetables plate"},
  {name:"冻柠茶", desc:"港式冻柠茶，冰爽解腻", sales:"月售354", price:"8", img:"iced lemon tea glass"}
];
for (const d of dishes) {
  const row = I(list, {type:"frame", name:"菜品行-"+d.name, layout:"horizontal", width:"fill_container", height:"fit_content", gap:12, alignItems:"start", padding:[10,0], animations:[{trigger:"inView", durationMs:360, easing:"emphasizedDecelerate", keyframes:[{offset:0, values:{opacity:0, translateY:16}}, {offset:1, values:{opacity:1, translateY:0}}]}]});
  const imgWrap = I(row, {type:"frame", name:"菜品图容器-"+d.name, layout:"none", width:88, height:88, cornerRadius:10, clipContent:true});
  I(imgWrap, {type:"image", name:"菜品图-"+d.name, width:88, height:88, cornerRadius:10, imageSearchQuery:d.img, imagePrompt:d.name === "例汤" ? "A bowl of slow-simmered Chinese clear soup in a ceramic bowl, overhead shot, warm soft lighting" : d.name === "冻柠茶" ? "A glass of Hong Kong style iced lemon tea on a wooden table, close-up, warm lighting" : "Cantonese siu mei dish " + d.name + " served on a white plate, close-up food photography, warm appetizing lighting"});
  if (d.sig) {
    I(imgWrap, {type:"frame", name:"招牌角标", layout:"horizontal", alignItems:"center", justifyContent:"center", x:0, y:0, width:40, height:20, cornerRadius:0, fill:[{type:"solid", color:"#FF5A1F"}], children:[]});
    // corner badge text
  }
  if (d.sig) {
    const badge = I(imgWrap, {type:"frame", name:"招牌标签", layout:"horizontal", alignItems:"center", justifyContent:"center", x:0, y:0, width:44, height:20, cornerRadius:10, fill:[{type:"solid", color:"#FF5A1F"}]});
    I(badge, {type:"text", name:"招牌文字", content:"招牌", fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:11, fontWeight:600, fill:[{type:"solid", color:"#FFFFFF"}]});
  }
  const info = I(row, {type:"frame", name:"菜品信息-"+d.name, layout:"vertical", width:"fill_container", height:"fit_content", gap:4});
  I(info, {type:"text", name:"菜名-"+d.name, content:d.name, width:"fill_container", textGrowth:"fixed-width", fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:15, fontWeight:600, lineHeight:1.3, fill:[{type:"solid", color:"#21140F"}]});
  I(info, {type:"text", name:"描述-"+d.name, content:d.desc, width:"fill_container", textGrowth:"fixed-width", fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:12, fontWeight:400, lineHeight:1.4, fill:[{type:"solid", color:"#A8917F"}]});
  I(info, {type:"text", name:"月售-"+d.name, content:d.sales, fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:11, fontWeight:400, fill:[{type:"solid", color:"#A8917F"}]});
  const priceRow = I(info, {type:"frame", name:"价格行-"+d.name, layout:"horizontal", width:"fill_container", height:"fit_content", gap:6, alignItems:"end", justifyContent:"space_between"});
  const pr = I(priceRow, {type:"frame", name:"价格-"+d.name, layout:"horizontal", gap:1, alignItems:"start"});
  I(pr, {type:"text", name:"货币符号", content:"¥", fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:12, fontWeight:700, fill:[{type:"solid", color:"#FF5A1F"}]});
  I(pr, {type:"text", name:"价格数值", content:d.price, fontFamily:"Plus Jakarta Sans, Noto Sans SC", fontSize:18, fontWeight:700, letterSpacing:-0.3, fill:[{type:"solid", color:"#21140F"}]});
  const plus = I(priceRow, {type:"frame", name:"加购按钮-"+d.name, layout:"horizontal", alignItems:"center", justifyContent:"center", width:30, height:30, cornerRadius:15, fill:[{type:"solid", color:"#FF5A1F"}], animations:[{trigger:"onTap", durationMs:220, easing:"emphasized", keyframes:[{offset:0, values:{scaleX:1, scaleY:1}}, {offset:0.5, values:{scaleX:0.88, scaleY:0.88}}, {offset:1, values:{scaleX:1, scaleY:1}}]}], events:{onTap:[{toast:"'已加入 ' + " + JSON.stringify(d.name)}]}});
  I(plus, {type:"icon_font", name:"加号图标", iconFontName:"plus", iconFontFamily:"lucide", width:16, height:16, fill:[{type:"solid", color:"#FFFFFF"}]});
}