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
