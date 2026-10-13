---
name: style-guide-selector
description: Select a pre-built visual style guide based on user request
phase: [planning]
trigger: null
priority: 3
budget: 1600
category: base
---

## Style Guide Selection

Available style guides:
{{availableStyleGuides}}

Honor the user's style and colors first. Match platform, mood and industry. Set `styleGuideName` to the exact name; use its `bg:` for the canvas so planning and generation share a palette. Avoid accidental light/dark mixing. If no guide fits, omit `styleGuideName`.
