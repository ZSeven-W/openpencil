---
name: deck-contract
description: Cross-tier deck contract — overflow/density/consistency laws, narrative arc, page-type routing, deck-specific slop bans
phase: [generation]
trigger:
  keywords: [slide, slides, deck, presentation, pitch deck, keynote, ppt, 幻灯片, 演示, 演示文稿, 路演, 课件, 汇报]
priority: 23
budget: 1700
category: domain
---

DECK CONTRACT

Holds for every deck. `slides` picks the tier and `deck-patterns` emits the skeleton; this decides what goes on a page, and in what order.

## State the communication job first

One sentence, before any page: "After this, [audience] should [decide / do / believe what], because [the one claim]." The audience is NOT necessarily whoever asked — a board, a class and a customer need different claims. Every page advances that sentence or is cut.

## Law 1 — overflow splits the page, it never shrinks the type

Content does not fit. Act in this order, always:

1. Cut the copy. Good layout cannot rescue bad editing.
2. Move to a denser page type (Law 2).
3. Split into two pages.

FORBIDDEN as fixes: a smaller font; more elements in the same region; `clipContent` to crop the excess; line-height below the floor. One extra page costs nothing, one crammed page costs the argument — this overrides any instinct to hold a page count.

## Law 2 — the density budget belongs to the page type, not the deck

A slot = one independent text node: title, kicker, bullet, figure, label, caption, footnote each count 1.

| density | slots | page types |
|---|---|---|
| low | <=3 (cover <=6) | cover, divider, statement, one big number — one thing to read, no bullets added to fill space |
| medium | 4-6 | argument, two-column compare, process — one claim plus light support |
| medium-high | 7-10 | three-column points, image+text, swimlane — structured, still an obvious primary |
| high | 11-16 | matrix, detail table, numbered paragraphs — evidence-dense; past the cap you SPLIT, not compress |

The cap follows the layout's carrying capacity — a matrix page may carry 16 slots where a process page may not carry 7. In a table a **row** is one slot, not a cell; header, legend, source and conclusion lines count 1 each. Counting cells would outlaw table pages, and tables are the reason the high tier exists. Check while planning, not after it breaks.

## Law 3 — visual language locked, information structure varies

Two mirror-image failures: eight pages of one template, and eight pages of eight styles. One table solves both.

| LOCKED across pages | MUST change across pages |
|---|---|
| aspect ratio; safe margins | position and form of the lead visual |
| type system (family + weight ladder) | information structure (compare / flow / list / data / narrate) |
| colour tokens and their roles | spatial anchor — never the same x/y twice |
| page-number and page-label position | rhythm: a dense page is followed by an open one |
| corner radius; strokes; icon style | emphasis words; compositional centre of gravity |

MARGIN FLOOR: the safe margin Law 3 locks is a floor, not a hint — slide root horizontal padding ≥64px, a 1080-wide card root ≥48px; no text sits against the canvas edge (a full-bleed background image is the only exception).

Adjacent pages may not share a page type (two consecutive tables is the only exception, and page 3 must change). A deck of >=6 pages covers >=4 page-type families. Do not centre every page.

Pre-flight, no rendering: say the page-type sequence aloud. If it repeats — "grid, grid, two-column, grid" — so does the deck. Fix the outline.

## Narrative

Pick exactly one arc: context-stakes-evidence-implication-action / question-analysis-answer / problem-cause-recommendation / today-shift-tomorrow / chronology or process progression.

**An agenda is not a narrative.** The sequence must accumulate: each page answers the question the page before it raised. Opening and closing are statements, not information. Never close on a detail page, a technical artefact, an unframed summary, or a "thank you".

**Ghost deck test**: read only the titles in order — do they carry the whole argument? If not, fix the outline before drawing. After each title the next should feel inevitable; a page that could sit anywhere is misplaced.

**Titles state conclusions, not topics** — "Churn concentrates in month two", not "Churn analysis". The judge: a real speaker would say this sentence out loud. If it reads like a prompt or a slogan, rewrite it.

## Page-type routing

Shared avoid rule: **when content fights the page type's region count, density cap or lead-visual role, change the page type — never bend the content into it.**

| page type | use when | avoid when |
|---|---|---|
| cover | one claim + one qualifier | it must carry agenda items or logo walls |
| statement / quote | one sentence is the whole point | it needs numbers beside it |
| big number | one figure carries the page (+ source) | two or more figures compete — use tiles or a matrix |
| three-column points | items are parallel and comparable | they differ in weight, or there are 4+ |
| two-column compare | before/after, option A/B | a 50/50 split (use 7:3) or mirrored sides |
| process / timeline | order carries meaning | the steps are unordered — that is a list |
| table / matrix | dense evidence read by column | under 3 rows, or one cell is the real message |
| image-led | the visual IS the evidence | decorative stock; one image reused in a deck |

## One accent, undecorated charts

- **One accent colour, at most once per page.** Occurrences count, not hues: an accent used 11 times is not an accent — the second occurrence goes neutral. The palette needs a lead: one colour over most of the area, one support, one accent — three sharing the stage equally is no position.
- **Charts carry no decoration**: no gridlines, legend or y-axis; values labelled on the marks; only the key series takes the accent, the rest a neutral ramp. Never pie, 3D, shadowed bars or gradient bars.
- Asymmetry beats symmetry: a perfectly centred page is static and directionless.

## Deck slop — each is a recognisable fingerprint

1. **Everything in a card.** Containers are for real grouping: a statement is the large type itself, a number is the number itself, a contrast can be space alone.
2. **Comparison pages mirroring one structure with only the colour swapped** — one template run twice. The sides must differ compositionally.
3. **The same spatial anchor on every page**; identical margins and content origin cancel out every other kind of variety.
4. **A rule under every title** — the clearest giveaway. Separate with space, colour, or a change of layout.
5. **Decorative shapes at 4-6% opacity** — pretending to design. A motif reads as a decision, or it is deleted.
6. **Flat hierarchy** — every size inside one 20px band. Largest >=2.5x body; no size above 60px anywhere means no hierarchy. Max 4 steps of the scale per page.
7. **Implementation language in visible copy** — "this slide", "the generated chart", "as mentioned above". Delete it.
8. **Formulaic titles** — "From X to Y", ad slogans, parallel phrasing on every page.
