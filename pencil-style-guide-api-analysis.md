# Pencil `public/style-guide` API Analysis

Date: 2026-06-23

Scope: `https://api.pencil.dev/public/style-guide` and the adjacent endpoints used by Pencil Desktop for style-guide discovery.

## Executive Summary

`/public/style-guide` is not a public static document endpoint. It is an authenticated POST API that returns one Markdown style guide selected either by `name` or by tag matching. The style guide content is injected into the agent prompt as design direction.

The style-guide system has three layers:

1. `style-guide-tags`: returns the available vocabulary of style tags.
2. `style-guide`: returns a single Markdown guide by `tags` and/or `name`.
3. `style-guides`: returns gallery/list metadata for the UI picker, including guide names and thumbnails.

Pencil Desktop wraps the returned guide in:

```text
# Use the following style guide in the current design task

## Name of the styleguide: `<blob_pathname without .md>`

Use the above name in `spawn_agent` if you want to pass it to subagents.

<guide markdown>
```

## Access Behavior

Direct unauthenticated probes:

| Endpoint | Method | Result |
| --- | --- | --- |
| `/public/style-guide` | `GET` | `405` |
| `/public/style-guide` | `POST {}` | `401 {"message":"Unauthorized"}` |
| `/public/style-guide-tags` | `POST {}` | `401 {"message":"Unauthorized"}` |
| `/public/style-guides` | `POST {}` | `401 {"message":"Unauthorized"}` |

Authenticated access through Pencil Desktop MCP succeeded for `get_style_guide_tags` and `get_style_guide`. I did not dump or print local session tokens.

## Client Request Model

Reverse-engineered from the Pencil Desktop renderer bundle:

```ts
sendAPIRequest("POST", "style-guide-tags", {})

sendAPIRequest("POST", "style-guide", {
  tags,
  name,
  version: 1,
})
```

The renderer maps this to:

```text
POST https://api.pencil.dev/public/{endpoint}
```

Authentication mode:

- JWT session: `Authorization: Bearer <token>` plus optional `X-Device-Id`.
- Legacy license token: body includes `email`, `token`, and `client`.

The body also includes:

```json
{
  "client": "desktop"
}
```

or `extension` / `editor` depending on runtime.

## Response Shapes

Observed/derived response shapes:

```ts
// style-guide-tags
{
  success: boolean;
  tags: string[];
}

// style-guide
{
  success: boolean;
  name: string;
  guide: string; // Markdown
}

// style-guides, used by UI picker
{
  guides: Array<{
    id: string;
    name: string;
    blob_pathname: string;
    thumbnail_blob_url?: string;
  }>;
}
```

`get_style_guide` exposes only the wrapped Markdown message to the agent. The raw `style-guides` gallery list is not exposed as an MCP tool.

## Full Tag Vocabulary

The `style-guide-tags` tool returned the following raw tag vocabulary:

```text
constructivist, confident, lowercase, pill-shaped, code-native, sophisticated,
semantic-color, vibrant, calm, matrix, numbered-nav, brutalist, publication,
command-center, minimal, pastel, webapp, noir, soft-shadows, industrial,
lime-accent, stone, warm, icon-nav, enterprise, condensed-type, rounded, bright,
cli, badges, dashboard, typography, serif, parchment, analytical, colorful,
cozy, expressive, sage-accent, japanese, layered, whitespace, green-gray, neon,
high-end, friendly, snake_case, stroke-based, devtools, serif-sans, typographic,
geometric, editorial, icon-rail, light-mode, black-stroke, quiet, data-focused,
executive, crisp, refined, engineered, print, scandinavian, soft, ruled-lines,
ivory, icons-only-nav, architectural, shadowed, gold-accent, primary-colors,
slate, electric, fintech, orange-accent, poster, organic, bold-type, bauhaus,
corporate, nordic, cream, wellness, earthy, clean, humanist, cyan-accent,
command-line, graphic, condensed, urban, sharp-edged, austere, precise,
red-accent, stone-palette, - fill: "#fef0e8", champagne, nature-inspired,
off-white, mechanical, dual-tone, monochrome, literary, bento-grid, numbered,
luxury, earth-tones, flat, tactile, monospace, color-blocks, navy-accent,
code-inspired, timeless, professional, italic, neon-green, serif-display,
gradient, bold, shapes, sage-green, dark-mode, minimalist, high-impact, gold,
functional, high-contrast, rational, modern, burgundy-accent, icon-sidebar,
blue-accent, tech, dual-font, data-dashboard, developer, masthead,
yellow-accent, warm-tones, floating-nav, black-white, informational,
typography-only, purple, swiss, paper, soft-corners, technical, elegant, mobile,
classical, premium, terracotta, terminal, flush-layout, subtle,
bold-typography, green-accent, single-font, crimson-accent, engineering,
dark-sidebar, sharp-corners, playful, sharp, approachable, mesh-gradient,
sidebar, institutional, data-driven, neutral, zen, dark-to-light, display,
uppercase, magazine, financial, airy
```

