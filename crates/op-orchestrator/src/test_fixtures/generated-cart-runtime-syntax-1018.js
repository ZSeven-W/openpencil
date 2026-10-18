const sec = I(null, {type:"frame", name:"购物车页", width:"fill_container", height:"fit_content", layout:"vertical", screen:"/screen-1", fill:[]});

// state
I(sec, {type:"frame", name:"cart-state-host", width:0, height:0, layout:"none",
  state:{ qty1:{type:"int",default:1}, qty2:{type:"int",default:1},
          sel1:{type:"bool",default:true}, sel2:{type:"bool",default:true},
          allSel:{type:"bool",default:true} },
  events:{}});

// ---- 顶部导航栏 ----
const nav = I(sec, {type:"frame", name:"顶部导航栏", width:"fill_container", height:56, layout:"horizontal", padding:[0,16], alignItems:"center", justifyContent:"space_between"});
const backBtn = I(nav, {type:"frame", name:"返回按钮", width:40, height:40, layout:"horizontal", alignItems:"center", justifyContent:"center", cornerRadius:20, fill:[{type:"solid",color:"#F5F5F4"}],
  events:{onTap:[{pop:null}]}});
I(backBtn, {type:"icon_font", iconFontName:"chevron-left", iconFontFamily:"lucide", width:22, height:22, fill:[{type:"solid",color:"#1C1917"}]});
I(nav, {type:"text", name:"导航标题", content:"购物车", fontFamily:"Plus Jakarta Sans", fontSize:18, fontWeight:600, fill:[{type:"solid",color:"#1C1917"}], width:"fit_content", textGrowth:"auto"});
const mgmt = I(nav, {type:"frame", name:"管理按钮", width:40, height:40, layout:"horizontal", alignItems:"center", justifyContent:"center", cornerRadius:20, fill:[{type:"solid",color:"#F5F5F4"}], events:{onTap:[{toast:"进入管理模式"}]}});
I(mgmt, {type:"text", content:"管理", fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:500, fill:[{type:"solid",color:"#57534E"}], width:"fit_content", textGrowth:"auto"});

// ---- 商品列表 ----
const list = I(sec, {type:"frame", name:"商品列表", width:"fill_container", height:"fit_content", layout:"vertical", padding:[8,24,0,24], gap:12});

const products = [
  {id:"p1", name:"轻氧陶瓷马克杯", spec:"奶油白 · 350ml", price:89, qty:"qty1", sel:"sel1",
   q:"ceramic mug cream", prompt:"奶油白陶瓷马克杯放在浅色木桌上的产品照，柔和自然光，简洁背景"},
  {id:"p2", name:"山野冷萃挂耳咖啡", spec:"中度烘焙 · 10袋装", price:39, qty:"qty2", sel:"sel2",
   q:"drip coffee kraft packet", prompt:"牛皮纸包装挂耳咖啡与一杯黑咖啡，浅色桌面，柔和自然光，简洁产品照"}
];

