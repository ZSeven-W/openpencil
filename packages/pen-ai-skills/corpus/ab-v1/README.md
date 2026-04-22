# A/B Corpus v1

Supplemental corpus covering the 8 element tools added after the
ab-v0 baseline was frozen (2026-04-20). Intended for an A/B v2 run
that measures routing + legality on the new tool surface without
re-running the full 24-prompt v0 corpus.

Split: 8 `obvious` prompts, one per new tool. No `optional` prompts
in v1 — the new tools are narrow enough that any reasonable design
request either maps to them directly or doesn't, and the optional
slot is already well-populated in v0.

| Prompt                     | Tool                       | Category  |
| -------------------------- | -------------------------- | --------- |
| `mobile-bio-textarea`      | `add_textarea_v0`          | mobile    |
| `mobile-loading-skeleton`  | `add_skeleton_v0`          | mobile    |
| `mobile-country-select`    | `add_select_v0`            | mobile    |
| `dashboard-revenue-line`   | `add_chart_line_v0`        | dashboard |
| `dashboard-category-pie`   | `add_chart_pie_v0`         | dashboard |
| `mobile-image-placeholder` | `add_image_placeholder_v0` | mobile    |
| `mobile-single-comment`    | `add_comment_v0`           | mobile    |
| `dashboard-confirm-modal`  | `add_modal_shell_v0`       | dashboard |

ab-v0 stays frozen (the A/B v1 results in
`openpencil-docs/superpowers/notes/2026-04-20-ab-v1-results.md` are
reproducible against it).

Schema per file matches ab-v0 (see `../ab-v0/README.md`). Load via
`loadCorpus(path)` from `../../src/corpus/corpus-loader.ts`.