Notable issue: `- fill: "#fef0e8"` appears in the tag list. That looks like a source-data leak or parsing bug, not a real semantic tag.

## Tag Taxonomy

Product/form-factor tags:

- `webapp`, `mobile`, `dashboard`, `data-dashboard`, `fintech`, `financial`, `developer`, `devtools`, `cli`, `enterprise`, `corporate`, `wellness`

Color/mode tags:

- `dark-mode`, `light-mode`, `dark-to-light`, `black-white`, `lime-accent`, `gold-accent`, `orange-accent`, `sage-accent`, `cyan-accent`, `red-accent`, `blue-accent`, `yellow-accent`, `green-accent`, `crimson-accent`, `burgundy-accent`, `navy-accent`, `neon-green`, `purple`, `terracotta`

Style-movement tags:

- `brutalist`, `bauhaus`, `swiss`, `japanese`, `scandinavian`, `nordic`, `constructivist`, `noir`, `editorial`, `publication`, `magazine`, `classical`, `literary`, `industrial`, `terminal`

Visual vocabulary tags:

- `minimal`, `minimalist`, `geometric`, `organic`, `rounded`, `sharp`, `sharp-edged`, `sharp-corners`, `soft-corners`, `flat`, `tactile`, `mesh-gradient`, `gradient`, `color-blocks`, `bento-grid`, `pill-shaped`, `shadowed`, `soft-shadows`, `ruled-lines`

Typography tags:

- `serif`, `serif-sans`, `serif-display`, `monospace`, `condensed-type`, `condensed`, `bold-type`, `bold-typography`, `typographic`, `typography-only`, `display`, `uppercase`, `lowercase`, `italic`, `single-font`, `dual-font`, `snake_case`

Layout/navigation tags:

- `sidebar`, `dark-sidebar`, `icon-sidebar`, `icon-nav`, `icons-only-nav`, `icon-rail`, `floating-nav`, `numbered-nav`, `flush-layout`, `whitespace`, `layered`, `command-center`

Mood/quality tags:

- `premium`, `luxury`, `high-end`, `sophisticated`, `elegant`, `confident`, `calm`, `quiet`, `refined`, `professional`, `approachable`, `friendly`, `playful`, `cozy`, `vibrant`, `bright`, `austere`, `precise`, `functional`, `rational`, `timeless`, `modern`, `high-impact`

## Guide Content Structure

Across 25 sampled guide responses, these sections appeared in every guide:

- `Style Summary`
- `Description`
- `Key Aesthetics`
- `Tags`
- `Color System`
- `Core Backgrounds`
- `Text Colors`
- `Accent Colors`
- `Typography`
- `Font Families`
- `Type Scale`
- `Font Weights`
- `Letter Spacing`
- `Spacing System`
- `Padding Scale`
- `Layout Pattern`
- `Corner Radius`
- `Icons`
- `Icon Style`
- `Icon Color States`

Common optional sections:

- `Border Colors`
- `Line Height`
- `Icons Used`
- `Icon Sizes`
- `Gap Scale`
- `Gap Scale (between elements)`
- `Gradients`
- `Shadows`

The guide is operationally useful for an agent because it gives exact values: color hexes, font names, point sizes, font weights, letter spacing, spacing scale, radii, icon library/style, and state colors.

## Sampled Guide Index

The following guide names were observed through tag-based sampling:

