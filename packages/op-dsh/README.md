<p align="center">
  <img src="./docs/images/dsh-openpencil-logo.png" alt="DSH OpenPencil" width="120" />
</p>

<h1 align="center">DSH OpenPencil</h1>

<p align="center">
  <strong>The DeepSeek Harness plugin for OpenPencil — preview, inspect, and edit real <code>.op</code> documents inside a conversation.</strong><br />
  <sub>Exact Multi-Frame Previews &bull; Interactive Canvas &bull; Managed Editor &bull; Agent-Native Design Tools</sub>
</p>

<p align="center">
  <sub>npm: <a href="https://www.npmjs.com/package/@zseven-w/dsh-openpencil"><code>@zseven-w/dsh-openpencil</code></a> · Current plugin release: <code>0.1.0-rc.16</code> · Editor panel verified on DSH Desktop <code>0.2.1-rc.3</code></sub>
</p>

<p align="center">
  <a href="./README.md"><b>English</b></a> · <a href="./README.zh.md">简体中文</a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md">हिन्दी</a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
</p>

<p align="center">
  <a href="https://www.npmjs.com/package/@zseven-w/dsh-openpencil"><img src="https://img.shields.io/npm/v/%40zseven-w%2Fdsh-openpencil?style=flat&color=cfb537" alt="npm" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/dsh-check.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/dsh-check.yml?label=CI" alt="CI" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/stargazers"><img src="https://img.shields.io/github/stars/ZSeven-W/openpencil?style=flat&color=cfb537" alt="Stars" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/blob/main/LICENSE"><img src="https://img.shields.io/github/license/ZSeven-W/openpencil?color=64748b" alt="License" /></a>
  <a href="https://discord.gg/h9Fmyy6pVh"><img src="https://img.shields.io/badge/Discord-Join%20chat-5865F2?logo=discord&logoColor=white" alt="Discord" /></a>
</p>

<br />

<p align="center">
  <img src="./docs/images/dsh-openpencil-overview.png" alt="DSH OpenPencil — multi-frame preview and sidebar editor" width="100%" />
</p>
<p align="center"><sub>Exact multi-frame <code>.op</code> previews with an interactive canvas and the managed editor workbench</sub></p>

## Why DSH OpenPencil

