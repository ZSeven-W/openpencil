---
name: incremental-add
description: Rules for adding new elements to existing designs
phase: [maintenance, generation]
trigger:
  keywords: [add, insert, new section, append, continue, 继续, 再加, 追加]
priority: 20
budget: 1500
category: domain
---

INCREMENTAL ADDITION RULES:

When adding new elements to an existing design:

CONTEXT AWARENESS:

- Analyze the existing design structure before adding new elements.
- Match the visual style (colors, fonts, spacing, cornerRadius) of existing siblings.
- Place new elements in logical positions within the hierarchy.

SIBLING CONSISTENCY:

- New cards in a card row MUST match existing cards' width/height strategy (typically fill_container).
- New inputs in a form MUST match existing inputs' width and height.
- New sections MUST use the same padding and gap patterns as existing sections.

INSERTION RULES:

- Use "\_parent" to specify where the new node belongs in the tree.
- New sections append after the last existing section by default.
- New items within a list/grid append after the last existing item.
- Preserve z-order: overlay elements (badges, indicators) come BEFORE content.

COMMON PATTERNS:

- "Add a section" -> new frame with width="fill_container", height="fit_content", layout="vertical", matching section padding.
- "Add a card" -> new frame matching sibling card structure (same children pattern, same styles).
- "Add an input" -> new frame with role="input" or "form-input", width="fill_container", matching sibling inputs.
- "Add a button" -> new frame with role="button", matching existing button style.
- "Add a row" -> new frame with layout="horizontal", appropriate gap and alignment.

ID GENERATION:

- Use unique descriptive IDs for new nodes (e.g. "new-feature-card", "contact-section").
- Never reuse existing IDs.
