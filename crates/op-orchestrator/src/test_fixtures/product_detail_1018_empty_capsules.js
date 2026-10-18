```javascript
const sec = I(null, {type:"frame", name:"规格选择区", width:"fill_container", height:"fit_content", layout:"vertical", padding:[0,24,8,24], gap:16});

I(sec, {type:"text", name:"规格标签", content:"规格", fontFamily:"Plus Jakarta Sans", fontSize:16, fontWeight:600, fill:[{type:"solid", color:"#1C1917"}], width:"fit_content", height:"fit_content"});

const chipsRow = I(sec, {type:"frame", name:"规格胶囊组", layout:"horizontal", width:"fill_container", height:"fit_content", gap:12, alignItems:"center"});

const specs = [
  {value:"350ml", label:"350ml"},
  {value:"450ml", label:"450ml"},
  {value:"giftbox", label:"礼盒装"}
];

for (const s of specs) {
  I(chipsRow, {type:"frame", name:"规格胶囊-" + s.value, layout:"horizontal", alignItems:"center", justifyContent:"center", padding:[10,20], cornerRadius:9999,
    fill:[{type:"solid", color:"#FFFFFF"}],
    stroke:{thickness:1.5, fill:[{type:"solid", color:"#E7E5E4"}]},
    events:{onTap:[
      {set:{"$app.selectedSpec":"'"+s.value+"'"}},
      {animate:{target:"spec-chip-" + s.value, property:"scaleX", from:0.94, to:1, durationMs:200, easing:"ease_out", fillMode:"both"}},
      {animate:{target:"spec-chip-" + s.value, property:"scaleY", from:0.94, to:1, durationMs:200, easing:"ease_out", fillMode:"both"}}
    ]},
    children: undefined});
}
```
