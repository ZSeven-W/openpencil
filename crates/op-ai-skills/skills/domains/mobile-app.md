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
- Mobile top rhythm: keep the title/header group close to the first useful control or content module. On 375-430px screens, the gap from header/title to search, primary action, chart, or first card should usually be 20-32px. Do not leave an empty hero-sized band unless the prompt explicitly asks for an editorial hero.
- Title font size must be uniform across ALL screens in the app
- Design for one-handed use: primary actions in lower half
- Single vertical scroll (avoid nested scrolls)
- Touch targets: minimum 44x44px
- Do not repeat the same predictable mobile stack of search + categories + orange promo + two cards. Choose a distinct concept for the domain and make one signature moment carry the personality.

DO NOT:

- Add per-section horizontal padding (wrapper handles it)
- Use spacer elements for bottom space (use padding-bottom)
- Cram multiple competing sections above the fold

## 3) BOTTOM TAB BAR — OPTIONAL, INTEGRATED

Do not force bottom navigation into every mobile screen. Use a bottom tab bar only when the product clearly has persistent top-level destinations (Home, Search, Orders, Profile, etc.). If the screen is a single-task flow, omit bottom navigation.

Tab Bar Container:

- Full screen width and part of the page flow, not a floating overlay
- Height: 62-72px including safe-area breathing room
- Background: same page palette or a subtle tonal surface
- Separation: quiet 1px divider or tonal contrast only; no detached shadow band

Nav Surface:

- Use role="bottom-tab-bar"
- Not a floating pill, not a nested rounded capsule, and not a separate footer band
- Direct tab item frames stay transparent: no fill, no stroke, no large rounded tile

Tab Items (3-5 tabs, top-level destinations only):

- Width: fill_container, height: fill_container
- Layout: vertical, gap: 4, centered on both axes
- Icon: 18px
- Label: 10-11px, weight 500-600, letterSpacing: 0

Active state: accent icon/label color or a tiny 2-3px indicator
Inactive state: transparent background + muted icon/label color

Rules:

- Tab switching preserves each tab's navigation state
- App content must never be obscured by the Tab Bar

## BLUEPRINT (internal planning)

Before generating nodes, mentally verify these three layers are accounted for:

1. Status Bar: standard or edge-to-edge?
2. App Content: what is the header, primary content, action placement, scroll behavior?
3. Bottom Bar: None or integrated tab bar (which tabs)?

Do NOT output this blueprint as text. Apply it silently through your node structure.
Your output must remain valid JSON/JSONL only.
