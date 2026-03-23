---
name: layout
description: Auto-layout engine rules (flexbox-based positioning)
phase: [generation]
trigger: null
priority: 10
budget: 1500
category: base
---

LAYOUT ENGINE (flexbox-based):
- Frames with layout: "vertical"/"horizontal" auto-position children via gap, padding, justifyContent, alignItems.
- NEVER set x/y on children inside layout containers.
- CHILD SIZE RULE: child width must be <= parent content area. Use "fill_container" when in doubt.
- In vertical layout: "fill_container" width stretches horizontally. In horizontal: fills remaining space.
- CLIP CONTENT: clipContent: true clips overflowing children. ALWAYS use on cards with cornerRadius + image.
- justifyContent: "space_between" (navbars), "center", "start"/"end", "space_around".
- WIDTH CONSISTENCY: siblings must use same width strategy. Don't mix fixed-px and fill_container.
- NEVER use "fill_container" on children of "fit_content" parent — circular dependency.
- Two-column: horizontal frame - two child frames each "fill_container" width.
- Keep hierarchy shallow: no pointless wrappers. Only use wrappers with visual purpose (fill, padding).
- Section root: width="fill_container", height="fit_content", layout="vertical".
- FORMS: ALL inputs AND primary button MUST use width="fill_container". Vertical layout, gap=16-20.
