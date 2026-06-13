---
name: element-manifest
description: Element manifest output protocol — one JSONL element declaration per line, system-assigned ids, `in` line-number nesting, schema-generated catalog of element kinds and params
phase: [generation]
trigger:
  flags: [hasManifest]
priority: 0
budget: 2400
category: base
---

# ELEMENT MANIFEST OUTPUT FORMAT

Output your section as an ELEMENT MANIFEST: one JSON object per line (JSONL).
Each line declares ONE element from the catalog below — the system builds,
styles, and lays out the element for you. Declaring elements is ALWAYS better
than hand-rolling frames: catalog elements come out pixel-correct.

RULES:

- `{"el":"<kind>", ...params}` — one element per line, top-to-bottom visual order.
- NEVER write `id`, `parent_id`, or `pageId`. Ids are system-assigned; these fields are ignored.
- Nest with `"in": <line number>` — the 1-based number of an EARLIER `{"el":"section"}` line in YOUR output. Lines without `in` stack vertically at the top level.
- `{"el":"section","direction":"vertical|horizontal","gap":16,"padding":[0,24],"role":"hero"}` is the only structural container. Use a horizontal section for card rows / column splits. Sections nest at most 2 deep.
- Param values: strings as-is, lists as JSON arrays. Omit a param to accept its default; `*`-marked params are expected (omitting them inserts placeholder content).
- Every kind also accepts `theme`: `"light" | "dark" | "system"` (default `"light"`; use `"system"` only when the document carries design-system variables).
- Only if NO catalog kind fits a custom visual, you may emit a raw canonical node line (`{"type":"frame",...}` / `{"type":"text",...}`); it may use `"in"` too. Use sparingly.

ANTI-PATTERN — DO NOT HAND-COMPOSE CATALOG ELEMENTS. Before writing any raw
`text` / `icon_font` / `frame` line, scan the catalog: if a kind covers that
visual, you MUST declare the kind instead. Hand-rolled copies score as broken.

- WRONG: `{"el":"section",...}` + `{"el":"text","content":"Active"}` + `{"el":"icon_font","iconFontName":"x"}`
- RIGHT: `{"el":"tag","label":"Active","removable":true}`
- Same rule for: badge, text_button, search_bar, stat_card, list_row, switch,
  checkbox, avatar, progress_bar, tabs, pagination, skeleton, divider — one
  declared kind beats three hand-rolled primitives, every time.

COVERAGE — DECLARE EVERY COMPONENT THE BRIEF NAMES. A multi-part brief
is one line per COMPONENT, never one element standing in for the whole.
Repeated items (6 activity entries, 4 stat cards, 5 table rows) get ONE
LINE EACH with their real content — do not collapse them into a single
line or skip the later ones. A typical section runs 4-12 element lines;
a one-line answer to a multi-part brief is wrong.
A component is one catalog kind with ALL its text packed into that one
line's params: a title with its muted subline is ONE section_header line
(title + subtitle), a value with its delta is ONE stat_card line — do
NOT shred a component into heading / body_text fragments.

EXAMPLE:
{"el":"section","gap":20,"padding":[0,24],"role":"stats"}
{"el":"heading","in":1,"content":"Revenue Overview","level":"h2"}
{"el":"section","in":1,"direction":"horizontal","gap":12}
{"el":"stat_card","in":3,"label":"MRR","value":"$48.2k","trend":"up","delta":"+12%"}
{"el":"stat_card","in":3,"label":"Churn","value":"2.1%","trend":"down","delta":"-0.4%"}

ELEMENT CATALOG — `kind: params` (`*` expected, `(a|b)` allowed values, `[..]` JSON array, `[{f1,f2}]` array of objects with those fields):
{{elementManifestCatalog}}
