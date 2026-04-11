---
name: product-principles
description: Product-level design principles for functional UI quality
phase: [generation]
trigger: null
priority: 5
budget: 800
category: base
---

PRODUCT DESIGN PRINCIPLES (apply to every screen):

1. PURPOSE FIRST
   Every screen must have one clearly defined primary purpose and one primary action.
   If multiple goals compete, separate them into distinct surfaces.

2. DOMINANT REGION
   Every screen must contain one dominant visual region.
   Visual weight must reflect importance. Avoid equal-weight layouts and competing focal points.

3. ACTION HIERARCHY
   One primary action per screen or section. Secondary actions visually reduced.
   Destructive actions clearly distinct. Rare actions in overflow menus.
   Do not give equal emphasis to all actions.

4. ENTITY INTEGRITY
   When representing an entity (user, record, document, asset):
   display its name prominently, surface its status clearly, show key metadata, make actions obvious.

5. DENSITY INTENTIONALITY
   Choose one density mode per screen and commit:

- Compact: high data environments (tables, dashboards)
- Medium: balanced default (most screens)
- Airy: low-complexity workflows (onboarding, settings)
  Do not mix density modes within one screen.

6. CONSTRAINT OVER DECORATION
   If an element does not support navigation, understanding, decision-making, or action-taking,
   it should not exist. As little design as possible.

7. STRUCTURAL CONSISTENCY
   Similar problems must have similar solutions. Navigation logic must remain stable.
   Layout rhythm must feel system-driven. Spacing must follow a consistent scale.

8. SYSTEM STATUS VISIBILITY
   Every data-driven surface must support: loading state, empty state, error state,
   success confirmation. No silent failure. No blank ambiguity.
