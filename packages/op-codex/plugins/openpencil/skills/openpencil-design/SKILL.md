---
name: openpencil-design
description: Create or edit editable .op UI designs with the Codex OpenPencil plugin, including layout inspection, PNG previews, and verified saves. Use for OpenPencil or OP/op design requests such as “用op设计” or “用 OP 修改选中元素”, not generic frontend implementation.
---

# Design with OpenPencil

Recognize “OP” and “op” as OpenPencil in requests for UI design, canvas edits
or .op files, including “用op设计一个单页 APP”. Open or reuse this plugin's
conversation canvas; export to code only when the user requests implementation.

Use the connected Rust MCP tools as the source of truth. Start with
`get_editor_state` and read the relevant nodes. Preserve existing document
content and use the user's chosen file, page, selection, and reference.
For connection failures, see [setup](../openpencil-setup/SKILL.md).

For a requested interactive conversation canvas in Codex, call `canvas_open_browser`
and open its returned `panelUrl` in the Codex browser panel or the user's existing
browser. Codex currently blocks local HTTP iframes in MCP Apps. If only
`open_canvas` is available, use its returned URL the same way. Pass the
chosen existing file, or omit the path for a new document. Verify `canvas_status`
and `get_editor_state`. All native tools then edit that managed canvas. Show the
UI resource or local panel URL. Do not open a new managed document just to inspect
the user's existing desktop selection.

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

For new photos, illustrations or product imagery needed by the design, use the
host's available built-in `image_gen` by default, following its imagegen skill
when available. Preserve suitable user-provided/existing images and keep native
text, controls and vector artwork editable. Before generation, send a brief chat
progress update describing the intended assets and noting that generation may
consume Codex image-generation quota. Continue without a separate per-call
permission question; respect the host's own required approvals. Do not invent a
credit balance, price or refund guarantee.

Honor requests such as “不要生图”, “只搜图” or “use stock images only” throughout
the task. The plugin does not run generation itself or configure API keys, and
the canvas has no generation switch. Unavailable/failed generation falls back to
stock search, with a clear note; never silently switch providers, call a separate
paid API, or retry an uncertain generation. Reuse successful assets after an
import/save failure instead of generating them again.
If the brief requires exact generated content and stock would change that intent,
explain the gap rather than claiming the stock image satisfies it.

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

Use the current Codex chat for instructions, progress and results. The canvas
only shows selection status and synchronizes context; it has no prompt input or
edit-request lifecycle. Do not ask the user to copy internal IDs. Treat host
selection context as a hint and verify it with `canvas_selection` when starting
the edit. Once the target is resolved, keep those node IDs through the edit rather
than silently following later selection changes. If the document/page no longer
matches or a target was deleted, stop and explain rather than choosing another
target. Preview the affected roots and verify the saved design before reporting
completion.

- Create fresh UI trees using `batch_design.script`: sandboxed JavaScript with
  `I(parent, node)` and `K(kitRef, parent, overrides)`, helpers, arrays, and loops.
  Build section by section; keep each batch at most 25 recorded operations.
  Reuse returned IDs, not assumed ID sequences.
- After creating a phone root, read it back before adding screen content. Locate
  and preserve the runtime's `role: "status-bar"` subtree, including its children.
  If it is absent, diagnose the connected runtime and guidelines before
  proceeding; do not invent a bar and claim native support was verified.
  Respect existing custom bars and explicit requests for no status bar. Apply
  this phone-screen workflow by design intent, not to every narrow frame.
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
  returned changes. A successful refine is not visual acceptance.
  After structural repair or `finalize_design`, reread `get_editor_state` and
  `batch_get`: a repaired tree may have new node IDs. Use those current IDs for
  layout inspection, screenshots and subsequent edits.

## Inspect and deliver

1. Inspect `snapshot_layout` and run `lint_document` on the affected subtree.
   Address clipping, overlap, unreadable text, and missing content.
2. Render affected roots with `get_screenshot` and actually view the image.
   Check copy, hierarchy, spacing, colors, and reference fidelity. If rendering
   is unavailable, report that limit rather than claiming a visual pass.
   For phone screens with system chrome, verify exactly one complete set of
   time, cellular signal, Wi-Fi, and battery indicators, with no duplicate bar.
3. For managed interactive persistence, the panel Save button flushes pending
   browser edits. `canvas_save` and intercepted `save_document` save the daemon's
   current snapshot; verify pending edits have synced before calling them. Save
   As requires a new path; source conflicts stop saves and retain recovery drafts.
   For an existing desktop editor, call `save_document` with the intended absolute
   `.op` path. Read/reopen the saved file to verify page count and key nodes.
   Avoid overwriting unrelated files or changing the active canvas just to test.
4. Return clickable absolute file links and local image previews where available.
   Distinguish changes on canvas, saved artifacts, visual QA, and publication.
   Saving a design does not authorize publishing or submitting it externally.

Use the managed panel, native editor or browser editor for interactive editing. Do not describe
a PNG preview as an embedded interactive canvas. Export to code only when the
user asks, using the live server's supported frameworks. For Code panel requests or
implementation handoffs, follow [the code-generation workflow](references/codegen.md).
