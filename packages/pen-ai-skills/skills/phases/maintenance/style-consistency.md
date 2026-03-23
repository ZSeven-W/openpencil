---
name: style-consistency
description: Preserve visual consistency when modifying existing designs
phase: [maintenance]
trigger: null
priority: 10
budget: 1000
category: base
---

STYLE CONSISTENCY RULES:

When modifying an existing design, preserve visual coherence:

COLOR PALETTE:
- Extract the existing palette from context nodes before making changes.
- New elements MUST use colors from the existing palette unless the user explicitly requests new colors.
- Maintain the same accent color usage pattern (primary for CTAs, secondary for highlights).

TYPOGRAPHY:
- Match existing font families — do not introduce new fonts unless requested.
- Maintain the same type scale (heading sizes, body sizes, caption sizes).
- Preserve lineHeight and letterSpacing patterns from existing text nodes.

SPACING:
- Match existing padding and gap values when adding new sections or elements.
- Section padding should be consistent across the design.
- Card internal padding should match sibling cards.

VISUAL TREATMENT:
- cornerRadius should be consistent across similar element types.
- Shadow styles should match existing elements of the same category.
- Border/stroke styles should be consistent (same color, same thickness).
- clipContent should match sibling containers.

HIERARCHY:
- Maintain the same depth of nesting — do not add unnecessary wrapper frames.
- Keep the same layout pattern (vertical sections with horizontal content rows).
- Width strategies (fill_container vs fixed) should match siblings.
