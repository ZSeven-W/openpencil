---
name: cjk-typography
description: CJK (Chinese/Japanese/Korean) typography rules
phase: [generation]
trigger:
  keywords:
    - "/[\\u4e00-\\u9fff\\u3040-\\u309f\\u30a0-\\u30ff\\uac00-\\ud7af]/"
priority: 21
budget: 800
category: domain
---

CJK TYPOGRAPHY (Chinese/Japanese/Korean):

- LANGUAGE CONSISTENCY (critical): when the request is in Chinese/Japanese/Korean, write EVERY UI string in that language — labels, nav tabs, buttons, placeholders, badges, section titles. Do NOT leave English boilerplate mixed in (e.g. "Deliver to", "See all", "Home", "Order now") next to CJK text; translate those too ("配送至", "查看全部", "首页", "立即下单"). A half-translated screen ("Deliver to · 现在") reads as broken. Brand/product proper nouns may stay in their original form.
- Headings: "Noto Sans SC" (Chinese) / "Noto Sans JP" (Japanese) / "Noto Sans KR" (Korean). NEVER "Space Grotesk" / "Manrope" for CJK — no CJK glyphs.
- Body family, DESIGN layer: from the chosen tier's font pairing, exactly as in a Latin design. Script-specific Noto is a HEADING-only rule.
- Body family, RENDER layer: when that family is not in the font bundle, emit "Inter" and let the system CJK fallback carry the glyphs — a FALLBACK, never the reason a body style was chosen. Matches get_design_prompt's `text` section, `decomposition.md` ("body='Inter'"), `add_body_text_v0`.
- lineHeight bands by FONT SIZE, not by heading-vs-body (1.3 tears a 96px title apart): >=64px 1.02-1.15; 48-63px 1.15-1.25; 40-47px 1.3-1.4; body 1.7-1.8 (Latin 1.5-1.6 — CJK is +0.2); captions 1.45-1.5.
- letterSpacing is absolute px here, not em. <48px: ALWAYS 0, never negative — negative tracking collides CJK glyphs. >=48px: negative tracking is allowed down to `|letterSpacing| <= fontSize * 0.02` (that is -0.02em), fractional values fine (96 -> -1.92, 168 -> -3.36). Compare the ratio; do NOT round the cap first — at 72px the cap is 1.44, so -1.4 is legal and -2 is not. Uppercase Latin micro-labels may take +1 to +2 — decide by the run's ACTUAL script, not the label's style: a CJK label that looks like a small-caps tag still takes 0, because positive tracking opens gaps in an already full-width em box.
- CJK buttons: each char is approximately fontSize wide. Container width >= (charCount × fontSize) + padding.
- Line length: body runs <=30 Han chars per line (a 1728-wide box at 32px fits 54) — narrow the block or split it into columns; available width is not permission to use it.
- Never truncate a CJK title with an ellipsis: rewrite the copy or change the layout.
- Mixed runs: one space between a CJK run and a Latin/number run, written into the copy (not letterSpacing); none between a number and its unit. Digits and Latin take the Latin family — a CJK family rendering digits makes column widths wobble; numbers read down a column need monospace or fixed widths.
- Detect CJK from the request language — apply these rules (script-specific Noto for headings; the size-banded lineHeight/letterSpacing above everywhere).
- **Line-start taboo** — don't start a line with closing punctuation `，。、；：？！》」』）】`; rewrite or widen text box.
- **Line-end taboo** — don't end a line with opening punctuation `《「『（【`; pull content forward.
- **Execution** — adjust width or rewrite to avoid orphaned punctuation at line edges.

