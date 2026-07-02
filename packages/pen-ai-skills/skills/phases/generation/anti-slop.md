---
name: anti-slop
description: Prevent generic AI aesthetics and enforce visual diversity across generations
phase: [generation]
trigger:
  keywords: [landing, website, marketing, 页面, 网站, promotional, homepage, 官网, 营销]
priority: 15
budget: 600
category: domain
---

ANTI-SLOP RULES (mandatory for every design):

## Visual Diversity

1. NO flat solid-color backgrounds for major sections.
   Use at minimum: subtle gradient, noise texture, geometric pattern, or layered fills.

2. Card layouts must NOT be identical.
   Each card group needs at least 1 differentiating element (size, image placement, accent).

3. Alternate section rhythm: text-heavy <-> visual, dark <-> light.
   Never stack multiple text-only sections consecutively.

4. Hero section: NEVER use AI-generated images as background fills with text on top.
   Images and text are siblings, not layers.

## Cross-Generation Diversity

Recent generation history (use to avoid repetition):
{{recentHistory}}

Rules:

- Heading font MUST differ from recent generations listed above
- Color palette MUST NOT duplicate the most recent generation

## Creative Variation (mandatory)

After establishing your baseline direction, introduce 1-3 small creative variations (~10% each):

- Asymmetric layout for one section
- Unusual image cropping or placement
- Typography personality shift (weight, case, spacing)
- Depth/layering effect

These variations must NOT repeat across generations.

IMPORTANT: Do NOT output prose or explanations. Your output must remain valid JSON/JSONL only.
Apply these rules silently through your design choices.
