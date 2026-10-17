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

Pick ONE archetype per screen; apply its style to the protagonist. In the owning subtask's `elements`, write `ARCHETYPE: <name> — protagonist <what>, first screen <x>%, <spatial relation>, imagery <rule>`. Assign each title and meta row ONE owner; typography applies only to copy that section owns. If a separate title/meta section exists, the photo hero contains only media and controls, without repeated titles or invented stats.

Explicit first-screen content comes first: reserve 62px system chrome, navigation, gaps and every required item BEFORE imagery. Percentages are starting points; reduce the hero when needed. Long/CJK titles usually use 28–36px with balanced lines; 48px suits short display copy/figures. Do not plan a status bar; it is pre-inserted.

ACTIONABLE DETAILS: when a photo introduces a repeated list and a pinned action (training exercises, itinerary stops, lesson steps), reserve the title/meta, list heading and at least three readable rows before sizing the photo. Start around 160–208px on an 812px phone; keep the rest of the list scrollable, never squeeze every row into one screenshot. Photo-led browsing, galleries and explicit large-image/reference layouts retain their requested composition. A large photograph is not mandatory merely because a page is called a detail screen.

Supporting subtasks start with `quiet section: secondary visual weight; retain legible chart colors and active states`, then their parts list. Never mark the protagonist quiet.

House defaults for an 812px screen (body 16px):

| name | purpose | protagonist / relation | imagery |
| image-led | photo-led browsing/product showcase | photo 35–45% only when imagery is the main task; actionable details use the content budget above | one relevant photo; overlay copy only if owned here |
| ledger | banking/wallet/stats | figure card 25–30%, quiet list below | figures lead; no decorative hero |
| route-map | ride/delivery/travel | map 40–50%, pinned controls, sheet below | map template, never photo |
| timeline | calendar/schedule | grid/time axis 30–40%, accent now, split | no decorative hero |
| content-stream | social/feed | first card 30% larger, then two-column tiles | coherent hero media, restrained tiles |
| progress | fitness/habits | ring/bar 25–35%, figure inside, plan tiles below | figure first; optional illustration |
