# PATTERN-AUDIT — dsh-openpencil (2026-09-06)

Method: source read focused on the defect-prone surfaces (design-quality,
design-visual-review, presentation-hydration, viewer-assets, editor modules),
baseline suites in an **isolated clone** (see P6 note), and a **live drive**:
the packed plugin installed into a private headless profile (`sweep-op`) and
driven with natural-language design tasks through the real pipeline.

## Baseline

Run in an isolated clone at `HEAD 99e05cd` with `pnpm install --frozen-lockfile`
(because this worktree's `node_modules` is a symlink into the shared
`~/.dsh/profiles/node_modules` tree at 0.1.1-rc.2 — see P6):

| Suite | Result |
|---|---|
| `test:host-api` (206 tests) | **204 pass / 2 fail** — both failures are the `viewer-assets` HTTP route absent because the staged wasm assets (`lib/viewer-assets`, built by `sync:viewer-assets`) do not exist on this machine. Failures are loud, not skips (correct direction). **not verified: viewer-asset route live behavior.** |
| `test:mcp` (7) | 7/7 pass |
| `test:client` (99) | 99/99 pass |

Note: on this machine as checked out, `pnpm run build` **cannot succeed** — the
shared module tree resolves `@deepseek-ai/dsh-client-runtime@0.1.1-rc.2` while
the lockfile pins `0.1.0-rc.6`, and the client TS build breaks on the API
changes (`sessionId` prop, `conversation.input.dock` event). The repo itself is
consistent; the machine-local symlink is not. All verification was therefore
done in the isolated clone.

## Live drive (the part that found the defect)

Two one-shot natural-language design tasks through the real pipeline:

1. "design a landing page … publish it" — **failed terminally** (see P2). The
   agent authored text nav-links with `width:"fit_content"`, the quality gate
   flagged the 44px touch-target minimum, the agent followed the diagnostic
   literally and set `width:44` with `padding:[0,16]`, and the container-flow
   rule then rejected that shape and returned `canContinue:false`. Draft rolled
   back; no `.op` published. Honest failure, wrong destination.
2. Same task with the `minWidth` hint — **published successfully**, both batches
   zero diagnostics, visual review ran (`needs_visual_review` with digest +
   checklist, then accept), final PNG rendered. The digest was structural and
   truthful (footer detected as final region, etc.).

## P1 — documentation vs implementation drift

No confirmed defect. Tool descriptions (`design-draft-tools.ts`) were read
against schemas and controller behavior: two-batch contract, repair-target
authorization for the U-only third script, engine enum, skip_visual_review all
match the code. Not every README locale was diffed line-by-line — **not
verified: 14 non-primary READMEs**.

## P2 — boundary and limit sanity

**1 confirmed (fixed).** The touch-target diagnostic for interactive roles said
only "missing a 44px minimum on width" — while a *fixed* `width:44` on a padded
horizontal hitbox with a text child is unsatisfiable (container-flow needs
padding + label > 44). Following the hint as written walks the agent into a
terminal `canContinue:false`. The repair-target machinery already computed the
correct patch (`minWidth:44` when sizing is `fit_content`), but the human/model
facing *diagnostic text* did not say so. Fixed: the hint now names
`minWidth/minHeight`, says to keep `fit_content` sizing, and warns that a fixed
44px fails container-flow terminally. Regression tests pin: (a) the
`fit_content + minWidth:44 + padding + text` shape satisfies BOTH rules, (b) the
hint text mentions minWidth/fit_content, (c) a fixed 44px hitbox still trips
container-flow (the constraint itself is correct and stays).

Boundaries reviewed and sound: every constant in design-quality (issue caps,
rule caps, node budget with loud `node-limit` diagnostic, 512 repair targets
with an omitted-count summary), digest bounds with visible `… N more regions
elided`, visual-review batch caps (16 calls / 6 KiB), hydration body/response
caps with HTTP errors, traversal-proof viewer-asset paths.

## P3 — timing and ordering assumptions

No confirmed defect. All editor-daemon waits are explicit bounded deadlines
with readable timeout errors (start 20 s, ready 15 s, stop 3 s, reset 8 s);
recovery TTL 7 d; hydration TTL 15 min with size-capped ring. Evidence is
sha256-pinned rather than bound to transient node identity — the dsh-qa
defect-2 shape (replay breaks on the second pass) is structurally avoided.

## P4 — honesty of status

No confirmed defect, and the live drive actively confirmed the honest
direction: terminal failures return `canContinue:false` with the full
diagnostic list, the agent report said "no .op file exists", publication is
atomic and only claimed after the gates pass, and the visual-review stage is a
digest the model must explicitly accept (no fabricated model reasoning enters
the artifact — the digest is deterministic code, not model narration).

## P5 — evidence and report integrity

No confirmed defect. presentation-hydration re-projects metadata only for
sha-matched documents, tombstones for revoked grants, strict route validation;
renderer artifacts carry bytes+sha256; viewer assets are manifest-verified
(sha-pinned, immutable revision); no unbounded attacker-influenced content
reaches a human-readable product unmarked (digest truncation is visibly
marked).

## P6 — coverage illusion

**1 environmental finding (documented, not fixed here).** The tests are real
(they fail loudly when staged assets are missing — the opposite of the smoke
skips in dsh-ios/android). But: (a) this machine's worktree cannot build at all
due to the `node_modules` symlink drift — anyone running the suite here gets a
build failure before any test executes; (b) `verify:packages` / `pack:artifact`
gates are not part of any `test:*` aggregate. Recorded as environment + wiring
notes; the new regression tests join `test:host-api`'s file set via
`tests/pattern-audit-regressions.test.mjs` (run by the same `node --test`
invocation when added to the suite — **note for the releaser: add the file to
`test:host-api`'s list**; it was not added to package.json to keep this commit
source-only, since scripts were untestable in this worktree).

## Fixes and red/green proof

Red (hint reverted to the old text, same tests):

```
✖ the touch-target diagnostic names minWidth/minHeight instead of inviting a fixed 44px width
ℹ pass 2   ℹ fail 1
```

Green (fix applied):

```
✔ fit_content + minWidth 44 nav-link satisfies BOTH touch-target and container-flow …
✔ the touch-target diagnostic names minWidth/minHeight instead of inviting a fixed 44px width …
✔ a fixed 44px hitbox with padding + text child still trips container-flow …
ℹ pass 3   ℹ fail 0
```

Full `test:host-api` after the fix: 204/206 (the 2 failures are the pre-existing
environmental viewer-assets ones). Post-fix live re-drive was not rerun end to
end (the second live task already exercised the minWidth path successfully);
the hint change is text-only over the same rule set.

## Would the pre-existing verification have caught this?

No. The 204 passing tests assert the *rules*; none of them asserts that a
rule's remediation *hint* keeps a satisfiable solution reachable, and none
drives the model-follows-the-hint loop. Only the live natural-language drive
exposed the contradiction, on the first attempt.
