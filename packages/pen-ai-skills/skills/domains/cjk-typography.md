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

CJK TYPOGRAPHY（Chinese/Japanese/Korean）：

- Headings："Noto Sans SC"（Chinese）/ "Noto Sans JP" / "Noto Sans KR"。CJK 绝不要使用 "Space Grotesk"/"Manrope"。
- Body："Inter"（system CJK fallback）或 "Noto Sans SC"。
- CJK lineHeight：headings 1.3-1.4（不是 1.1），body 1.6-1.8。letterSpacing：0，绝不要为负数。
- CJK buttons：每个 char 约等于 fontSize 宽。Container width >= (charCount x fontSize) + padding。
- 从 user request language 检测 CJK — 所有 text nodes 都使用 CJK fonts。
