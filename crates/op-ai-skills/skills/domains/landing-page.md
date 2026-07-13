---
name: landing-page
description: Landing page and marketing site design patterns
phase: [generation]
trigger:
  keywords: [landing, marketing, hero, homepage]
priority: 35
budget: 1700
category: domain
---

LANDING PAGE — CONVERSION-DRIVEN MARKETING DEPTH

You design a page with ONE conversion intent (sign up / buy / book a demo). Every element moves toward that action; cut what doesn't. Sell the transformation, not the feature — the visitor buys a better version of themselves. The always-on design craft (type scale, 8px spacing, contrast, don't-box-everything, transparent structural wrappers) and the semantic ROLES (`section`/`hero`/`navbar`/`cta-section`/`footer`/`stats-section`/`feature-grid`/`card`/`button`) already supply layout defaults — do NOT restate their padding/layout numbers. This adds the marketing DEPTH on top.

## Section flow (narrative arc)

Build promise → proof → trust → action: what it is → why it works → why to trust it → what to do next. Order sections to tell that story; don't dump features at random. Define the copy first, then design around it — words drive layout, not the reverse.

## Rhythm (the highest-value lever)

- Alternate density: never stack two sections of similar weight. Follow a text-heavy section with a visual one, a dense one with an airy one.
- Alternate tone: DARK sections read as credibility / depth / emphasis (hero, CTA band, key stat); LIGHT sections read as explanation / detail (features, how-it-works). Use a dark band to make the final CTA punch.
- Backgrounds carry the rhythm — alternate page-bg vs surface; do NOT separate sections with decorative dividers.

## Composition discipline

- ONE alignment axis per section — never mix centered and left-aligned content in the same section. Hero/CTA/quote = centered; feature lists/specs = left.
- Proximity = relationship: tight gaps inside a group, generous space between groups.
- Don't center-align more than 2–3 lines; longer copy goes left.
- Body line length 50–75 characters: constrain long paragraphs to ~`width: 640` rather than letting them span the full 1200.

## Color for conversion

- The CTA is the single most prominent thing on the page. Reserve `$color-accent` (or the style-guide accent; hex when no palette) for ACTIONS only — don't spend it on decorative chrome, or the button stops standing out.
- One primary action per section; secondary actions are ghost/outline, visually quieter.

## Imagery

- During initial image-query or image-prompt authoring, prefer scenes from the visitor's future — people in the outcome state > product-in-context > product-in-environment > isolated product (use last sparingly). Use "would the visitor want to feel that way?" only as an initial selection heuristic before inserting the image.
- During automatic screenshot-driven self-check, a visibly rendered image is presentation-only: do not revisit or replace it merely because another subject, stock photo, search result, or generated image might feel more persuasive or attractive, unless the user explicitly requests an image edit.
- Text over an image needs its own contrast treatment (overlay scrim, shadow, or text in a sibling container). NEVER set an AI image as a background fill with text on top — image and text are SIBLINGS, not layers. Never stretch or distort.

## Headline hierarchy (write strongest-down)

1. Transformation — "Finally feel in control of your inbox" (strongest)
2. Outcome — "Ship more content, grow your audience"
3. Benefit — "Write 10x faster"
4. Feature — "AI-powered writing assistant" (weakest)

Lead the hero with transformation or outcome; benefit/feature only in supporting copy.

## Section archetypes (pick per section; `intent — structure`)

- Nav — `navbar` 3 groups: logo | center links | accent CTA, `space_between`
- Hero — `hero`, ONE headline (40–56) + ONE subtitle (16–18) + ONE CTA; optional visual right (2-col). Hero = the entire pitch compressed into the first screen.
- Logo wall — muted row of customer logos, equal size, low contrast (proof, not focus)
- Features — section title + `feature-grid` of 3–4 equal `feature-card`s (`fill_container` width+height for even rows)
- How-it-works — 3 numbered steps in a row, each: number/icon + label + 1-line desc
- Social proof — `testimonial` card (quote + avatar + name/title) OR `stats-section` (3–4 big numbers + labels)
- Pricing — 2–3 `pricing-card`s in a row, ONE highlighted (accent border / scale) as the recommended plan
- FAQ — left-aligned vertical list of question + answer pairs
- CTA band — `cta-section`, dark/accent background, centered, ONE headline + ONE button (the conversion climax)
- Footer — `footer`, multi-column brand + link groups + social, muted, smaller text

GENERAL: keep a consistent centered content max-width (~1040–1160) across sections for alignment stability; consistent card radius (12–16) and subtle shadow; `clipContent: true` on cards holding images. Apply all of this silently through node structure; never emit archetype labels as visible text.