| Guide name | Title | Typical role |
| --- | --- | --- |
| `mobile-01-minimalplayful_light` | Minimal Playful Mobile Dashboard | light mobile app, friendly rounded UI |
| `mobile-02-brutalistluxury_light` | Brutalist Luxury Mobile Dashboard | dark mobile, sharp geometry, gold accent |
| `mobile-02-cleanminimal_light` | Clean Minimal Mobile Dashboard | light mobile, Scandinavian/organic |
| `mobile-02-editorialdatadriven_light` | Editorial Data-Driven Mobile Dashboard | mobile data/information UI |
| `mobile-02-japaneseswiss_light` | Japanese Swiss Mobile Dashboard | calm mobile, off-white, navy accent |
| `mobile-03-darkbold_light` | Dark Bold Mobile | dark neon/lime mobile |
| `mobile-03-swissclean_light` | Swiss Clean Mobile | rational light mobile, blue accent |
| `webapp-01-elegantluxury_light` | Elegant Luxury Dashboard | dark web dashboard, gold/serif |
| `webapp-01-industrialtechnical_light` | Industrial Technical Web Dashboard | dark technical dashboard |
| `webapp-01-japaneseswiss_light` | Japanese Swiss Web Dashboard | light dashboard, red accent |
| `webapp-01-nordicbrutalist_light` | Nordic Brutalist Web Dashboard | warm brutalist dashboard |
| `webapp-02-bauhausdigital_light` | Bauhaus Digital Dashboard | primary colors, geometric |
| `webapp-02-brutalistluxury_light` | Brutalist Luxury Dashboard | dark web, gold accent, high contrast |
| `webapp-02-monochromeexpressive_light` | Monochrome Expressive Dashboard | black/white, editorial typography |
| `webapp-02-terminalindustrial_light` | Terminal Industrial Dashboard | dark mechanical terminal style |
| `webapp-02-terminalswiss_light` | Terminal Swiss Dashboard | high-contrast terminal/swiss |
| `webapp-03-editorialdata_light` | Editorial Data Dashboard | serif editorial web dashboard |
| `webapp-03-elegantluxury_light` | Elegant Luxury Dashboard | premium dark sidebar webapp |
| `webapp-03-nordicbrutalist_light` | Nordic Brutalist Dashboard | architectural warm dashboard |
| `webapp-03-scandinavianminimal_light` | Scandinavian Minimal Dashboard | warm wellness/minimal webapp |
| `webapp-03-swisscleanexpressive_light` | Swiss Clean Expressive Dashboard | bold red-accent dashboard |
| `webapp-03-terminaltechnicalcrisp_light` | Terminal Technical Crisp Dashboard | developer/terminal dashboard |

The `_light` suffix appears in all sampled names, even when the guide describes a dark-mode visual system. It should be treated as part of the server blob name, not necessarily a visual-mode label.

## Selection Behavior

The API can return a guide by exact name, but when called with tags it behaves like a matcher. The returned guide is not guaranteed to include every requested tag. For example, mobile-oriented requests can return webapp guides when the remaining style tags match better.

Practical implication: for reproducible design runs, use `name` once a guide has been selected. Use `tags` only for discovery or when some variability is acceptable.

## Design System Pattern

Each style guide encodes design DNA at four levels:

1. Narrative identity: target vibe, product fit, and visual metaphor.
2. Tokens: colors, fonts, sizes, spacing, radii, shadows, gradients.
3. Components/states: navigation, cards, buttons, labels, badges, status states.
4. Agent instructions: what to emphasize, what to avoid, and how to maintain consistency.

This lets the agent turn style into concrete `.pen` operations:

- `fill`, `stroke`, `shadow`, `radius`, `opacity`
- `fontFamily`, `fontSize`, `fontWeight`, `letterSpacing`, `lineHeight`
- layout gaps, padding, fixed widths, screen widths
- icon family, stroke style, active/inactive icon states

## Animation Relevance

The style-guide API itself does not define motion, keyframes, transitions, timeline animation, or canvas animation rules. The returned content is static design direction. Pencil's visible "animated" generation effect comes from streaming `batch_design` operations into the canvas and flashing changed nodes, not from style-guide animation fields.

## Takeaways for OpenPencil

To reproduce this capability cleanly:

1. Store style guides as Markdown documents with structured headings.
2. Expose two APIs: `style-guide-tags` for discovery and `style-guide` for retrieval.
3. Support both `tags` matching and exact `name` lookup.
4. Return exact token values, not only mood words.
5. Make guide names stable so subagents can request the same guide.
6. Avoid copying full style guide prose into code; use it as prompt context.
7. Validate tag extraction to avoid malformed entries like `- fill: "#fef0e8"`.

## Local Artifacts

Sampled Markdown guide responses were saved under:

```text
/tmp/pencil-styleguides
```

They are temporary reverse-analysis artifacts, not committed source assets.
