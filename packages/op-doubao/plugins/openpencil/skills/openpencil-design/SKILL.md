---
name: openpencil-design
description: Create or edit editable .op UI designs with the Doubao Desktop (豆包办公 / 豆包电脑版) OpenPencil connector, including the interactive Rust canvas, layout inspection, PNG previews, and verified saves. Use for OpenPencil or OP/op design requests such as “用op设计” or “用 OP 修改选中元素”, not generic frontend implementation.
---

# Design with OpenPencil in Doubao Desktop (豆包办公 / 豆包电脑版)

Recognize “OP” and “op” as OpenPencil in requests for UI design, canvas edits
or `.op` files, including “用op设计一个单页 APP”. Open or reuse this connector's
conversation canvas; export to code only when the user requests implementation.

Use the connected Rust MCP tools (`openpencil` connector) as the source of truth.
Start with `get_editor_state` and read the relevant nodes. Preserve existing
document content and use the user's chosen file, page, selection, and reference.
For connection failures, see [setup](../openpencil-setup/SKILL.md).

## Open the interactive canvas

For an interactive canvas in Doubao Desktop Work Mode (工作任务模式), call
`canvas_open_browser` or `open_canvas` (passing the user's existing absolute
`.op` path, or omitting it for a new document). Present the returned local
`panelUrl` so the user or Doubao's browser can open the interactive Rust WASM
canvas directly while preserving its exact URL and fragment. Verify
`canvas_status` and `get_editor_state` before making edits; all native tools then
edit that managed canvas. Do not open a new managed document just to inspect an
existing desktop editor selection.

## Establish the output

For a new design, resolve **one page or multiple pages** explicitly if the brief
does not establish it; continue inspecting references while awaiting the answer.
Do not silently choose three pages. Distinguish document pages from top-level
frames/boards. Establish viewport and content goals from the brief. Preserve
provided copy; do not reduce real content to fit a decorative mockup.

Load only the relevant connected design guidelines/style guides/UI kits.
Use [node and layout reference](references/node-schema.md) for field conventions;
the live tool schemas and guidelines take precedence over this bundled snapshot.
Reuse the document's variables, components, and style before inventing new ones.
For a new phone screen, load `get_guidelines` with `topic: "mobile"` before
creating its root frame. Use the Rust runtime's canonical status bar; do not
hand-draw a replacement or use emoji for system indicators.

## Image acquisition

For new photos, illustrations or product imagery needed by the design, use
Doubao's built-in image-generation capability (e.g. Seedream) by default when
available. Preserve suitable user-provided/existing images and keep native text,
controls and vector artwork editable. Before generation, send a brief chat
progress update describing the intended assets and noting that generation may
consume Doubao quota/credits. Continue without a separate per-call permission
question while respecting host-required approvals. Do not invent a credit
balance, price or refund claim.

Honor requests such as “不要生图”, “只搜图” or “use stock images only” throughout
the task. The connector does not invoke generation providers itself or store API
keys. Unavailable/failed generation falls back to stock search with a clear note;
never silently switch providers, call a separate paid API, or retry an uncertain
generation. Reuse successful assets after an import/save failure instead of
generating them again.

For image slots, search fallback and local-asset import, read the
[image workflow](references/images.md). Images are assets within editable UI;
keep text, controls and layout as native nodes.

## Build and edit

For prompts such as “modify the selected elements”, call `canvas_selection` on
the managed canvas immediately before planning the edit. It returns `pageId`,
`nodeIds`, names, types and parent IDs. Read the selected nodes with `batch_get`;
limit edits to those nodes and their descendants. An empty selection is not a
request to edit the whole page: ask the user to select a target. For a desktop
editor without a managed canvas, use `get_editor_state` and read its selection.

Use the current Doubao chat/task for instructions, progress and results. Once
the target is resolved, keep those node IDs through the edit rather than silently
following later selection changes. If the document/page no longer matches or a
target was deleted, stop and explain rather than choosing another target. Preview
the affected roots and verify the saved design before reporting completion.

- Create fresh UI trees using `batch_design.script`: sandboxed JavaScript with
  `I(parent, node)` and `K(kitRef, parent, overrides)`, helpers, arrays, and loops.
  Build section by section; keep each batch at most 25 recorded operations.
  Reuse returned IDs, not assumed ID sequences.
- After creating a phone root, read it back before adding screen content. Locate
  and preserve the runtime's `role: "status-bar"` subtree, including its children.
  If it is absent, diagnose the connected runtime and guidelines before
  proceeding; do not invent a bar and claim native support was verified.
  Respect existing custom bars and explicit requests for no status bar.
- Use `batch_design.operations` for editing existing nodes with `U/C/R/M/D/G`.
  Do not mix creation and edits in script, and do not use `I()` in operations.
  Prefer native UI kits (`list_ui_kits`) for standard controls.
- The batch is transactional. When it returns errors, repair the whole failing
  batch before resending. After a transport timeout, inspect the canvas first:
  a write may already have applied. Never automatically retry a mutation.
- Use horizontal/vertical layout, padding, gap, and container sizing deliberately.
  For repeated cards in a row, use consistent `fill_container` widths.
  Absolute x/y coordinates on children do not replace a proper flow layout.
- Use `design_refine` on the affected root when appropriate, then inspect the
  returned changes. After structural repair or `finalize_design`, reread
  `get_editor_state` and `batch_get` because repaired trees may have new IDs.

## Inspect and deliver

1. Inspect `snapshot_layout` and run `lint_document` on the affected subtree.
   Address clipping, overlap, unreadable text, and missing content.
2. Render affected roots with `get_screenshot` and actually view the image.
   Check copy, hierarchy, spacing, colors, and reference fidelity.
   For phone screens with system chrome, verify exactly one complete set of
   time, cellular signal, Wi-Fi, and battery indicators.
3. For managed interactive persistence, the panel Save button flushes pending
   browser edits. `canvas_save` and intercepted `save_document` save the daemon's
   current snapshot; verify pending edits have synced before calling them. Save
   As requires a new path; source conflicts stop saves and retain recovery drafts.
   Read/reopen the saved file to verify page count and key nodes.
4. Return clickable absolute file links and local image previews where available.
   Distinguish changes on canvas, saved artifacts, visual QA, and publication.

Use the browser panel or native editor for interactive editing. Do not describe
a PNG preview as an embedded interactive canvas. Export to code only when the
user asks, following [the code-generation workflow](references/codegen.md).
