# A/B Corpus v3

Successor to ab-v1 (frozen 2026-04-28 after the live A/B v2 sweep).
v3 carries forward all 40 v1 obvious prompts unchanged so v1 ↔ v3
results stay comparable on the overlapping subset, then layers in
two new dimensions:

1. **Element-tool coverage for v0.8.0 additions** — 7 obvious prompts
   for tools that shipped after the v1 freeze (setting_row /
   member_row / filter_group / invite_row / activity_log / event_card
   / step_card). Each tool gets exactly one obvious yaml so the
   right-tool routing metric stays interpretable per tool, same
   contract as v1.
2. **Composite difficulty class** — multi-tool prompts where no
   single `expected_tool_if_any` applies. They evaluate the
   `batch_design` fallback path and the model's ability to compose
   element tools when the brief spans more than one component. See
   `src/corpus/types.ts` for the new `difficulty: 'composite'` enum
   value and `score-run.ts::classifyRouting` for the
   `multi-tool / fallback / garbage` verdict it produces on
   composite-treatment runs.

## File counts (target)

- 40 obvious — inherited verbatim from v1
- 7 obvious — new (tools 91–97)
- 5 composite — new

Total: 52 prompts. Adjust `corpus-loader.test.ts` when adding more.

## Schema

Same as v0/v1 (see `../ab-v0/README.md`) plus:

```yaml
difficulty: composite # NEW — alongside obvious / optional
expected:
  must_contain_roles: [...] # describes the multi-element shape
# expected_tool_if_any:        # OMITTED on composite — no single tool
```

ab-v0 / ab-v1 stay frozen. Their results in
`openpencil-docs/superpowers/notes/2026-04-20-ab-v1-results.md` and
`2026-04-28-ab-v2-results.md` are reproducible against the originals.

## Loading

```ts
import { loadCorpus } from '@zseven-w/pen-ai-skills/src/corpus/corpus-loader';
const prompts = loadCorpus('packages/pen-ai-skills/corpus/ab-v3');
```

Or via the harness:

```bash
bun scripts/ab-corpus/run.ts --corpus ab-v3 --models gpt-5.5
```
