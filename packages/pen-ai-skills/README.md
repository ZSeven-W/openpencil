# @zseven-w/pen-ai-skills

AI prompt skill engine for [OpenPencil](https://github.com/ZSeven-W/openpencil) — phase-driven prompt loading with intent matching, token budgets, and design memory.

## Install

```bash
npm install @zseven-w/pen-ai-skills
# or
bun add @zseven-w/pen-ai-skills
```

## Overview

When an LLM generates designs for OpenPencil, it needs context: PenNode schema, layout rules, semantic roles, icon names, style guides, and more. Loading everything at once wastes tokens. This package solves that with **phase-based skill resolution** — only the relevant prompts are loaded for each stage of the design workflow.

```
User message ──► resolveSkills(phase, message, options)
                      │
                      ├─ Phase filter (planning / generation / validation / maintenance)
                      ├─ Intent matching (landing page? dashboard? form? mobile app?)
                      ├─ Flag-based activation (hasDesignMd? hasVariables?)
                      ├─ Priority sorting (higher priority = selected first)
                      └─ Token budget trimming (cap per phase)
                      │
                      ▼
               AgentContext { skills[], memory, budget }
```

## Quick Start

```typescript
import { resolveSkills } from '@zseven-w/pen-ai-skills';

// Generation phase — user wants a landing page
const ctx = resolveSkills('generation', 'design a SaaS landing page', {
  flags: { hasDesignMd: false, hasVariables: true },
});

// ctx.skills contains the relevant prompts
for (const skill of ctx.skills) {
  console.log(`${skill.meta.name} (${skill.tokenCount} tokens)`);
  // schema (800 tokens)
  // layout (600 tokens)
  // landing-page (400 tokens)
  // ...
}

// Budget tracking
console.log(`${ctx.budget.used} / ${ctx.budget.max} tokens used`);
```

## Phases

| Phase         | Budget | Purpose                                                  |
| ------------- | ------ | -------------------------------------------------------- |
| `planning`    | 4,000  | Analyze requirements, plan sections, choose style        |
| `generation`  | 8,000  | Generate PenNode trees with full design knowledge        |
| `validation`  | 3,000  | Check layout, spacing, accessibility, best practices     |
| `maintenance` | 5,000  | Edit existing nodes, delete, reparent, modify properties |

## Skill Categories

### Base Skills

Core design principles and workflow guides. Always loaded for their phase.

### Domain Skills

Activated by intent matching — keywords in the user message trigger specialized knowledge:

| Skill          | Triggers                                        |
| -------------- | ----------------------------------------------- |
| Landing page   | `landing`, `marketing`, `homepage`              |
| Dashboard      | `dashboard`, `admin`, `analytics`               |
| Form UI        | `form`, `login`, `signup`, `input`              |
| Mobile app     | `mobile`, `app`, `screen`, `ios`, `android`     |
| CJK typography | `chinese`, `japanese`, `korean`, CJK characters |

### Knowledge Skills

Reference material loaded by priority until the token budget is exhausted:

- **Role definitions** — semantic roles (button, input, card, navbar) and their auto-defaults
- **Icon catalog** — Lucide/Feather icon naming conventions
- **Design examples** — complete component patterns in DSL
- **Copywriting** — headline length, CTA text, placeholder copy rules
- **Code generation guides** — React, Vue, Svelte, Flutter, SwiftUI, Compose, React Native, HTML

## Design Memory

Track context across multi-turn generation sessions:

### Document Context

```typescript
import { createDesignContext, contextToPromptString } from '@zseven-w/pen-ai-skills';

const ctx = createDesignContext('/path/to/doc.op');
// Accumulates: palette, typography, spacing, aesthetic, page structure

const prompt = contextToPromptString(ctx);
// "Design system: palette #2563EB, #F8FAFC; font Space Grotesk / Inter; ..."
```

### Generation History

```typescript
import { createHistoryEntry, getRecentEntries } from '@zseven-w/pen-ai-skills';

const entry = createHistoryEntry({
  documentPath: '/path/to/doc.op',
  prompt: 'design a pricing section',
  phase: 'generation',
  skillsUsed: ['schema', 'layout', 'landing-page'],
  nodeCount: 28,
  sectionTypes: ['pricing'],
});

// Feed recent history back to prevent repetitive designs
const recent = getRecentEntries(allEntries, 5);
```

## Diagnostics

Detect common design issues in generated output:

```typescript
import { detectAllIssues } from '@zseven-w/pen-ai-skills';

const issues = detectAllIssues(document);
// [{ severity: 'warning', category: 'invisible-container', nodeId: 'frame-7', message: '...' }]
```

| Detector                | Catches                                                  |
| ----------------------- | -------------------------------------------------------- |
| Invisible containers    | Frames with no fill, stroke, or visual children          |
| Empty paths             | Path nodes with no `d` attribute                         |
| Text explicit heights   | Text nodes with hardcoded pixel height (causes overflow) |
| Sibling inconsistencies | Siblings with mixed width strategies in the same layout  |

## Adding a Skill

Create a Markdown file in `skills/` with YAML frontmatter:

```markdown
---
name: my-custom-skill
description: Guidelines for designing checkout flows
phase: [generation, validation]
trigger:
  keywords: [checkout, cart, payment, purchase]
priority: 8
budget: 1500
category: domain
---

## Checkout Flow Design Rules

1. Always show order summary alongside the form
2. Use a single-column layout for payment fields
3. ...
```

The Vite plugin auto-compiles skills into a TypeScript registry on save during development.

## Style Guide

Parse and apply external style guides:

```typescript
import { parseStyleGuideFile, buildStyleMapping } from '@zseven-w/pen-ai-skills';

const guide = parseStyleGuideFile(markdownContent);
const mappings = buildStyleMapping(guide);
// [{ property: 'fill', from: '#000', to: '$text-primary' }, ...]
```

## License

[MIT](./LICENSE)
