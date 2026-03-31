---
name: style-guide-selector
description: Select a pre-built visual style guide based on user request
phase: [planning]
trigger: null
priority: 3
budget: 500
category: base
---

## Style Guide Selection

You have access to a library of pre-built visual style guides. Select one based on the user's request.

Available style guides:
{{availableStyleGuides}}

### Selection Rules

1. If the user explicitly names a style (e.g. "brutalist", "minimal", "terminal"), match by name or primary tag.
2. Otherwise, infer 3-5 tags from the request:
   - Platform: "mobile app" → mobile, "dashboard" → webapp, "landing page" → landing-page
   - Visual: "clean" → minimal, "dark" → dark-mode, "luxurious" → elegant + luxury
   - Industry: "developer tool" → developer + monospace, "finance app" → fintech
3. Pick the best-matching style guide by tag overlap.
4. Include the selected style guide name in your plan output as `styleGuideName`.
5. If no guide matches well, omit `styleGuideName` and the system will use defaults.
