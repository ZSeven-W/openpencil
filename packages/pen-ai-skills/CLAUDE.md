# pen-ai-skills

AI prompt skill engine for OpenPencil design generation.

## Structure

- `skills/` — Markdown + frontmatter skill files, organized by phase/domain/knowledge
- `src/engine/` — Skill resolution pipeline (loader → resolver → budget)
- `src/memory/` — Document context and generation history persistence
- `vite-plugin-skills.ts` — Build-time compiler: .md → _generated/skill-registry.ts

## Adding a new skill

1. Create a `.md` file in the appropriate `skills/` subdirectory
2. Add YAML frontmatter (name, description, phase, trigger, priority, budget, category)
3. Write prompt content as markdown body
4. Dev server auto-recompiles via HMR

## Testing

```bash
bun --bun vitest run packages/pen-ai-skills/src/__tests__/
```
