---
name: mobile-app-predesign
description: Mandatory pre-design step for phone screens — pick the signature moment and write it into the subtask elements
phase: [planning]
trigger:
  keywords: [mobile, phone, ios, android, app, 移动, 手机, 应用, 375]
priority: 8
budget: 600
category: domain
---

MOBILE PRE-DESIGN (perform INTERNALLY, then encode the result in the plan — never as prose):

A phone screen that is merely correct reads as a template. Before decomposing, decide ONE signature moment for this screen from the domain's mood:

- hero card — one painted 24px-radius card inside the rail carrying the title and the primary control (address, search, balance); only when the header owns a control
- hero number — one 40-56px value with a caption; everything else stays small (dashboards, fitness, banking, stats)
- full-bleed image + scrim — image touching three edges, title in white on a bottom gradient scrim (product, recipe, travel, media detail)
- stacked depth — 2-3 large cards with tonal surface and one soft shadow, not thin rows (cards, tickets, plans, wallets)

Then:

1. Pick the style guide whose mood fits that moment (dark / warm / pastel / vibrant), not the generic clean-blue default; a mobile screen always gets a mobile style guide.
2. Write the moment into the `elements` text of the subtask that owns it, in these words: "SIGNATURE MOMENT: <name> — <one concrete sentence>". Vary the pick across screens of one app; hero card is not the default. Every other subtask's `elements` gets "quiet section: tonal surfaces, no accent fills, one hairline max".
3. Do not add a subtask for the status bar; it is pre-inserted.

Output only the plan JSON. Do not explain these steps.
