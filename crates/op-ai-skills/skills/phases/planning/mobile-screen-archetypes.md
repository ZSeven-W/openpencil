---
name: mobile-screen-archetypes
description: Planning-phase composition choices for phone screens — choose one protagonist, area budget, and spatial relation per screen
phase: [planning]
trigger:
  keywords: [mobile, phone, ios, android, app, 移动, 手机, 应用, 375]
priority: 8
budget: 600
category: domain
---

MOBILE SCREEN ARCHETYPES (perform internally; output only plan JSON):

A phone screen gets one composition, not a parts list. Select a style guide, read its Key aesthetics, and apply them to the protagonist. Pick exactly ONE archetype per screen. P/S/N means protagonist/supporting/navigation as percentages of the 812px first screen; `hd` means house default. Row sources (`HIG+M3`): Apple HIG “Layout › Visual hierarchy”, “Typography › Conveying hierarchy”, “Images”; Google Material 3 Expressive research summary on shape/size/colour grouping.

| name | when | protagonist | P/S/N of 812px | relation | type display/body | imagery | source |
| image-led | detail/recipe/product/workout | full-bleed image 35–45%, title on scrim | 35–45/40–50/10–15% hd | overlap-with-scrim | 48/16px hd | relevant image; no second hero | HIG+M3; observed in ADA 2025 Mela / Lumy |
| ledger | banking/wallet/stats home | hero figure card 25–30%, tabular figures, quiet list | 25–30/50–60/10–15% hd | stack | 48/16px hd | no decorative hero; figures lead | HIG+M3 |
| route-map | ride/delivery tracking/travel | map/route block 40–50%, pinned controls, sheet below | 40–50/35–45/10–15% hd | stack | 32/16px hd | map placeholder, never photo | HIG+M3 |
| timeline | calendar/schedule/itinerary | dense grid/time axis 30–40%, accent “now” | 30–40/45–55/10–15% hd | split | 32/16px hd | no decorative hero imagery | HIG+M3; observed in ADA 2025 Mela / Lumy |
| content-stream | social/video/feed | first card is a hero card 30%+ larger, then a rhythm of 2-column tiles | 30–40/45–55/10–15% hd | stack | 36/16px hd | coherent hero media, restrained tiles | HIG+M3 |
| progress | fitness/habit/goals | ring/bar figure 25–35% with number inside, plan tiles below | 25–35/50–60/10–15% hd | stack | 48/16px hd | no photo; figure first, optional illustration | HIG+M3 |

CONTRACT: The planner picks ONE archetype per screen. In the owning subtask’s `elements`, write an 80–120 token summary exactly in this shape: `ARCHETYPE: <name> — protagonist <what>, first screen <x>% , <spatial relation>, display <n>px / body <n>px, imagery <rule>`. Use the selected row’s values. Every other subtask’s `elements` starts with `quiet section: tonal surfaces, no accent fills, one hairline max`; keep its existing parts list AFTER the quiet line. Do not add a status bar subtask; it is pre-inserted.
