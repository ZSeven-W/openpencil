# Image acquisition

## Doubao generation by default

Preserve suitable existing/user-provided images. For new photo, illustration or
product assets needed by the design, use Doubao Desktop (豆包电脑版)'s available
built-in image generation tool by default. Follow `SKILL.md`: announce the intended
assets and possible quota/credit use in a brief progress update, then proceed
without a separate permission question while respecting host-required approvals.
Honor explicit no-generation/stock-only instructions throughout the task.

Keep the resolved target IDs through generation and import. If a call times out,
check for an existing output instead of resubmitting. Generate only the required
asset; do not flatten the editable interface into an image. Preserve requested
transparency and inspect the actual output. Reuse completed assets after an
import/save failure. If generation is unavailable or fails, use stock search;
do not switch implicitly to an API/CLI requiring separate credentials.

## Stock search fallback and explicit stock-only requests

For search slots, use `type: "image"`, explicit dimensions and a concrete English
`imageSearchQuery` describing subject, action and composition. Omit `imagePrompt`
for this search route.

After structural repair, reread current node IDs. Run the native `enrich_images`
with `root_ids` scoped to the requested roots or selected image nodes, then read
its `targets`, `resolved`, `failed` and `unresolved` counts. Never omit scope for
a selected-subtree edit: omission fills the whole active page.

`enrich_images` is search-only (Openverse with Wikimedia fallback). A slot with
only `imagePrompt` is explicitly Generate and will be skipped. For an unresolved
slot covered by search fallback or a stock-only instruction, set a concrete
`imageSearchQuery` and clear `imagePrompt` with `update_node` before enrichment.
Do not overwrite already populated or unrelated slots.

## Import, inspect and persist

For a managed canvas:

1. Persist a chosen generated asset in the user's project asset directory with a
   non-overwriting filename. Copy the actual output path returned by the tool or
   materialize its available output bytes; do not guess a path or rely only on a
   preview.
2. Read the current canvas and target. Confirm the document, page and original
   image node still match the task. A later selection change does not retarget an
   in-flight asset. Create an image node first if the design needs a new slot.
3. Call `canvas_import_image` with the absolute local `filePath`, exact `nodeId`,
   active `pageId` and fresh `expectedVersion` from `canvas_status`. It accepts
   PNG/JPEG/WebP up to 8 MiB, embeds the bytes in the document, clears resolved
   acquisition hints and preserves geometry. A version mismatch requires a fresh
   read and target review, not blind retry.
4. Render and view the affected root, inspect crop/subject/transparency and
   unrelated content, then use `canvas_save`. Reopen/read the saved `.op` and
   verify the image bytes are embedded, not a temporary path or `blob:` URL.
