---
name: cjk-typography
description: CJK (Chinese/Japanese/Korean) typography rules
phase: [generation]
trigger:
  keywords:
    - "/[\\u4e00-\\u9fff\\u3040-\\u309f\\u30a0-\\u30ff\\uac00-\\ud7af]/"
priority: 25
budget: 500
category: domain
---

CJK TYPOGRAPHY (Chinese/Japanese/Korean):

- Headings: "Noto Sans SC" (Chinese) / "Noto Sans JP" (Japanese) / "Noto Sans KR" (Korean). NEVER "Space Grotesk" / "Manrope" for CJK — no CJK glyphs.
- Body: ALWAYS "Inter" (uses system CJK fallback at render time). Do NOT pick a script-specific Noto for body — that rule is HEADING-only. Matches the `text` section of get_design_prompt, `decomposition.md` ("body='Inter'"), and `add_body_text_v0`.
- CJK lineHeight: headings 1.3-1.4 (NOT 1.1 like Latin), body 1.6-1.8 (Latin body is 1.4-1.6). letterSpacing: 0, NEVER negative (causes CJK character overlap).
- CJK buttons: each char is approximately fontSize wide. Container width >= (charCount × fontSize) + padding.
- Detect CJK from user request language — apply CJK rules (script-specific Noto for headings; Inter + CJK lineHeight/letterSpacing for body; 0 letterSpacing everywhere).