DSH OpenPencil connects [DeepSeek Harness](https://github.com/deepseek-ai/DSH) with [OpenPencil](https://github.com/ZSeven-W/openpencil) so an Agent drives a real, editable, interactive design canvas instead of returning a generated image.

<table>
<tr>
<td width="50%">

### 🖼️ Exact Multi-Frame Previews

The installed OpenPencil headless exporter renders design-faithful previews: the first top-level frame as a large replay-safe PNG, plus a horizontally scrollable thumbnail rail, click-to-select, and previous/next navigation for multi-frame documents.

</td>
<td width="50%">

### 🗺️ Interactive Canvas

"Open interactive canvas" lazily mounts the read-only OpenPencil Web SDK with pan, zoom, and fit — inspect any page, nested node, or inactive page without leaving the conversation.

</td>
</tr>
<tr>
<td width="50%">

### ✏️ Managed Editor

With `editable: true`, the edit action opens the managed OpenPencil editor — selection, layers, properties, drawing tools, undo/redo, and explicit save semantics — in a resizable right-hand workbench with a full-screen option.

</td>
<td width="50%">

### 🤖 Agent-Native Design Tools

Five direct-canvas tools plus six `openpencil_pipeline_*` tools let the Agent create, inspect, refine, publish, modify, and read a real canvas through managed OpenPencil runtimes.

</td>
</tr>
<tr>
<td width="50%">

### 🔐 Capability-Gated Grants

Image and document grants are signed, hash-bound capabilities. Browser metadata never exposes an arbitrary host path, and signed preview/editor capabilities never enter the canonical tool result or model context.

</td>
<td width="50%">

### ⚡ Transactional Safety

A full-pipeline document stays in a private unpublished draft until every native and DSH quality gate passes. Publication never overwrites an existing path, and aborts or failed batches leave no empty target behind.

</td>
</tr>
<tr>
<td width="50%">

### 🌍 Follows DSH Look & Feel

The tool card and managed editor follow DSH's Chinese/English locale and light/dark theme without reloading the editing session.

</td>
<td width="50%">

### 🎯 One Complete Workflow

"Requirement → private draft → semantic batches → live user previews → deterministic structure/layout/quality validation → exact-PNG-integrity atomic publish" — one complete loop inside DSH.

</td>
</tr>
</table>

## Install into DSH

DSH is a separate package. Install it once if you do not already have it:

```sh
npm install -g @deepseek-ai/dsh@latest
```

Then add the plugin to a profile and start the web app:

```sh
dsh plugin --profile web add @zseven-w/dsh-openpencil@latest
dsh web
```

The plugin is still on a prerelease line. Releases use the npm `latest` tag; install from `latest` to receive the current version. The older `next` tag is not the active release channel.

For local development, build the checkout, link its absolute path into the Web profile, and then restart DSH:

```sh
pnpm run build
dsh plugin --profile web add link:/absolute/path/to/dsh-openpencil
dsh web
```

The `link:` dependency exposes subsequent rebuilds from this checkout, but DSH must be fully restarted after replacing the profile dependency because the shipped Web profile does not hot-reload host bundles by default.

Prefer not to install DSH globally? Run the same two steps through `pnpm dlx`:

```sh
pnpm dlx --package=@deepseek-ai/dsh@latest dsh plugin --profile web add @zseven-w/dsh-openpencil@latest
pnpm dlx --package=@deepseek-ai/dsh@latest dsh web
```

> The OpenPencil plugin is public and requires no npm token. If the DSH prerelease itself requires registry authentication, keep that credential in a user-level or temporary npm config outside the checkout. This repository intentionally contains no registry credentials.

## Design Tools

Optional style selection uses `openpencil_style_plan` → `openpencil_style_confirm` → `openpencil_pipeline_begin` with the original brief and `style_selection` handle. Catalog browsing and exact-guide choice work without a model recommendation; the recommendation provider is off by default. This handoff supports the script engine. App-agent and commerce handoffs remain unsupported; ordinary app-agent generation still works.

| Tool | What it does |
| --- | --- |
| `openpencil_new` | Compatible fast path for simple jobs: runs one transactional QuickJS `batch_design` script, publishes with create-if-absent semantics, and returns an editable presentation. Prefer the full pipeline below for production design. |
| `openpencil_pipeline_begin` | Starts an owner-scoped private draft, internally seeds its only root, returns `rootNodeId`, `continuationStyle`, and the compact runtime-matched canvas/build contract, then immediately opens the same live draft in the sidebar; the target `.op` remains unpublished. |
| `openpencil_pipeline_context` | Loads one bounded guideline, style, theme, or UI-kit detail that is genuinely missing from the begin contract; it is not a startup-context refresh loop. |
| `openpencil_pipeline_batch` | Runs at most two direct native `I(...)`/`K(...)` generation scripts: the bounded first-visible-viewport script returned by begin's `next`, then one script for every remaining region. Before exposing the second preview, an authored-structure gate rejects empty category/product helpers, atomically restores the first-batch snapshot, and permits one corrected second script without consuming the two-script budget. Each successful transaction attempts an exact user-facing PNG and must be followed immediately, without narration, by its returned `next`. A third generation script is rejected; only the complete repair gate returned by finish may authorize one bounded `U(...)` QuickJS repair script. |
| `openpencil_pipeline_inspect` | Provides manual diagnostics only when the user explicitly requests them; ordinary generation never uses it as a preview or model-inspection step. |
| `openpencil_pipeline_finish` | In one healthy call, finalizes and validates the document, automatically renders the exact post-final root preview, and atomically publishes with `createIfAbsent`. One `U(...)`-only repair and one final finish are authorized only when the result is `needs_correction`, `canContinue: true`, and contains a complete non-empty `repairTargets` array with `omitted: 0`; every other non-published result is terminal. |
| `openpencil_pipeline_abort` | Discards the unpublished draft without creating the target file. |
| `openpencil_create` | Applies a transactional `batch_design` program to generate or restructure nodes on an existing live canvas. |
| `openpencil_edit` | Modifies an explicit node or the single node selected by the user. |
| `openpencil_render` | Creates an immutable, content-addressed `.op` snapshot and renders every top-level frame on the active page — optional `scale` and `editable`. |
| `openpencil_selection` | Reads the exact nodes selected in the live editor canvas. |

## Agent Design Workflow
### Model tier for design turns

Generation quality tracks the model driving the pipeline turns. The two-batch
contract works on fast tiers, but layout judgment, copywriting, and contract
adherence improve markedly on a stronger or reasoning-enabled model. For
design-heavy profiles, prefer a non-flash tier (for example
`deepseek-v4` with `reasoningEffort` enabled over `deepseek-v4-flash`) in the
profile's `agent-default-model`; keep the fast tier for chat-only profiles
where design output is incidental.


Desktop commerce Hero geometry is validated before batch two: every right-side visual child and layered shape must stay inside the fixed Hero inner width and height. An oversized visual is rolled back with the entire first batch instead of inflating the live canvas or reaching final publication. Generation receipts expose only committed node mappings and previews; native diagnostics are deferred to finish so they produce one exact repair transaction instead of a speculative mid-generation loop.

For an ordinary one-line request, call `openpencil_pipeline_begin` immediately without spending a model step loading the optional skill, then use the fixed fast path: the bounded first direct-JS batch from begin's `next` → a second and final direct-JS batch containing every remaining region → one `openpencil_pipeline_finish`. The current first batch renders only the **first visible viewport**: a complete navigation/header plus a polished primary hero/content selected from the brief, capped at **32 `I`/`K` calls and 8 KiB**. This budget preserves real structure and visual hierarchy without reopening an unbounded generation loop; secondary cards, detail, and below-fold content wait for the second batch. For commerce, begin now selects the concrete App-aligned `ecommerce-modern-light` recipe before drawing: a white base with warm-tinted section rhythm, restrained orange actions, 1120px centered content, a 56px Hero display, and visible copy in the language of the user's request (a Chinese request keeps Chinese copy, except for an optional short ASCII brand). A desktop commerce Hero is a full-width horizontal frame with `padding:[64,160]`; its inner content is copy 512px + gap 64px + product visual 448px, and both headline and subtitle use `width:"fill_container"`. Never combine a fixed 1120px Hero width with that padding. Generic commerce creates the visual directly under Hero as `I(hero,{type:"image",name:"Hero product image",width:448,height:360,imageSearchQuery:"gray loveseat isolated photo"})`: `imageSearchQuery` is a direct node field, never an `image:{...}` object, and the image is neither wrapped nor mixed with decorative shapes or reused by a product card. It is enriched as soon as the first batch commits. An explicit `layout:"none"` stack of 4–6 positioned layers, including at least one ellipse/path, is allowed only when the user explicitly requests illustration or no photos. Plain stacked rounded rectangles, an empty right field, overflow, or an undersized decorative tile are rejected and atomically rolled back before batch two. Platform and viewport come from the latest direct user request: a model-expanded `brief` cannot silently turn an unspecified request into a mobile canvas. When the brief provides no brand, immediately use a neutral shop name or short placeholder that fits its context, without comparing naming options. When no logo asset is supplied, use a text-only brand; do not invent a letter badge or give a text node a background, fixed height, or effects. Begin creates the only root and opens the private live canvas immediately. After every successful begin or batch call, make the next required call immediately without narration, planning, comparison, inspection, or any unrelated tool call. A healthy finish runs native finalization first, enriches the canonical image slots, renders the post-final user preview, and publishes in that same call.

The App-aligned commerce preset fixes the 64px Header at `padding:[0,160]`: the Header is a `navbar`, its Nav is one `nav-links` collection of 44px `nav-link` items, and Header actions are a `toolbar` of 44×44 `icon-button` frames rather than bare icons. The 160×48 primary CTA is a `button`, contains its own label, and uses the AA pair `#C2410C`/`#FFFFFF`. All three product cards stay in one coherent collection. For a generic home brief, the exact runtime-validated set is `gray armchair isolated photo`, `artemide tolomeo lamp photo`, and `potted plant isolated photo`; the last card must be labeled as a potted plant rather than a vase.

The begin result is the compact authoritative run contract. Omit `path` unless the user explicitly named one; the plugin creates the concrete collision-resistant `.op` filename, avoiding template syntax or a retrying begin/abort loop. Preserve its exact `rootNodeId`, `continuationStyle`, `canvas`, and `buildContract`, then implement the bounded first script exactly as `next` specifies without broadening its scope or inserting a user-facing reasoning message. After its automatic preview, write the second and final script for all remaining regions with arrays and loops. This direct two-script flow keeps execution bounded and responsive.

Both generation scripts use direct `I(...)`/`K(...)` QuickJS plus begin's exact `rootNodeId`, `continuationStyle`, and `buildContract`. Inside `run_code`, construct the embedded source with a `String.raw` tagged template so escapes and intended text newlines reach QuickJS unchanged. Use this exact multiline wrapper:

```js
const draftId = "<exact begin.draftId>";
const script = String.raw`...`;
const r = await tools.openpencil_pipeline_batch({ draftId, script });
return r;
```

Quote the exact `begin.draftId` into the standalone `draftId` string first, then declare `script`. The fixed tool argument object contains only `draftId` and `script`; never append `canvasWidth`, another field, or return inside it. Return only `r` without logging, printing, or stringifying it. Each QuickJS batch has a fresh scope: local bindings do not cross batches. `I` and `K` return opaque node-id strings, not mutable nodes: use a binding only as a later `I`/`K` parent and never assign `binding.x`, `binding.y`, or any other member. In batch 2, never recreate Page, App Content, Header, or Hero. Prefer attaching each new bound section rail directly to begin's `rootNodeId`. If batch 1 created a shared content wrapper, reuse it only through the exact nodeId returned by batch 1; never rebuild another wrapper with the same name.

`rootNodeId` is already the page frame: attach top-level regions directly, never create another Page/root wrapper. Only frame/group bindings may parent children; round artwork containing an icon uses a frame plus `cornerRadius`, never an ellipse parent. Every semantic container—Header, Nav, Search, Hero, Card, Section, Toolbar, Button, or CTA—must be assigned from `I`/`K` and immediately receive all intended visible children through that binding; never leave it empty or place its intended children as siblings. A minimal valid header is `const h=I(root,{type:"frame",name:"Header",layout:"horizontal"}); I(h,{type:"text",content:"Shop",fontFamily:"Inter, system-ui, sans-serif"});`. On mobile, only full-width chrome/full-bleed sections attach directly to root; bare text, icons, small controls, and section titles must be inside a bound rail/section with a 24px left/right gutter. Every category card contains a real visual tile plus its label; a mobile category item uses a 56×56 tile frame. Heterogeneous product cards never reuse one glyph as fake product art: use a matching distinct icon/shape or omit that media wrapper. A desktop commerce category rail spans the 1120px content width with `justifyContent:"space_between"`; its product rail contains exactly three equal `fill_container` cards with 24px gaps and no unused right tail. Every Button/CTA frame must immediately receive its visible text/icon child through the binding returned when that frame is created, never as a sibling. The minimum icon is `{type:'icon_font',name:'Search icon',iconFontName:'search',width:20,height:20}`: `name` is the layer label, while `iconFontName` selects the glyph and must come from the verified Lucide set, including `smartphone`/`camera` for electronics and `utensils`/`sandwich`/`croissant` for food; never substitute `lamp` for electronics or `coffee` for food. The broader verified set also includes `home`, `search`, `shopping-bag`, `shopping-cart`, `user`, `heart`, `star`, `plus`, `arrow-right`, `sparkles`, `sun`, `apple`, `snowflake`, `droplet`, `cookie`, `leaf`, `coffee`, `package`, `gift`, `baby`, `spray-can`, `lamp`, `sofa`, `armchair`, `shirt`, `headphones`, `laptop`, `monitor`, `gamepad-2`, `watch`, `palette`, and `cake`; compose shapes when none fits. Outside commerce, default to exactly one `type:"image"` leaf unless the user explicitly requests multiple images. Commerce uses exactly three distinct product images. Every query stays at four English words or fewer, names one concrete product, and never requests a lifestyle, collection, or broad category; generic home uses the exact validated set above. Isolated queries require positive isolated/cutout/white-background metadata rather than merely accepting a photograph. Multi-word queries must match at least two meaningful metadata tokens. Never put a lone small icon inside a large fixed product-media frame. Every Hero/Product/Art/Media frame has exactly one primary visual—one image with a concrete English `imageSearchQuery`, or one substantial composed-shape visual—never an image plus an icon placeholder; image nodes are leaves, never parents. Every generated text node explicitly uses `fontFamily: "Inter, system-ui, sans-serif"`; ordinary values are `fontSize: 16` and `lineHeight: 1.5`. Desktop keeps its bundled Inter, while the web host—which deliberately does not bundle Inter—uses the generic fallback without a missing-font prompt. Never use bare `Inter`. A CJK `lineHeight < 1.3` is raised to `1.5`, and a numeric text height becomes `fit_content` unless `textGrowth` explicitly requests `fixed-width-height`. Override size and line height only for headings or special typography. The runtime contract remains the single source for every other current node, style, script, and repair rule. A third generation script is rejected. Only the complete repair gate described below may authorize one third and final bounded QuickJS repair script, using `U(...)` only for the named targets.

Category rails use one distinct matching icon per label. Each `<label> icon tile` card contains a nested 56×56 `Category glyph surface` frame, then the `icon_font` inside that surface and the text label directly under the card; wrapper names must never contain `art`, `media`, or `image`, so native finalization cannot reinterpret an icon surface as a stock-photo slot. Complete each repeated product card's media (or intentionally omit it), name, and price before starting the next card. The release gate collapses an empty media shell, and a partially executed script can never publish a blank product section. The second-batch structure gate rolls back incomplete category/product composition, excessive category/product-rail height, mixed card widths, or compact-viewport overflow before preview or finish and allows only one corrected replacement script. Child-node post-processing preserves authored rail height instead of applying document-root expansion. Canonical post-final image enrichment is independent of optional earlier context enrichment; its zero-result retry keeps the trailing concrete subject phrase, rejects illustration/drawing/engraving/painting/catalog metadata for photo queries, and preserves the positive isolated-subject contract through retries. Publication stops if any requested product image remains unresolved instead of shipping empty or asymmetric product cards. Mobile finalization retains every authored category node in a clipped rail, while desktop rails are never rewritten by mobile-only normalization.

An unspecified invented brand uses a short ASCII store name; requested Unicode text is written literally, never as hand-authored JSON/JS escape sequences. `text_input` is a leaf: use its own placeholder, or create a named Search frame and attach the icon and hint text to that wrapper.

The live sidebar is continuous user feedback, and both successful generation batches attempt exact PNG preview cards for the user. Immediately after each successful commerce batch, the host runs one best-effort eight-second image-enrichment pass when that committed document contains unresolved queries, before rendering the live preview. The Hero can therefore show its real product photo after batch one, while the product rail fills after batch two, rather than all imagery appearing only after finish. Its header includes an explicit **Close** button; dirty drafts retain the existing save/confirmation guard. If `next` reports `previewUnavailable`, the script is already committed on the live canvas: follow `next`, never rerun the batch, and do not call `openpencil_pipeline_inspect` or `read_image`. Inspection is only for a user-requested diagnostic.

Finishing first enriches the authoritative document automatically, once, only when it contains a non-empty `imageSearchQuery`—no separate context step is needed and icon/shape art never triggers it. Up to three stock searches run concurrently under one unchanged 20-second deadline; writes remain deterministic in document order, so a three-card commerce row no longer lets the first provider ladder starve the later products. It then runs OpenPencil's native finalization, lint, and layout checks plus DSH's quality gate, automatically renders the exact post-final root PNG, and publishes atomically in one healthy call. Keep the finish return object in the same `run_code`. A repair is authorized if and only if the result has `stage: "needs_correction"`, `canContinue: true`, a complete non-empty `repairTargets` array, and `checks.dsh.repairTargetSummary.omitted: 0`, with every target containing `operation: "U"`, an exact non-empty `nodeId`, and a non-empty `patch`. Apply all targets together in exactly one `U(...)`-only batch, then call finish exactly once more without narration. After that repair attempt, anything except `published: true` is terminal. A thrown error, `canContinue: false`, or any other non-published finish result is also terminal: report it once and do not retry, inspect, read an image/context, abort, or start a replacement draft. `lint_document` Info plus only these Warning categories are non-blocking: `invisible-container`, `mixed-sibling-padding`, `sibling-inconsistency`, `text-effect`, and `text-explicit-height`. Other Warning categories—including `widget-a11y`, `excessive-frame-effects`, and `empty-path`—remain blocking, as does every Error. `imageSlots` remains observational when script mode cannot use `G(...)`. Native hard quality, contrast, and layout diagnostics plus DSH hard gates also remain blocking. When the retained result has `published: true`, it already contains the exact final PNG and live editor, so return it to the user and end the run immediately—do not call `openpencil_render`, `read_image`, or `openpencil_pipeline_inspect`. A failed gate or `openpencil_pipeline_abort` leaves the target absent. The document-scoped editable grant auto-opens the sidebar only when idle and always keeps **Edit canvas** for an explicit switch. Nested results hydrate their exact PNG instead of an unavailable-preview fallback. Historical or hydrated cards never auto-open.

Within the same running DSH service, switching browsers or reloading can recover a strictly parsed durable publication from `openpencil_new` or `openpencil_pipeline_finish` as the exact PNG plus an explicit **Edit canvas** action. A historical card never auto-opens the sidebar; the user must click that action. An ordinary historical `openpencil_render` remains read-only, and non-loopback connections never receive an editor grant.

The bundled `openpencil-design` skill remains an optional thin reference adapter; ordinary creation starts from the complete compact begin contract without loading it first. The current native tool schema remains authoritative, and the managed runtime does not depend on the desktop binary. `openpencil_new` remains a compatible single-batch path only for explicitly requested simple one-shot work.

Use `openpencil_create` and `openpencil_edit` only for an existing live canvas. Their edits remain unsaved until the editor Save action.

## Web Viewer Assets

DSH serves only `client.js` for a client plugin, so the OpenPencil ESM SDK, its WASM, and CanvasKit are staged as explicit same-origin assets:

```sh
pnpm run sync:viewer-assets
```

The sync command uses the enclosing OpenPencil monorepo root (`../..`), falling back to a sibling `../openpencil` checkout. Override it with `OPENPENCIL_ROOT` or `--openpencil-root`. A complete prebuilt asset directory can be selected with `DSHPLUGIN_OPENPENCIL_VIEWER_SOURCE`. Runtime lookup can be overridden with `DSHPLUGIN_OPENPENCIL_VIEWER_ASSET_DIR`.

Viewer assets are lazy-loaded only after the user opens the canvas. If they are absent or invalid, PNG preview remains available and no canvas button is advertised.

## Managed Editor

Editable sessions use OpenPencil's managed web host — the same architecture used by `op-vscode`. The plugin starts the single-tenant host only after an authorized user action and does not send `X-OpenPencil-Token` or `Authorization` to it; those request credentials remain reserved for online multi-tenant deployments. The managed daemon is loopback-only, inherits the supervisor's stdin lifetime, validates browser origins, and is closed when the editor session ends. DSH's ephemeral read-only browser proxy uses a separate per-attach capability and strips it before forwarding native requests. The editor surface is selected progressively: native Tool details when the host declares that seam, otherwise the plugin's right-hand workbench with resize and full-screen controls.

Desktop requests whose HTTP proxy omits `Origin` are accepted only after DSH authenticates the request and the plugin verifies its loopback peer and exact listening authority. Explicit browser origins retain the same-origin checks; without the Host authentication service, requests missing `Origin` remain rejected.

The editor panel was verified on the official DSH Desktop `0.2.1-rc.3` build: opening the canvas, editing a layer name, saving and reading the file back, reopening the editor, and opening a fresh card after restarting DSH. This verification covers the editor panel flow.

Startup uses a slow-mount-safe listening handshake: readiness probes begin only after the bundled host announces its bound address. No desktop OpenPencil installation is required.

Managed editor daemons keep settings and user resources in a private directory under `DSH_HOME`, separated by the pinned runtime version and revision. They do not read or rewrite the standalone OpenPencil settings file.

Published installations provide six native package targets: `darwin-arm64`, `darwin-x64`, `linux-arm64`, `linux-x64`, `win32-arm64`, and `win32-x64`; both Linux packages target glibc. The root package declares every platform package under exact-version `optionalDependencies`, allowing npm to select the matching package by OS and CPU. Each platform package stages `op-host-web-server`, the editor web bundle, and CanvasKit as one matching atomic runtime. New packages use the daemon's native deploy layout: the executable lives in `bin/`, the wasm-bindgen bundle in `bin/web-bundle/`, and CanvasKit in `bin/web-bundle/canvaskit/`. Release smoke tests start that executable with both asset-discovery environment variables removed, so the package must boot on its own. The managed editor therefore does not depend on `/Applications/OpenPencil.app`, `openpencil-desktop` on `PATH`, or an OpenPencil source checkout.

If DSH reloads or unloads the plugin while the canvas is dirty, the host keeps an opaque local recovery draft for up to seven days. Reopening the same source asks before restoring it into the live canvas; recovery never overwrites the `.op` file until the user explicitly saves.

Official six-platform packages receive their China and Global collaboration bootstrap endpoints during the protected release build, which validates the injected endpoints before publishing. A local self-build without that injection may override the bootstrap before starting DSH with `OPENPENCIL_COLLAB_BOOTSTRAP_URL=https://<your-host>/api/v1/collaboration/bootstrap`; the value must use `https` and exactly the `/api/v1/collaboration/bootstrap` path.

Cross-device canvas synchronization requires both the PC/DSH native runtime and the mobile app to be updated to the same OpenPencil release line that includes the current collaboration queue fix. Mixing an older mobile app with a newer PC runtime may still show remote cursors while failing to receive canvas commits.

When developing from this repository, build the editor Web bundle, build the native host, and then stage that matching runtime before launching DSH.

`pnpm run build:editor-web` runs OpenPencil's supported WASM bundle gate. It requires Bash, Cargo/Rust with the `wasm32-unknown-unknown` target, the `wasm-bindgen` CLI, Binaryen's `wasm-opt`, Node.js, and `gzip`; CanvasKit does not require EMSDK. The Web build does not use the collaboration bootstrap build variables. Before `pnpm run build:editor-runtime`, set both `OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN` and `OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL`; they are used only by the native Cargo build, which fails closed if either is missing. After both builds succeed, stage the runtime with the final command below.

```sh
pnpm run build:editor-web
pnpm run build:editor-runtime
pnpm run stage:editor-runtime
```

For a local, non-production smoke without the protected release bootstrap URLs, build the native host directly and stage it after the Web bundle. This validates the exact preview, MCP, and managed-editor paths; collaboration still requires an explicit runtime bootstrap override or an official release package.

```sh
cargo build --manifest-path ../../Cargo.toml --locked --release -p op-host-web-server
pnpm run stage:editor-runtime
```

Explicit runtime overrides are accepted only as one complete, matching set:

- `DSHPLUGIN_OPENPENCIL_EDITOR_BINARY` for `op-host-web-server`;
- `DSHPLUGIN_OPENPENCIL_EDITOR_WEB_BUNDLE_DIR` for the built editor web bundle;
- `DSHPLUGIN_OPENPENCIL_EDITOR_CANVASKIT_DIR` for the CanvasKit assets.

Providing only part of the set is invalid; the plugin does not combine custom paths with packaged runtime assets.

The older `0.1.0-rc.5` platform packages used the legacy `web/pkg` plus `web/canvaskit` layout. When starting one of those daemons directly rather than through the plugin, pass both native asset variables together:

```sh
OPENPENCIL_WEB_BUNDLE_DIR="<runtime-root>/web/pkg" \
OPENPENCIL_CANVASKIT_DIR="<runtime-root>/web/canvaskit" \
"<runtime-root>/bin/op-host-web-server" --serve-web
```

Saves use an optimistic source hash, an atomic replace, and a successor capability. If the source changes outside the editor, the plugin reports a conflict instead of overwriting it.

## Result Metadata

The model-visible result stays plain JSON. Browser-only `presentationMeta.$dshOpenPencil` carries additive grants for:

- `image`: PNG path, preview/download URLs, and real width/height;
- `frames`: every exact-rendered top-level frame in active-page order, including its node id/name/index and signed PNG URLs;
- `document`: source action path plus immutable snapshot URL, bytes, and SHA-256;
- `viewer`: revisioned SDK/WASM/CanvasKit URLs when the asset route is attached;
- `editor`: scoped launch/refresh capabilities when `editable: true` is authorized.

The result also records `renderer`, `rendererBinary`, `fidelity`, and any warnings. Existing PNG-only schema-v1 messages remain renderable.

DSH `0.1.1-rc.2` does not persist browser presentation metadata for tools nested under PTC/Code Mode. The plugin recovers that UI-only projection through a same-origin, session-bound endpoint: the browser sends only the session id, call id, and immutable document SHA-256, while the host resolves the authoritative result from the durable DSH session log and uses a short-lived in-process marker only to authorize recent live editing. Signed preview/editor capabilities never enter the canonical tool result or model context. Durable ordinary `openpencil_render` history remains read-only. A strictly parsed durable publication from `openpencil_new` or `openpencil_pipeline_finish` may receive a loopback-only editor grant only after an explicit user click; automatic sidebar opening is reserved for recent, trusted live results.

For bounded replay, nested metadata recovery accepts up to 128 top-level frames; larger Code Mode results remain available through their canonical JSON fallback.

## Current Limits

- Follow-up edits to an existing canvas require an already-open managed editor. Changes remain unsaved until the user invokes its Save action.
- The lightweight Web SDK canvas is read-only; full editing uses the separate managed editor surface. On DSH `0.1.1-rc.2`, the plugin uses the resizable right workbench with a full-screen option.
- The exact gallery covers top-level frames on the active page; the interactive canvas remains the way to inspect inactive pages and nested nodes.
- Render and snapshot caches still need a product-level retention policy.

## Project Structure

```text
packages/op-dsh/
├── src/                       Plugin sources (TypeScript)
│   ├── index.ts               Host plugin entry — Cordis service, tools, assets
│   ├── tool.ts / design-tools.ts / new-tool.ts   Host-side design tools
│   ├── renderer.ts            Exact OpenPencil renderer + Jian fallback
│   ├── editor-host.ts / editor-recovery.ts       Managed editor lifecycle + drafts
│   ├── viewer-assets.ts       Web SDK / WASM / CanvasKit asset staging
│   ├── mcp-client.ts          OpenPencil MCP connection
│   └── client/                Browser client — React workbench, gallery, selection dock
├── lib/                       Compiled output (published to npm)
├── scripts/                   Build helpers — viewer asset sync, client build, host tests
├── tests/                     Node test suites (client, host API, MCP, viewer assets)
├── docs/images/               Documentation screenshots
├── cordis.patch.yml           DSH bundle patch that mounts the plugin
├── tsconfig.json              Host / Node TypeScript config
└── tsconfig.client.json       Browser client TypeScript config
```

## Build and Verify

```sh
pnpm run sync:viewer-assets
pnpm run build
pnpm run test:viewer-assets
pnpm run test:client
pnpm run test:host /absolute/path/to/design.op 375 1091
```

Builds require Node 24.11 or newer and pnpm. DSH host/client packages are peer dependencies supplied by the target DSH profile. Build tools are resolved from local dev dependencies, the active linked DSH checkout, or an installed DSH source bundle; `DSH_SOURCE_ROOT` can select a source checkout explicitly. The lockfile pins standalone public build tooling when that environment is provisioned separately.

For a private DSH prerelease, keep the issued npm credential outside this repository (for example in a user-level or temporary `.npmrc`) and run the requested version directly:

```sh
pnpm dlx --package=@deepseek-ai/dsh@latest dsh web
```

Never commit `.npmrc`, `NPM_TOKEN`, or copied registry credentials. This repository ignores local npm configuration by default.

`test:host` performs a real exact render, validates PNG IHDR geometry and SHA-256, exercises immutable image/document capabilities over HTTP, starts the staged managed editor, pushes a live selection, applies an MCP mutation, saves, and proves that the latest bytes reopen. The expected dimensions are fixture-specific.

## Ecosystem

DSH OpenPencil is the DeepSeek Harness plugin for **[OpenPencil](https://github.com/ZSeven-W/openpencil)** — the world's first open-source AI-native vector design tool — and part of the **[ZSeven-W](https://github.com/ZSeven-W)** family of pure-Rust, AI-native tools.

| Project | What it is |
| ------- | ---------- |
| **[OpenPencil](https://github.com/ZSeven-W/openpencil)** | The design tool this plugin drives — prompt-to-canvas generation, concurrent agent teams, design-as-code `.op` files, and a built-in MCP server. The exact previews, interactive canvas, and managed editor here are powered by OpenPencil itself. |
| **[agent-rs](https://github.com/ZSeven-W/agent-rs)** | A pure-Rust async runtime for shipping LLM agents — multi-provider, tool-capable end-to-end, structured permissions, real MCP, zero `unsafe`. Powers OpenPencil's built-in agent runtime. |
| **[jian](https://github.com/ZSeven-W/jian)** | Pure-Rust, GPU-Skia UI framework — widgets, layout, events, and hot reload in one stack. OpenPencil's UI framework, and the source of this plugin's fallback renderer. |
| **[Zode](https://github.com/ZSeven-W/zode)** | Open-source, AI-native coding assistant for your terminal — reads your code, runs commands, and drives OpenPencil over MCP. |
| **[noema](https://github.com/ZSeven-W/noema)** | Local-first, non-vector memory system for coding agents — durable memory as inspectable files, works across runtimes. |
| **[openpencil-skill](https://github.com/ZSeven-W/openpencil-skill)** | The LLM skill plugin that teaches AI agents how to design with `op` — a companion to this DSH plugin. |

Sibling DSH plugins:

- [DSH Android](https://github.com/ZSeven-W/dsh-android) — a live Android emulator or USB device inside the conversation, driven entirely through adb
- [DSH Crew](https://github.com/ZSeven-W/dsh-crew) — dispatch work to DSH agents from Claude Code / Codex
- [DSH iOS](https://github.com/ZSeven-W/dsh-ios) — a live iOS Simulator and a USB-connected iPhone, inside the conversation
- [DSH Noema](https://github.com/ZSeven-W/dsh-noema) — long-term memory for DSH

## Contributing

Contributions are welcome! Fork and clone, create a branch, run `pnpm run build` and the test suites, commit with [Conventional Commits](https://www.conventionalcommits.org/), and open a PR against `main`.

## Community

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="https://raw.githubusercontent.com/ZSeven-W/openpencil/main/screenshot/logo-discord.svg" alt="Discord" width="16" />
  <strong> Join our Discord</strong>
</a>
— Ask questions, share designs, suggest features.

**Recognized community: [LINUX DO](https://linux.do/)**

## License

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W

Third-party components are listed in [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md).
