# Static design captures and Preview evidence

Use full-content PNGs for reviewing generated page composition. A phone's
initial viewport is not a complete static design: it can end midway through a
row while additional authored content remains inside its scroll area.

```sh
cargo local build -p op-host-desktop
target/debug/openpencil-desktop --render-shots document.op static-shots 1 --full-content
```

The optional `--full-content` mode builds an ephemeral export snapshot. Explicit
vertical `scroll`, `scroll-area`, and `scroll-viewport` containers expand to their
measured content height; their flow ancestors grow and the footer follows the
complete body. Content width, card clipping and horizontal carousels retain
their authored geometry. Source files, editor canvas geometry, routes, events,
and Preview viewports remain unchanged. Geometry-preserving imports and
absolute or size-limited layouts retain the normal export behavior.

Retain ordinary viewport exports separately when checking the first screen:

```sh
target/debug/openpencil-desktop --render-shots document.op viewport-shots 1
```

This is an offscreen static export, not an interactive Preview capture. Preview
evidence must separately exercise scrolling, hit targets, fixed bars, state and
navigation. Never count a successful full-content export as proof that an item
is reachable in Preview.

For generated design batches:

- Put full-content PNGs directly inside each work's folder, beside its `.op` and
  original prompt. Label them `01-完整设计稿.png`, `02-完整设计稿.png`, etc.
- Put initial-view PNGs and native Preview captures in a separate
  `交互预览` folder and label their state or scroll position.
- Keep original model/pipeline results and acceptance failures. Re-exporting a
  source must not be described as regenerating or repairing that source.
- Record the binary and source hashes, export mode, layout bounds, and commands
  under `_过程记录`. Verify the `.op` hash before and after capture.
- Check the final content row and footer in the static PNG, and separately
  verify that Preview retains the original device viewport and scrolling.
- If a source uses an unsupported absolute/limited scroll layout, inspect it
  explicitly; the mode does not remove arbitrary clips to disguise overflow.
