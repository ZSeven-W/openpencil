---
name: text-rules
description: Text sizing, typography, and wrapping rules
phase: [generation]
trigger: null
priority: 15
budget: 1000
category: base
---

TEXT RULES:
- Body/description in vertical layout: width="fill_container" + textGrowth="fixed-width" (wraps text, auto-sizes height).
- Short labels in horizontal rows: width="fit_content" + textGrowth="auto". Prevents squeezing siblings.
- NEVER fixed pixel width on text inside layout frames — causes overflow.
- Text >15 chars MUST have textGrowth="fixed-width". NEVER set explicit pixel height on text nodes — OMIT height.
- Typography: Display 40-56px, Heading 28-36px, Subheading 20-24px, Body 16-18px, Caption 13-14px.
- lineHeight: headings 1.1-1.2, body 1.4-1.6. letterSpacing: -0.5 for headlines, 0.5-2 for uppercase.
