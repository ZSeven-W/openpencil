// Emit the TS shadcn UIKit (apps/web/src/uikit/kits/) as JSON for
// embedding into the Rust editor core (GAP #23). The kit modules are
// PenNode JSON literals with type-only `@/...` imports, so Bun can
// execute them directly without the web app's path aliases.
//
// Run from the repo root after editing the TS kit:
//   bun tools/export-shadcn-kit.ts
//
// Output: crates/op-editor-core/assets/shadcn-kit.json — embedded via
// `include_str!` in crates/op-editor-core/src/uikit_shadcn.rs.
import { SHADCN_KIT_DOCUMENT } from '../apps/web/src/uikit/kits/shadcn-kit';
import { SHADCN_KIT_META } from '../apps/web/src/uikit/kits/shadcn-kit-meta';

const out = {
  document: SHADCN_KIT_DOCUMENT,
  meta: SHADCN_KIT_META,
};

const dest = new URL('../crates/op-editor-core/assets/shadcn-kit.json', import.meta.url);
await Bun.write(dest, `${JSON.stringify(out, null, 2)}\n`);
console.log(
  `wrote ${dest.pathname} (${SHADCN_KIT_DOCUMENT.children.length} kit document children)`,
);