for (const p of products) {
  const card = I(list, {type:"frame", name:"商品卡-"+p.id, width:"fill_container", height:"fit_content", layout:"horizontal", padding:16, gap:12, cornerRadius:16, fill:[{type:"solid",color:"#FFFFFF"}],
    stroke:{thickness:1, fill:[{type:"solid",color:"#E7E5E4"}]}, alignItems:"start"});
  // 勾选圆点
  const dot = I(card, {type:"frame", name:"勾选-"+p.id, width:22, height:22, cornerRadius:11, layout:"horizontal", alignItems:"center", justifyContent:"center",
    fill:[{type:"solid",color:"#F97316"}],
    bindings:{fill:"$state."+p.sel+" ? '#F97316' : '#FFFFFF'"},
    stroke:{thickness:1.5, fill:[{type:"solid",color:"#D6D3D1"}]},
    events:{onTap:[{toggle:"$state."+p.sel},{set:{"$state.allSel":"$state.sel1 && $state.sel2"}}]}});
  I(dot, {type:"icon_font", name:"勾选图标", iconFontName:"check", iconFontFamily:"lucide", width:13, height:13,
    fill:[{type:"solid",color:"#FFFFFF"}],
    bindings:{opacity:"$state."+p.sel+" ? 1 : 0"}});
  // 缩略图
  const thumb = I(card, {type:"frame", name:"缩略图-"+p.id, width:84, height:84, cornerRadius:12, clipContent:true, fill:[{type:"solid",color:"#FFF7ED"}]});
  I(thumb, {type:"image", name:"商品图-"+p.id, width:84, height:84, cornerRadius:12, imageSearchQuery:p.q, imagePrompt:p.prompt});
  // 信息列
  const info = I(card, {type:"frame", name:"信息列-"+p.id, width:"fill_container", height:"fit_content", layout:"vertical", gap:8});
  I(info, {type:"text", name:"商品名-"+p.id, content:p.name, fontFamily:"Plus Jakarta Sans", fontSize:15, fontWeight:600, fill:[{type:"solid",color:"#1C1917"}], width:"fill_container", textGrowth:"fixed-width", lineHeight:1.3});
  // 规格胶囊
  const specPill = I(info, {type:"frame", name:"规格胶囊-"+p.id, width:"fit_content", height:26, layout:"horizontal", alignItems:"center", padding:[0,10], gap:4, cornerRadius:13, fill:[{type:"solid",color:"#F5F5F4"}]});
  I(specPill, {type:"text", content:p.spec, fontFamily:"Plus Jakarta Sans", fontSize:12, fontWeight:400, fill:[{type:"solid",color:"#57534E"}], width:"fit_content", textGrowth:"auto"});
  I(specPill, {type:"icon_font", iconFontName:"chevron-down", iconFontFamily:"lucide", width:13, height:13, fill:[{type:"solid",color:"#A8A29E"}]});
  // 价格 + 步进器
  const prow = I(info, {type:"frame", name:"价格行-"+p.id, width:"fill_container", height:"fit_content", layout:"horizontal", alignItems:"center", justifyContent:"space_between"});
  I(prow, {type:"text", name:"价格-"+p.id, content:"¥" + p.price.toFixed(2), fontFamily:"Plus Jakarta Sans", fontSize:17, fontWeight:700, fill:[{type:"solid",color:"#EA580C"}], width:"fit_content", textGrowth:"auto"});
  const stepper = I(prow, {type:"frame", name:"数量步进器-"+p.id, width:"fit_content", height:30, layout:"horizontal", alignItems:"center", cornerRadius:15, fill:[{type:"solid",color:"#F5F5F4"}], gap:0});
  const minus = I(stepper, {type:"frame", name:"减-"+p.id, width:32, height:30, layout:"horizontal", alignItems:"center", justifyContent:"center",
    events:{onTap:[{if:{expr:"$state."+p.qty+" > 1", then:[{set:{["$state."+p.qty]:"$state."+p.qty+" - 1"}}]}}]}});
  I(minus, {type:"icon_font", iconFontName:"minus", iconFontFamily:"lucide", width:14, height:14, fill:[{type:"solid",color:"#57534E"}]});
  const qtyBox = I(stepper, {type:"frame", name:"数量-"+p.id, width:36, height:30, layout:"horizontal", alignItems:"center", justifyContent:"center"});
  I(qtyBox, {type:"text", name:"数量文本-"+p.id, content:"1", fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:600, fill:[{type:"solid",color:"#1C1917"}], width:"fit_content", textGrowth:"auto",
    bindings:{content:"\"\" + $state."+p.qty}});
  const plus = I(stepper, {type:"frame", name:"加-"+p.id, width:32, height:30, layout:"horizontal", alignItems:"center", justifyContent:"center",
    events:{onTap:[{set:{["$state."+p.qty]:"$state."+p.qty+" + 1"}}]}});
  I(plus, {type:"icon_font", iconFontName:"plus", iconFontFamily:"lucide", width:14, height:14, fill:[{type:"solid",color:"#57534E"}]});
}

// ---- 金额汇总卡 ----
const sum = I(sec, {type:"frame", name:"金额汇总卡", width:"fill_container", height:"fit_content", layout:"vertical", padding:[20,24,0,24], gap:12});
const sumCard = I(sum, {type:"frame", name:"汇总卡体", width:"fill_container", height:"fit_content", layout:"vertical", padding:16, gap:12, cornerRadius:16, fill:[{type:"solid",color:"#FFF7ED"}]});
const rows = [
  {label:"商品合计", expr:"\"¥\" + (($state.sel1 ? $state.qty1 * 89 : 0) + ($state.sel2 ? $state.qty2 * 39 : 0)).toFixed(2)", bold:true},
  {label:"运费", expr:"(($state.sel1 ? $state.qty1 * 89 : 0) + ($state.sel2 ? $state.qty2 * 39 : 0)) >= 99 ? '免运费' : '¥6.00（满 ¥99 免运费）'", bold:false},
];
for (const r of rows) {
  const rr = I(sumCard, {type:"frame", name:"汇总行-"+r.label, width:"fill_container", height:"fit_content", layout:"horizontal", justifyContent:"space_between", alignItems:"center"});
  I(rr, {type:"text", content:r.label, fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:400, fill:[{type:"solid",color:"#57534E"}], width:"fit_content", textGrowth:"auto"});
  I(rr, {type:"text", content:"—", fontFamily:"Plus Jakarta Sans", fontSize:r.bold?17:14, fontWeight:r.bold?700:400, fill:[{type:"solid",color:r.bold?"#1C1917":"#57534E"}], width:"fit_content", textGrowth:"auto",
    bindings:{content:r.expr}});
}
const promo = I(sumCard, {type:"frame", name:"优惠提示行", width:"fill_container", height:"fit_content", layout:"horizontal", alignItems:"center", gap:6, padding:[8,12], cornerRadius:10, fill:[{type:"solid",color:"#FEF3C7"}]});
I(promo, {type:"icon_font", iconFontName:"tag", iconFontFamily:"lucide", width:14, height:14, fill:[{type:"solid",color:"#EA580C"}]});
I(promo, {type:"text", content:"已享新客立减优惠，结算时自动抵扣", fontFamily:"Plus Jakarta Sans", fontSize:12, fontWeight:500, fill:[{type:"solid",color:"#92400E"}], width:"fill_container", textGrowth:"fixed-width"});

