const sec = I(null, { type: "frame", name: "商家列表", width: "fill_container", height: "fit_content", layout: "vertical", padding: [0, 24, 24, 24], gap: 16 });

// Section header + filter tabs
const head = I(sec, { type: "frame", name: "列表头部", width: "fill_container", height: "fit_content", layout: "horizontal", justifyContent: "space_between", alignItems: "center" });
I(head, { type: "text", name: "区块标题", content: "附近商家", fontFamily: "Plus Jakarta Sans, Noto Sans SC", fontSize: 20, fontWeight: 700, fill: [{ type: "solid", color: "#21140F" }], letterSpacing: -0.3, width: "fit_content", textGrowth: "auto" });
const tabs = I(head, { type: "frame", name: "筛选tab组", width: "fit_content", height: "fit_content", layout: "horizontal", gap: 16, alignItems: "center" });
const tabData = [
  { label: "综合排序", active: true },
  { label: "销量", active: false },
  { label: "距离", active: false }
];
for (const t of tabData) {
  I(tabs, { type: "text", name: "tab-" + t.label, content: t.label, fontFamily: "Plus Jakarta Sans, Noto Sans SC", fontSize: 13, fontWeight: t.active ? 600 : 400, fill: [{ type: "solid", color: t.active ? "#FF5A1F" : "#A8917F" }], width: "fit_content", textGrowth: "auto" });
}

// Store rows
const stores = [
  { name: "老陈家 · 热汤面", meta: "月售 1200+ · 30 分钟", cost: "起送 ¥20 · 配送 ¥3", tags: ["满 25 减 8", "新客立减 5"], q: "hot noodle soup bowl", p: "Steaming bowl of hot noodle soup with scallions in a warm-lit restaurant, overhead shot, appetizing" },
  { name: "咔滋炸鸡工坊", meta: "月售 860 · 35 分钟", cost: "起送 ¥25 · 配送 ¥4", tags: ["满 30 减 10", "折扣菜"], q: "crispy fried chicken plate", p: "Golden crispy fried chicken pieces on a white plate, warm light, appetizing close-up" },
  { name: "一碗盖饭食堂", meta: "月售 720 · 28 分钟", cost: "起送 ¥18 · 配送 ¥3", tags: ["满 25 减 8"], q: "rice bowl with beef", p: "Japanese-style beef rice bowl topped with egg, warm tones, overhead food photography" },
  { name: "甜野甜品饮品", meta: "月售 540 · 40 分钟", cost: "起送 ¥15 · 配送 ¥3", tags: ["新客立减 5"], q: "mango dessert drink", p: "Iced mango dessert drink in a clear cup with fruit toppings, bright warm lighting" },
  { name: "深夜麻辣烫", meta: "月售 980 · 32 分钟", cost: "起送 ¥20 · 配送 ¥4", tags: ["满 30 减 6", "夜宵专送"], q: "spicy malatang bowl", p: "Bubbling spicy malatang hot pot bowl with vegetables and skewers, warm red broth, overhead shot" }
];

const list = I(sec, { type: "frame", name: "商家行列表", width: "fill_container", height: "fit_content", layout: "vertical", gap: 20 });

stores.forEach((s, i) => {
  const card = I(list, {
    type: "frame", name: "商家行-" + s.name, width: "fill_container", height: "fit_content",
    layout: "horizontal", gap: 12, alignItems: "start",
    fill: [{ type: "solid", color: "#FFFFFF" }],
    stroke: { thickness: 1, fill: [{ type: "solid", color: "#F3E7DC" }] },
    cornerRadius: 14, padding: 12, clipContent: true,
    events: { onTap: [{ push: "\"/screen-1\"" }] },
    animations: [{ trigger: "inView", durationMs: 360, easing: "emphasizedDecelerate", delayMs: i * 60, keyframes: [{ offset: 0, values: { opacity: 0, translateY: 24 } }, { offset: 1, values: { opacity: 1, translateY: 0 } }] }]
  });

  I(card, {
    type: "image", name: "商家图-" + s.name, width: 88, height: 88, cornerRadius: 10,
    imageSearchQuery: s.q, imagePrompt: s.p
  });

  const col = I(card, { type: "frame", name: "商家信息", width: "fill_container", height: "fit_content", layout: "vertical", gap: 6 });
  I(col, { type: "text", name: "商家名", content: s.name, fontFamily: "Plus Jakarta Sans, Noto Sans SC", fontSize: 16, fontWeight: 600, fill: [{ type: "solid", color: "#21140F" }], width: "fill_container", textGrowth: "fixed-width", lineHeight: 1.2 });
  I(col, { type: "text", name: "月售配送", content: s.meta, fontFamily: "Plus Jakarta Sans, Noto Sans SC", fontSize: 12, fontWeight: 400, fill: [{ type: "solid", color: "#7A5B48" }], width: "fill_container", textGrowth: "fixed-width", lineHeight: 1.4 });
  I(col, { type: "text", name: "起送配送费", content: s.cost, fontFamily: "Plus Jakarta Sans, Noto Sans SC", fontSize: 12, fontWeight: 400, fill: [{ type: "solid", color: "#A8917F" }], width: "fill_container", textGrowth: "fixed-width", lineHeight: 1.4 });

  const tagRow = I(col, { type: "frame", name: "活动标签行", width: "fit_content", height: "fit_content", layout: "horizontal", gap: 6 });
  for (const tg of s.tags) {
    I(tagRow, { type: "text", name: "标签-" + tg, content: tg, fontFamily: "Plus Jakarta Sans, Noto Sans SC", fontSize: 11, fontWeight: 500, fill: [{ type: "solid", color: "#FF5A1F" }], width: "fit_content", textGrowth: "auto", height: 18, cornerRadius: 4, stroke: { thickness: 0.5, fill: [{ type: "solid", color: "#FF5A1F33" }] }, fill: [{ type: "solid", color: "#FFF0E3" }], padding: [2, 6] });
  }
});
