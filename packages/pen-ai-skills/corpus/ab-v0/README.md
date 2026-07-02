# A/B Corpus v0

24 prompts for the element-tools A/B quality evaluation. See
`~/workspace/openpencil-docs/superpowers/plans/2026-04-20-element-tools-ab-corpus.md`
for the experiment design.

Split: 8 prompts × 3 categories (`mobile` / `dashboard` / `landing`), each
category further split 4 `obvious` + 4 `optional`.

- **obvious** — prompt matches one of the 39 `add_*_v0` tools perfectly; a
  well-calibrated decision tree MUST pick that tool. Used to score
  treatment-arm routing quality (M5).
- **optional** — prompt has no dominant element-tool fit; either tool path
  or `batch_design` is acceptable. Used to check that N-tool availability
  doesn't HURT on cases where it doesn't help.

Schema per file: see `../../../src/corpus/corpus-loader.ts` for the canonical
definition. Keys:

| key                  | required | notes                                           |
| -------------------- | -------- | ----------------------------------------------- |
| id                   | yes      | kebab-case, unique, stable                      |
| category             | yes      | mobile / dashboard / landing                    |
| difficulty           | yes      | obvious / optional                              |
| prompt               | yes      | natural-language design request                 |
| expected             | yes      | shape gates applied to rendered output          |
| expected_tool_if_any | no       | obvious-only: which add\_\*\_v0 tool should win |
