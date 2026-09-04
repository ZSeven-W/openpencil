---
name: kinetic-typography
description: Word-split text animation - staggered reveals, counters, and kinetic headlines via child Text nodes plus animate
phase: [generation]
trigger:
  keywords: [kinetic, typewriter, stagger, staggered, word-by-word, letter, reveal, count-up, counter, scramble, 逐词, 逐字, 打字机, 数字滚动, 文字动画]
priority: 26
budget: 1500
category: domain
---

KINETIC TYPOGRAPHY - split text into child nodes, animate with staggered delays.

There is NO document-level textSplit primitive. You build the effect yourself:
one Frame per line, one fit_content Text child per word, one animate per child
with an increasing delayMs. The runtime already supports everything needed
(shared clock, per-track delay, 256 track cap).

## Structure: the word-split pattern

- Line container: Frame, layout="horizontal", gap 6-10 (word spacing), width="fit_content".
- Each word: Text, width="fit_content", textGrowth="auto" (never fixed-width for single words).
- Multi-line headlines: NO flex-wrap exists. You must split words into
  line Frames yourself (vertical parent, one horizontal Frame per line).
  Estimate ~0.55 x fontSize per Latin char, 1.0 x fontSize per CJK char;
  keep each line's estimated width within the parent's inner width.

## Animation: stagger via delayMs

Attach the trigger (onMount / onTap / onScreenEnter) to the container, with
one animate per word in ONE action list (same dispatch = same clock origin):

    "onMount": [
      { "animate": { "target": "w1", "property": "opacity", "from": 0, "to": 1,
                     "durationMs": 240, "delayMs": 0,   "fillMode": "backwards" } },
      { "animate": { "target": "w2", "property": "opacity", "from": 0, "to": 1,
                     "durationMs": 240, "delayMs": 80,  "fillMode": "backwards" } },
      { "animate": { "target": "w3", "property": "opacity", "from": 0, "to": 1,
                     "durationMs": 240, "delayMs": 160, "fillMode": "backwards" } }
    ]

Rules that make or break it:
- fillMode "backwards" (or "both") is MANDATORY with delayMs > 0: without it
  the word shows at full opacity during its delay, killing the reveal.
- All animates for one effect go in ONE list. Splitting them across separate
  events drifts their clock origins and breaks the rhythm.
- Stagger step 60-100ms reads as a wave; >150ms reads as items loading.
- Combine properties per word freely (opacity + y offset via "y" from
  baseline+12 to baseline) - each (target, property) is its own track.
- Budget: track count = words x properties. Stay well under 256; a 20-word
  headline with 2 properties uses 40.

## Count-up numbers

One Text node, animate its text is NOT possible (text is not an animatable
property). Use a state counter instead: set $app.n via repeated interval
events, bind the Text content to $app.n. Prefer this only when asked;
otherwise show the final number statically.

## Baseline constraint (accept it, do not fight it)

The layout engine has no font-metrics baseline pass. Words of DIFFERENT
fontSize in one horizontal line align by center or end only - true baseline
alignment is impossible. Either keep one fontSize per line, or use
alignItems="end" and accept ~5-10% visual offset. Never mix three or more
sizes in one line.

## When NOT to split

- Body copy, paragraphs: never split (wrapping text must stay one node).
- More than ~25 words: the effect reads as lag, not craft. Animate the
  container's opacity instead.
- Glyph-level effects (per-character morph, variable-font axes): out of
  scope - the runtime has no glyph handles. Do not fake them with
  per-character nodes beyond a single short word.
