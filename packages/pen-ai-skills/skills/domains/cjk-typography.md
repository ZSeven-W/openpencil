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
- Headings: "Noto Sans SC" (Chinese) / "Noto Sans JP" / "Noto Sans KR". NEVER "Space Grotesk"/"Manrope" for CJK.
- Body: "Inter" (system CJK fallback) or "Noto Sans SC".
- CJK lineHeight: headings 1.3-1.4 (NOT 1.1), body 1.6-1.8. letterSpacing: 0, NEVER negative.
- CJK buttons: each char is approximately fontSize wide. Container width >= (charCount x fontSize) + padding.
- Detect CJK from user request language — use CJK fonts for ALL text nodes.
