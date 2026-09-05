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

A phone screen that is merely correct reads as a template. Before decomposing:

1. Pick the mobile style guide whose mood fits the brief (dark / warm / pastel / vibrant / editorial), not the generic clean-blue default; a mobile screen always gets a mobile style guide. Read its "Key aesthetics" — that is the signature treatment for this screen.
2. Decide which section carries it: normally the first content section (greeting / title / primary control); a media-led screen (product, recipe, workout, travel) carries it on full-bleed media under the status bar; a numbers screen (balance, calories, tasks) carries it on one display number. Never a separate "sheet" floating over a header.
3. Write it into that subtask's `elements` text as "SIGNATURE MOMENT: <guide key aesthetic in one sentence> — <how this section shows it>". Every other subtask's `elements` gets "quiet section: tonal surfaces, no accent fills, one hairline max".
4. Do not add a subtask for the status bar; it is pre-inserted.

Output only the plan JSON. Do not explain these steps.
