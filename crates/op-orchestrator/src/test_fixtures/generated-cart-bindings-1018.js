const sec = I(null, {type:"frame", name:"购物车页 · 商品列表", width:"fill_container", height:"fit_content", layout:"vertical", padding:[0,24], gap:12, fill:[]});

const items = [
  {id:"product-1", name:"轻氧保温随行杯 450ml", spec:"已选：月白 / 450ml", price:"¥129", query:"insulated water bottle", prompt:"minimal matte white insulated bottle standing on a soft beige studio backdrop, clean e-commerce product photo", checked:true, qty:1},
  {id:"product-2", name:"同系列咖啡滤杯", spec:"已选：燕麦 / 陶瓷", price:"¥89", query:"ceramic pour-over dripper", prompt:"matte ceramic pour-over coffee dripper on a light neutral backdrop, soft studio lighting, clean e-commerce product photo", checked:false, qty:1}
];

for (const it of items) {
  const card = I(sec, {type:"frame", name:"cart-item-"+it.id, width:"fill_container", height:"fit_content", layout:"horizontal", padding:12, gap:12, alignItems:"center", cornerRadius:12, fill:[{type:"solid", color:"#FFFFFF"}], stroke:{thickness:1, fill:[{type:"solid", color:"#E7E5E4"}]}});

  const chkWrap = I(card, {type:"frame", name:"check-wrap-"+it.id, width:44, height:44, layout:"horizontal", alignItems:"center", justifyContent:"center", fill:[]});
  I(chkWrap, {type:"checkbox", name:"item-check-"+it.id, checked:it.checked, fill:[{type:"solid", color:"#F97316"}], stroke:{thickness:2, fill:[{type:"solid", color:"#E7E5E4"}]}, cornerRadius:9999});

  const thumb = I(card, {type:"frame", name:"thumb-wrap-"+it.id, width:80, height:80, layout:"none", clipContent:true, cornerRadius:8, fill:[{type:"solid", color:"#FFF7ED"}]});
  I(thumb, {type:"image", name:"item-image-"+it.id, width:80, height:80, cornerRadius:8, imageSearchQuery:it.query, imagePrompt:it.prompt});

  const info = I(card, {type:"frame", name:"item-info-"+it.id, width:"fill_container", height:"fit_content", layout:"vertical", gap:6, fill:[]});
  I(info, {type:"text", name:"item-name-"+it.id, content:it.name, fontFamily:"Plus Jakarta Sans", fontSize:15, fontWeight:600, fill:[{type:"solid", color:"#1C1917"}], width:"fill_container", textGrowth:"fixed-width", lineHeight:1.3});
  I(info, {type:"text", name:"item-spec-"+it.id, content:it.spec, fontFamily:"Plus Jakarta Sans", fontSize:12, fontWeight:400, fill:[{type:"solid", color:"#A8A29E"}], width:"fill_container", textGrowth:"fixed-width"});

  const bottom = I(info, {type:"frame", name:"item-bottom-"+it.id, width:"fill_container", height:"fit_content", layout:"horizontal", justifyContent:"space_between", alignItems:"center", fill:[]});
  I(bottom, {type:"text", name:"item-price-"+it.id, content:it.price, fontFamily:"Plus Jakarta Sans", fontSize:17, fontWeight:700, fill:[{type:"solid", color:"#EA580C"}], width:"fit_content", textGrowth:"auto"});

  const stepper = I(bottom, {type:"frame", name:"stepper-"+it.id, width:"fit_content", height:32, layout:"horizontal", alignItems:"center", cornerRadius:8, fill:[{type:"solid", color:"#FFF7ED"}], stroke:{thickness:1, fill:[{type:"solid", color:"#F5F5F4"}]}, fill2:undefined});
  const minusBtn = I(stepper, {type:"frame", name:"qty-minus-"+it.id, width:32, height:32, layout:"horizontal", alignItems:"center", justifyContent:"center", cornerRadius:8, fill:[], events:{onTap:[{if:{expr:"$app.qty_"+it.id+" > 1", then:[{set:{["$app.qty_"+it.id]:"$app.qty_"+it.id+" - 1"}}]}}]}});
  I(minusBtn, {type:"icon_font", name:"minus-icon-"+it.id, iconFontName:"minus", iconFontFamily:"lucide", width:14, height:14, fill:[{type:"solid", color:"#57534E"}]});
  I(stepper, {type:"text", name:"qty-value-"+it.id, content:String(it.qty), fontFamily:"Plus Jakarta Sans", fontSize:14, fontWeight:600, fill:[{type:"solid", color:"#1C1917"}], width:28, height:32, textAlign:"center", bindings:{content:"String($app.qty_"+it.id+")"}});
  const plusBtn = I(stepper, {type:"frame", name:"qty-plus-"+it.id, width:32, height:32, layout:"horizontal", alignItems:"center", justifyContent:"center", cornerRadius:8, fill:[], events:{onTap:[{set:{["$app.qty_"+it.id]:"$app.qty_"+it.id+" + 1"}}]}});
  I(plusBtn, {type:"icon_font", name:"plus-icon-"+it.id, iconFontName:"plus", iconFontFamily:"lucide", width:14, height:14, fill:[{type:"solid", color:"#F97316"}]});
}