// ---- 底部结算条 ----
const bottom = I(sec, {type:"frame", name:"底部结算条", width:"fill_container", height:"fit_content", layout:"horizontal", padding:[16,24,32,24], alignItems:"center", gap:12,
  fill:[{type:"solid",color:"#FFFFFF"}], stroke:{thickness:{top:1}, fill:[{type:"solid",color:"#E7E5E4"}]}});
// 全选
const allRow = I(bottom, {type:"frame", name:"全选", width:"fit_content", height:"fit_content", layout:"horizontal", alignItems:"center", gap:8,
  events:{onTap:[{toggle:"$state.allSel"},{set:{"$state.sel1":"$state.allSel","$state.sel2":"$state.allSel"}}]}});
const allDot = I(allRow, {type:"frame", name:"全选圆点", width:22, height:22, cornerRadius:11, layout:"horizontal", alignItems:"center", justifyContent:"center",
  fill:[{type:"solid",color:"#F97316"}],
  bindings:{fill:"$state.allSel ? '#F97316' : '#FFFFFF'"},
  stroke:{thickness:1.5, fill:[{type:"solid",color:"#D6D3D1"}]}});
I(allDot, {type:"icon_font", iconFontName:"check", iconFontFamily:"lucide", width:13, height:13, fill:[{type:"solid",color:"#FFFFFF"}],
  bindings:{opacity:"$state.allSel ? 1 : 0"}});
I(allRow, {type:"text", content:"全选", fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:500, fill:[{type:"solid",color:"#57534E"}], width:"fit_content", textGrowth:"auto"});
// 合计
const totalCol = I(bottom, {type:"frame", name:"合计列", width:"fill_container", height:"fit_content", layout:"vertical", gap:2});
const tRow = I(totalCol, {type:"frame", name:"合计行", width:"fit_content", height:"fit_content", layout:"horizontal", alignItems:"end", gap:4});
I(tRow, {type:"text", content:"合计", fontFamily:"Plus Jakarta Sans", fontSize:12, fontWeight:400, fill:[{type:"solid",color:"#A8A29E"}], width:"fit_content", textGrowth:"auto"});
I(tRow, {type:"text", content:"¥0.00", fontFamily:"Plus Jakarta Sans", fontSize:20, fontWeight:700, fill:[{type:"solid",color:"#EA580C"}], width:"fit_content", textGrowth:"auto",
  animations:[{trigger:"mount",durationMs:600,easing:"emphasizedDecelerate",keyframes:[{offset:0,values:{opacity:0,translateY:10}},{offset:1,values:{opacity:1,translateY:0}}]}],
  bindings:{content:"\"¥\" + (($state.sel1 ? $state.qty1 * 89 : 0) + ($state.sel2 ? $state.qty2 * 39 : 0)).toFixed(2)"}});
// 去结算
const cta = I(bottom, {type:"frame", name:"去结算按钮", width:120, height:48, layout:"horizontal", alignItems:"center", justifyContent:"center", cornerRadius:12, fill:[{type:"solid",color:"#F97316"}],
  animations:[{trigger:"mount",durationMs:400,easing:"emphasizedDecelerate",keyframes:[{offset:0,values:{opacity:0,translateY:12}},{offset:1,values:{opacity:1,translateY:0}}]}],
  events:{onTap:[{animate:{target:"cta-icon",property:"scaleX",from:1,to:0.92,durationMs:120,easing:"ease_out",iterations:2,direction:"alternate",fillMode:"both"}},
                  {animate:{target:"cta-icon",property:"scaleY",from:1,to:0.92,durationMs:120,easing:"ease_out",iterations:2,direction:"alternate",fillMode:"both"}},
                  {toast:"跳转结算流程"}]}});
I(cta, {type:"text", name:"cta-icon", content:"去结算", fontFamily:"Plus Jakarta Sans", fontSize:16, fontWeight:600, fill:[{type:"solid",color:"#FFFFFF"}], width:"fit_content", textGrowth:"auto"});