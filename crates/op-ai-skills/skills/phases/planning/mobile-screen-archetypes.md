---
name: mobile-screen-archetypes
description: Planning-phase composition choices for phone screens — choose one protagonist, area budget, and spatial relation per screen
phase: [planning]
trigger:
  keywords: [mobile, phone, ios, android, app, 移动, 手机, 应用, 375]
priority: 8
budget: 800
category: domain
---

MOBILE SCREEN ARCHETYPES (perform internally; output only plan JSON):

Pick ONE archetype per screen; apply its style to the protagonist. In the owning subtask's `elements`, write `ARCHETYPE: <name> — protagonist <what>, first screen <x>%, <spatial relation>, display <n>px / body <n>px, imagery <rule>`, using the selected row's values. Assign each title and meta row ONE owner; typography applies only to copy that section owns. If a separate title/meta section exists, the photo hero contains only media and controls, without repeated titles or invented stats.

Explicit first-screen content comes first: reserve 62px system chrome, navigation, gaps and every required item BEFORE imagery. Percentages are starting points; reduce the hero when needed. Long/CJK titles usually use 28–36px with balanced lines; 48px suits short display copy/figures. Do not plan a status bar; it is pre-inserted.

ACTIONABLE DETAILS: when a photo introduces a repeated list and a pinned action (training exercises, itinerary stops, lesson steps), keep the rest of the list scrollable; never squeeze every row into one screenshot.

Supporting subtasks start with `quiet section: secondary visual weight; retain legible chart colors and active states`, then their parts list. Never mark the protagonist quiet.

House defaults for an 812px screen (body 16px):

| name | purpose | protagonist / relation | imagery |
| image-led | photo-led detail/product/workout browsing | full-bleed photo 35–45%, title on scrim; overlap-with-scrim; display 48px / body 16px | one relevant photo; overlay copy only if owned here |
| ledger | banking/wallet/stats | figure card 25–30%, quiet list below; display 48px / body 16px | figures lead; no decorative hero |
| route-map | ride/delivery/travel | map 40–50%, pinned controls, sheet below; display 32px / body 16px | map template, never photo |
| timeline | calendar/schedule | grid/time axis 30–40%, accent now, split; display 32px / body 16px | no decorative hero |
| content-stream | social/feed | first card 30% larger, then two-column tiles; display 36px / body 16px | coherent hero media, restrained tiles |
| progress | fitness/habits | ring/bar 25–35%, figure inside, plan tiles below; display 48px / body 16px | figure first; optional illustration |
