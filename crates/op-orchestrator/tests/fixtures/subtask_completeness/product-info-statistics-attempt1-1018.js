const sec = I(null, {type:"frame", name:"价格与商品信息", width:"fill_container", height:"fit_content", layout:"vertical", padding:[16,24,0,24], gap:12});

I(sec, {type:"text", name:"商品标题", content:"云感纯棉圆领 T 恤", fontFamily:"SF Pro", fontSize:20, fontWeight:600, fill:[{type:"solid",color:"#111827"}], width:"fill_container", textGrowth:"fixed-width", lineHeight:1.2, letterSpacing:-0.3,
  animations:[{trigger:"mount",durationMs:360,easing:"emphasizedDecelerate",keyframes:[{offset:0,values:{opacity:0,translateY:10}},{offset:1,values:{opacity:1,translateY:0}}]}]});

const priceRow = I(sec, {type:"frame", name:"价格行", layout:"horizontal", width:"fill_container", alignItems:"end", gap:8, height:"fit_content"});

const numBox = I(priceRow, {type:"frame", name:"价格数字容器", layout:"none", width:"fit_content", height:"fit_content", clipContent:true});
I(numBox, {type:"text", name:"价格-元符号", content:"¥", fontFamily:"SF Pro", fontSize:20, fontWeight:700, fill:[{type:"solid",color:"#2563EB"}], width:"fit_content", x:0, y:10, textGrowth:"auto"});
I(numBox, {type:"text", name:"价格-数字", content:"129", fontFamily:"SF Pro", fontSize:34, fontWeight:700, fill:[{type:"solid",color:"#2563EB"}], width:"fit_content", x:14, y:0, textGrowth:"auto", letterSpacing:-0.5,
  animations:[{trigger:"mount",durationMs:500,easing:"emphasizedDecelerate",keyframes:[{offset:0,values:{opacity:0,translateY:22}},{offset:1,values:{opacity:1,translateY:0}}]}]});

I(priceRow, {type:"text", name:"划线原价", content:"¥169", fontFamily:"SF Pro", fontSize:14, fontWeight:400, fill:[{type:"solid",color:"#9CA3AF"}], width:"fit_content", textGrowth:"auto", fontStyle:"normal", textDecoration:"line-through"});

const tag = I(priceRow, {type:"frame", name:"限时标签", layout:"horizontal", padding:[3,8], cornerRadius:8, fill:[{type:"solid",color:"#2563EB15"}], alignItems:"center", height:"fit_content"});
I(tag, {type:"text", name:"限时标签文字", content:"限时直降", fontFamily:"SF Pro", fontSize:11, fontWeight:500, fill:[{type:"solid",color:"#2563EB"}], width:"fit_content", textGrowth:"auto"});

const metaRow = I(sec, {type:"frame", name:"销量评分行", layout:"horizontal", width:"fill_container", alignItems:"center", gap:6, height:"fit_content"});
const meta = [
  {icon:"star", label:"4.8 分"},
  {icon:null, label:"1,236 条评价"},
  {icon:null, label:"已售 3,200+"}
];
meta.forEach(function(m, i){
  if(i>0){ I(metaRow, {type:"text", name:"分隔点"+i, content:"·", fontFamily:"SF Pro", fontSize:13, fill:[{type:"solid",color:"#D1D5DB"}], width:"fit_content", textGrowth:"auto"}); }
  if(m.icon){ I(metaRow, {type:"icon_font", name:"评分星标", iconFontName:"star", iconFontFamily:"lucide", width:14, height:14, fill:[{type:"solid",color:"#F59E0B"}]}); }
  I(metaRow, {type:"text", name:"元数据"+i, content:m.label, fontFamily:"SF Pro", fontSize:13, fontWeight:400, fill:[{type:"solid",color:"#6B7280"}], width:"fit_content", textGrowth:"auto"});
});

const coupon = I(sec, {type:"frame", name:"优惠券提示条", layout:"horizontal", width:"fill_container", height:"fit_content", padding:[10,12], cornerRadius:12, fill:[{type:"solid",color:"#F9FAFB"}], alignItems:"center", justifyContent:"space_between",
  stroke:{thickness:1, fill:[{type:"solid",color:"#F3F4F6"}]},
  animations:[{trigger:"mount",durationMs:400,delayMs:120,easing:"emphasizedDecelerate",keyframes:[{offset:0,values:{opacity:0,translateY:10}},{offset:1,values:{opacity:1,translateY:0}}]}]});
const couponLeft = I(coupon, {type:"frame", name:"券文案组", layout:"horizontal", gap:6, alignItems:"center", height:"fit_content"});
I(couponLeft, {type:"icon_font", name:"券图标", iconFontName:"tag", iconFontFamily:"lucide", width:16, height:16, fill:[{type:"solid",color:"#2563EB"}]});
I(couponLeft, {type:"text", name:"券文案", content:"领券满 199 减 20", fontFamily:"SF Pro", fontSize:13, fontWeight:500, fill:[{type:"solid",color:"#111827"}], width:"fit_content", textGrowth:"auto"});
I(coupon, {type:"icon_font", name:"券箭头", iconFontName:"chevron-right", iconFontFamily:"lucide", width:16, height:16, fill:[{type:"solid",color:"#9CA3AF"}]});
