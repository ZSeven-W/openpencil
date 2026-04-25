# A/B Corpus v1

Supplemental corpus covering element tools added after the ab-v0
baseline was frozen (2026-04-20). Intended for an A/B v2 run that
measures routing + legality on the new tool surface without re-running
the full 24-prompt v0 corpus.

All `obvious` prompts, one per new tool. Loaded by
`loadCorpus('corpus/ab-v1')` and asserted by
`src/corpus/__tests__/corpus-loader.test.ts` (which is the canonical
count + tool-list source of truth — keep it in sync when adding
prompts).

Batches:

- 2026-04-22 — 12 prompts (textarea / skeleton / select / chart_line /
  chart_pie / image_placeholder / comment / modal_shell_v0 / status_badge
  / tooltip / metric_comparison / notification_row)
- 2026-04-24 — 5 prompts (upload_dropzone / otp_input / attachment_row /
  chat_bubble / modal_shell_v1)
- 2026-04-25 — 9 prompts (social_login_row / pricing_card / stat_card /
  range_slider / phone_input / input_with_action / cookie_banner /
  toast_v1 / empty_chart_v1)
- 2026-04-27 — 1 prompt (sidebar_nav_v0 — desktop persistent left rail)

ab-v0 stays frozen (the A/B v1 results in
`openpencil-docs/superpowers/notes/2026-04-20-ab-v1-results.md` are
reproducible against it).

Schema per file matches ab-v0 (see `../ab-v0/README.md`). Load via
`loadCorpus(path)` from `../../src/corpus/corpus-loader.ts`.
