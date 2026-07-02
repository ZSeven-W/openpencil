---
name: landing-page
description: Landing page and marketing site design patterns
phase: [generation]
trigger:
  keywords: [landing, marketing, hero, homepage]
priority: 35
budget: 1500
category: domain
---

LANDING PAGE DESIGN PATTERNS:

STRUCTURE:

- Navigation - Hero - Features - Social Proof - CTA - Footer
- Each section: width="fill_container", height="fit_content", layout="vertical"
- Root frame: width=1200, height=0 (auto-expands), gap=0

NAVIGATION:

- justifyContent="space_between", 3 groups: logo | nav-links | CTA button
- padding=[0,80], alignItems="center", height 64-80px
- Links evenly distributed in center group

HERO SECTION:

- padding=[80,80] or larger, generous whitespace
- ONE headline (40-56px), ONE subtitle (16-18px), ONE CTA button
- Optional visual: phone mockup or illustration on the right (two-column horizontal layout)
- Every extra element dilutes focus — keep it minimal

FEATURE SECTIONS:

- Section title + 3-4 feature cards in horizontal layout
- Cards: width="fill_container", height="fill_container" for even row alignment
- Alternate section backgrounds (#FFFFFF / #F8FAFC) for natural separation
- Section vertical padding: 80-120px

SOCIAL PROOF:

- Testimonials: card with quote + avatar + name/title
- Stats: horizontal row with stat-cards (number + label)
- Logos: horizontal row of company logos

CTA SECTION:

- Centered content, compelling headline, accent background or gradient
- Single prominent button

FOOTER:

- Multi-column layout: brand + link groups + social
- Muted colors, smaller text
- padding=[48,80]

GENERAL:

- Centered content container ~1040-1160px across sections for alignment stability
- Consistent cornerRadius (12-16px for cards)
- clipContent: true on cards with images
- Subtle shadows on cards

## Headline Hierarchy

Write headlines from the strongest level down:

1. Transformation: "Finally feel in control of your inbox" (strongest)
2. Outcome: "Ship more content, grow your audience"
3. Benefit: "Write 10x faster"
4. Feature: "AI-powered writing assistant" (weakest)

Lead with transformation or outcome. Use benefit/feature in supporting copy only.

## Image Intent Hierarchy

When choosing imagery (prioritize top → bottom):

1. Transformation imagery: people in the "after state" — emotion, outcome, identity achieved
2. Contextual use: people using the product in real environments
3. Product-in-environment: product in a setting implying use/outcome
4. Isolated product: product alone — use sparingly

Every image should be a scene from the visitor's future life.
Ask: "Would the visitor think 'I want to feel that way'?"

NEVER use AI images as background fills with text on top.
Images and text are siblings, not layers.
