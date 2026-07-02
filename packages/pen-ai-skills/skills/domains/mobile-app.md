---
name: mobile-app
description: Mobile app three-section architecture with enforced Blueprint
phase: [generation]
trigger:
  keywords: [mobile, phone, ios, android, 移动, 手机]
priority: 25
budget: 1500
category: domain
---

MOBILE APP — MANDATORY THREE-SECTION ARCHITECTURE:

Every mobile screen is composed as a vertical stack of exactly three sections.
You MUST define all three before generating any content.

## 1) STATUS BAR (OS-controlled) — PRE-INSERTED

The status bar (time, signal, wifi, battery) is **automatically pre-inserted** by the orchestrator as the first child of the root frame. It is a fixed 62px-tall frame with hardcoded path icons.

- **DO NOT generate a status bar** — it already exists
- **DO NOT delete or modify** the pre-inserted status bar
- Your first section should start BELOW the status bar (it occupies ~62px)

## 2) APP CONTENT (your layout)

ALL content elements must sit inside ONE wrapper container (vertical stack).

Wrapper provides:

- Consistent left/right padding: 16-20px (applied ONCE at wrapper level)
- Gap-based vertical spacing between sections (use gap, NOT margins)
- padding-bottom equal to the gap value for bottom space (NOT spacer elements)

Content stacking order inside the wrapper:

1. Top context: title / navigation header / search / filters
2. Primary content: the main "job to be done" for this screen
3. Supporting content: secondary modules, help text, empty states
4. Floating actions (optional): FAB or sticky CTA

Rules:

- One primary intent per screen. Everything else is subordinate.
- First 1-2 elements must answer "where am I" + "what can I do here"
- Title font size must be uniform across ALL screens in the app
- Design for one-handed use: primary actions in lower half
- Single vertical scroll (avoid nested scrolls)
- Touch targets: minimum 44x44px

DO NOT:

- Add per-section horizontal padding (wrapper handles it)
- Use spacer elements for bottom space (use padding-bottom)
- Cram multiple competing sections above the fold

## 3) BOTTOM TAB BAR — PILL STYLE

Tab Bar Container:

- Full screen width
- Padding: [12, 21, 21, 21] (includes home-indicator safe area)
- Fill: gradient overlay (transparent at top → solid background at 30%)

Pill (menu items wrapper):

- Height: 62px, width: fill_container
- Corner radius: 36px
- Border: 1px solid (theme border color)
- Inner padding: 4px

Tab Items (3-5 tabs, top-level destinations only):

- Width: fill_container, height: fill_container
- Corner radius: 26px
- Layout: vertical, gap: 4, centered on both axes
- Icon: 18px
- Label: 10px, weight 500-600, UPPERCASE, letterSpacing: 0.5

Active state: solid fill (accent color) + contrasting icon/label color
Inactive state: transparent background + muted icon/label color

Rules:

- Labels MUST be uppercase
- Tab switching preserves each tab's navigation state
- App content must never be obscured by the Tab Bar

## BLUEPRINT (internal planning)

Before generating nodes, mentally verify these three layers are accounted for:

1. Status Bar: standard or edge-to-edge?
2. App Content: what is the header, primary content, action placement, scroll behavior?
3. Bottom Bar: None or Pill Tab Bar (which tabs)?

Do NOT output this blueprint as text. Apply it silently through your node structure.
Your output must remain valid JSON/JSONL only.
