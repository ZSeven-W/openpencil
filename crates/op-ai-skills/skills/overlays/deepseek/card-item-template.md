---
name: card-item-template
description: DeepSeek experiment overlay — full worked example of the cards ITEM TEMPLATE rule (define one item structure, copy it N times)
phase: [generation]
model_families: [deepseek]
trigger:
  keywords: [card, 卡片, 知识卡, 封面, 小红书, cover]
priority: 27
budget: 600
category: domain
---

CARD ITEM TEMPLATE — WORKED EXAMPLE

The compact rule lives in the `cards` skill (ITEM TEMPLATE). Worked skeleton
for a three-item knowledge card: the item structure is authored ONCE; the
three copies differ only in sequence number and copy text:

```
frame(name="card", width=1080, height=1440, layout="vertical", padding=48, gap=24)  -- card board (cards rule 1: margins owned by the root)
├── text(content="早起三件事", fontSize=52, fontWeight=700)
├── frame(name="item-01", width="fill_container", height="fit_content", layout="horizontal", gap=16, alignItems="center", padding=20, cornerRadius=16, stroke={thickness:1, fill:[{type:"solid", color:"$color-border"}]})
│   ├── text(content="01", fontSize=28, fontWeight=700, fill:[{type:"solid", color:"$color-accent"}])
│   ├── text(content="阳光", fontSize=24, fontWeight=600)
│   └── text(content="拉开窗帘，站到窗边两分钟", fontSize=20, fill:[{type:"solid", color:"$color-text-muted"}])
├── frame(name="item-02", width="fill_container", height="fit_content", layout="horizontal", gap=16, alignItems="center", padding=20, cornerRadius=16, stroke={thickness:1, fill:[{type:"solid", color:"$color-border"}]})
│   ├── text(content="02", fontSize=28, fontWeight=700, fill:[{type:"solid", color:"$color-accent"}])
│   ├── text(content="喝水", fontSize=24, fontWeight=600)
│   └── text(content="一杯温水，唤醒身体", fontSize=20, fill=[{type:"solid", color:"$color-text-muted"}])
└── frame(name="item-03", width="fill_container", height="fit_content", layout="horizontal", gap=16, alignItems="center", padding=20, cornerRadius=16, stroke={thickness:1, fill:[{type:"solid", color:"$color-border"}]})
    ├── text(content="03", fontSize=28, fontWeight=700, fill:[{type:"solid", color:"$color-accent"}])
    ├── text(content="计划", fontSize=24, fontWeight=600)
    └── text(content="写下今天最重要的一件事", fontSize=20, fill=[{type:"solid", color:"$color-text-muted"}])
```

item-02 / item-03 are byte-level copies of item-01 with ONLY the sequence
number and copy strings changed; for five items, copy the skeleton five times.

HARD WORDING:

- `MUST define the item structure ONCE and copy it for every item — NEVER invent a fresh structure, child order, or ornament per item.`
- `ORNAMENT DISCIPLINE: the numbering/ornament treatment (here the 01/02/03 accent lead) repeats VERBATIM on every item; only the item's copy text changes.`

