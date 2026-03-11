---
title: "feat: Jeans Opinionated Content Creation Workflow"
type: feat
status: active
date: 2026-03-10
deepened: 2026-03-10
origin: docs/brainstorms/2026-03-10-jeans-content-workflow-brainstorm.md
---

# feat: Jeans Opinionated Content Creation Workflow

## Enhancement Summary

**Deepened on:** 2026-03-10
**Sections enhanced:** All 8 phases
**Research focus:** Maximize reuse of OpenPencil's existing infrastructure, minimize new code

### Key Improvements
1. **Kit applicator uses existing batch history** — `startBatch()`/`endBatch()` + existing `setVariable()`/`setThemes()` calls. No new store logic.
2. **Vibe Kit editor IS the variables panel** — `variables-panel.tsx` already has theme tabs, variant columns, preset load/save, and inline editing. Just filter by `VibeCategory`.
3. **Template picker IS the component browser** — `component-browser-panel.tsx` with category tabs, search, grid cards, click-to-insert. Point at template kit.
4. **Kit import/export reuses UIKit pipeline** — `kit-import-export.ts` already collects variable refs and exports as `.pen` files.
5. **Video export reuses animation engine** — Playback loop + canvas bridge already do frame interpolation and Fabric mutation. Just add FFmpeg piping (~200 LOC).
6. **Extraction preview follows Figma import dialog pattern** — URL input → processing → preview modal.
7. **Kit persistence follows theme-preset-store pattern** — Zustand + localStorage, already proven.

### Infrastructure Reuse Map

| Jeans Feature | Reuses From | New Code |
|---------------|-------------|----------|
| Kit store | `theme-preset-store.ts` pattern | ~50 LOC (wrapper) |
| Kit applicator | `startBatch()` + `setVariable()` + `setThemes()` | ~30 LOC |
| Kit editor panel | `variables-panel.tsx` + `variable-row.tsx` | ~100 LOC (category grouping) |
| Template picker | `component-browser-panel.tsx` | ~80 LOC (wrapper) |
| Template instantiation | `deepCloneNode()` + `addPage()` | ~20 LOC |
| Kit import/export | `kit-import-export.ts` | ~40 LOC (metadata) |
| Format switch | `forcePageResync()` + `computeLayoutPositions()` | ~60 LOC |
| Variable resolution | Extend `resolveNodeForCanvas()` | ~40 LOC |
| Video export | `playback-loop.ts` + `canvas-bridge.ts` + `export.ts` | ~200 LOC |
| Extraction preview modal | `figma-import-dialog.tsx` pattern | ~150 LOC |
| Streaming progress | `ai-chat-panel.tsx` pattern | ~50 LOC |

**Estimated total new code: ~800-1000 LOC** (plus 5 template definitions and starter kits as data)

---

## Overview

Jeans layers an opinionated content creation workflow on top of OpenPencil's canvas engine, targeting solo creators/founders who need to produce LinkedIn content (carousel, video, post) quickly with minimal design skill. Three pillars: **Vibe Kit** (swappable design primitive system), **Token Extraction** (scrape any website → Vibe Kit), and **Fluid Grid** (auto-reflow across formats).

All styling flows from the active Vibe Kit. Swapping kits completely restyles all templates. Content auto-reflows when switching canvas format. Block-first editing with freeform escape hatch. (see brainstorm: `docs/brainstorms/2026-03-10-jeans-content-workflow-brainstorm.md`)

## Problem Statement / Motivation

Solo creators spend hours manually designing social content in Canva, Figma, or PowerPoint. They lack design skills to maintain brand consistency across formats. Repurposing a carousel into a story or post requires manual re-layout. OpenPencil has powerful design infrastructure (variables, auto-layout, animation, code generation) but no opinionated workflow that makes the common case effortless.

Jeans solves this by making content creation as simple as: pick your vibe → pick a template → fill in content → export to any format.

## Proposed Solution

### Architecture: Opinionated UX Layer on OpenPencil

Jeans is NOT a new engine — it's a curated UX on top of existing infrastructure. Every feature builds on proven OpenPencil systems. (see brainstorm: rejected alternatives — "Separate Content Layer" and "Web Component Renderer")

- **Vibe Kit** = existing `VariableDefinition` + `themes` with a schema contract and category grouping
- **Templates** = UIKit reusable components with all styling bound to `$variables`
- **Fluid Grid** = existing `canvas-layout-engine.ts` + `forcePageResync()` triggered on format change
- **Escape hatch** = natural: templates ARE PenNodes

### The Three Pillars

#### Pillar 1: Vibe Kit — Design Primitive System

A complete, swappable visual identity. All categories use existing `VariableDefinition` types (`color | number | boolean | string`):

| Category | Existing Type | Example Variables |
|----------|--------------|-------------------|
| Typography stack | `string` | `$font-heading`, `$font-body`, `$font-editorial` (e.g., `"Inter, sans-serif"`) |
| Color palettes | `color` | `$color-primary`, `$color-secondary`, `$color-accent`, `$color-bg`, `$color-surface`, `$color-text` |
| Texture palettes | `string` | `$texture-bg-1`, `$texture-pattern-1` (asset URLs) |
| LUT filters | `string` | `$lut-warm`, `$lut-cool`, `$lut-vintage` (asset URLs) |
| SFX | `string` | `$sfx-whoosh`, `$sfx-pop`, `$sfx-click` (asset URLs) |
| Animations | `string` | `$anim-enter`, `$anim-exit`, `$anim-emphasis` (preset names) |
| Transitions | `string` | `$transition-slide`, `$transition-fade` (preset names) |
| Strokes | `number` | `$stroke-default`, `$stroke-decorative` (thickness values) |
| Size tokens | `number` | `$size-heading-xl`, `$size-heading-lg`, `$size-body`, `$size-caption` |
| Space tokens | `number` | `$space-xs`, `$space-sm`, `$space-md`, `$space-lg`, `$space-xl` |
| Graphic sets | `string` | `$icon-set` (e.g., `"lucide"`, `"phosphor"`) |

#### Pillar 2: Token Extraction

Server-side Nitro API endpoint (follows existing `server/api/ai/` patterns):
1. Fetches target URL via native `fetch()` + DOM parsing (no Playwright needed for V1 — most sites work with raw HTML + CSS parsing)
2. Extracts computed styles: colors, fonts, spacing, textures
3. Maps to the canonical Vibe Kit variable schema
4. Returns a preview for user confirmation (follows `figma-import-dialog.tsx` modal pattern)
5. Saves as a Vibe Kit via existing preset persistence

#### Pillar 3: Fluid Grid System

Existing auto-layout engine does the work:
1. Root frame resizes to target format dimensions
2. `fill_container` children stretch/shrink via existing `computeLayoutPositions()`
3. `fit_content` containers recalculate from children
4. Fixed-size nodes scale proportionally
5. `forcePageResync()` triggers re-render

### V1 Scope: LinkedIn Only

- **Formats:** LinkedIn carousel (1080x1350), LinkedIn video (1080x1920), LinkedIn post (1200x1200)
- **Templates:** 5 essentials — title/intro, content, quote, stat/metric, CTA/closing
- **AI Assets:** Textures, LUTs, SFX generated on demand via Replicate/Fal
- **Onboarding:** No default kit — user extracts from their site or picks from gallery

## Technical Approach

### Phase 1: Vibe Kit Foundation

The schema, store, and variable resolution extensions. No new variable types — everything maps to existing `color | number | boolean | string`.

#### 1.1 Vibe Kit Schema

**New file: `src/vibekit/schema.ts`**

Canonical variable names that all kits must define and all templates reference. This is the CONTRACT between kits and templates:

```typescript
type VibeCategory =
  | 'typography' | 'color' | 'texture' | 'lut' | 'sfx'
  | 'animation' | 'transition' | 'stroke' | 'size' | 'space' | 'graphic'

interface VibeKitSchemaEntry {
  type: 'color' | 'number' | 'boolean' | 'string'
  fallback: string | number | boolean
  category: VibeCategory
}

const VIBE_KIT_SCHEMA: Record<string, VibeKitSchemaEntry> = {
  '$font-heading':    { type: 'string', fallback: 'Inter, sans-serif', category: 'typography' },
  '$color-primary':   { type: 'color', fallback: '#2563eb', category: 'color' },
  '$space-md':        { type: 'number', fallback: 16, category: 'space' },
  // ... full schema
}
```

#### 1.2 Vibe Kit Container Type

**New file: `src/types/vibekit.ts`**

```typescript
interface VibeKit {
  id: string
  name: string
  description?: string
  version: string
  sourceUrl?: string
  variables: Record<string, VariableDefinition>  // must satisfy VIBE_KIT_SCHEMA
  assets: Record<string, VibeAsset>
  metadata: {
    createdAt: string
    extractedFrom?: string
    generatedBy?: 'extraction' | 'ai' | 'manual'
  }
}

interface VibeAsset {
  type: 'texture' | 'lut' | 'sfx'
  url: string
  mimeType: string
  size?: number
}
```

#### 1.3 Vibe Kit Store

**New file: `src/stores/vibekit-store.ts`**

**Reuses:** `theme-preset-store.ts` pattern — Zustand + `persist` middleware + localStorage.

```typescript
// Store shape (follows theme-preset-store exactly)
interface VibeKitStoreState {
  activeKitId: string | null
  kits: Record<string, VibeKit>
  saveKit: (kit: VibeKit) => void
  removeKit: (kitId: string) => void
  setActiveKit: (kitId: string) => void
}
```

#### 1.4 Kit Applicator

**New file: `src/vibekit/kit-applicator.ts`**

**Reuses:** Existing document-store CRUD. No new store logic needed.

```typescript
// ~30 LOC — calls existing functions in batch
function applyKit(kit: VibeKit) {
  const { startBatch, endBatch } = useHistoryStore.getState()
  const { setVariable, setThemes } = useDocumentStore.getState()

  startBatch()
  setThemes(kit.themes ?? {})
  for (const [name, def] of Object.entries(kit.variables)) {
    setVariable(name, def)
  }
  endBatch()
  // Canvas re-renders automatically — use-canvas-sync detects variable changes
}
```

**Research insight:** `startBatch()`/`endBatch()` groups all variable changes into a single undo entry. Already proven with 300-state history limit.

#### 1.5 Extend Variable Resolution

**File: `src/variables/resolve-variables.ts`**

`resolveNodeForCanvas()` currently resolves `$refs` on: opacity, gap, padding, fill colors, stroke colors, effects. Extend to also resolve:
- `fontFamily` — resolve from `$font-*` string variables
- `fontSize` — resolve from `$size-*` number variables
- `cornerRadius` — resolve from `$radius-*` number variables
- `lineHeight`, `letterSpacing` — resolve from number variables

**File: `src/variables/replace-refs.ts`**

Extend `replaceVariableRefsInTree()` for same properties.

**Research insight:** The resolution pattern is consistent — check `isVariableRef(value)`, if true call `resolveVariableRef()`. Adding new properties is ~5 lines each.

#### Phase 1 Deliverables

- [x] `VIBE_KIT_SCHEMA` with canonical variable names (`src/vibekit/schema.ts`)
- [x] `VibeKit` and `VibeAsset` types (`src/types/vibekit.ts`)
- [x] `vibekit-store.ts` following `theme-preset-store` pattern (`src/stores/vibekit-store.ts`)
- [x] `applyKit()` using `startBatch()` + existing `setVariable()`/`setThemes()` (`src/vibekit/kit-applicator.ts`)
- [x] Extended `resolveNodeForCanvas()` for fontFamily, fontSize, cornerRadius, lineHeight, letterSpacing (`src/variables/resolve-variables.ts`)
- [x] Extended `replaceVariableRefsInTree()` for same (`src/variables/replace-refs.ts`)
- [x] Tests (`src/vibekit/schema.test.ts`, `src/vibekit/kit-applicator.test.ts`)

---

### Phase 2: Templates as UIKit Components

Templates leverage the **existing UIKit component system**. A template = reusable UIKit component (`reusable: true` FrameNode) with all styling bound to `$variables`.

**Existing infrastructure used as-is:**
- `extractComponentsFromDocument()` → discovers reusable components
- `deepCloneNode()` → clones with `$variable` refs preserved
- `collectVariableRefs()` → tracks variable dependencies
- `findReusableNode()` → retrieves by ID
- `PenPage[]` multi-page system → each carousel slide is a page
- `page-tabs.tsx` → page reordering, add, remove
- `addPage()` from `document-store-pages.ts` → adds new page

#### 2.1 Template Metadata

**File: `src/types/uikit.ts`** — Extend `KitComponent`:

```typescript
interface KitComponent {
  // ... existing fields (id, name, category, tags, width, height)
  contentType?: 'slide' | 'post' | 'video-frame'
  supportedFormats?: string[]
}

interface FormatPreset {
  id: string
  name: string
  platform: 'linkedin'
  width: number
  height: number
  contentType: 'carousel' | 'video' | 'post'
}
```

#### 2.2 Build 5 Essential Templates

**New file: `src/vibekit/content-templates.ts`**

Code-defined PenNode trees (versionable, variable-bound by construction):

1. **Title/Intro** — Large heading, subtitle, author name, background fill
2. **Content** — Heading + body text, optional image, stacked layout
3. **Quote** — Large pull quote, attribution, decorative stroke
4. **Stat/Metric** — Big number, label, trend indicator, supporting text
5. **CTA/Closing** — Call-to-action text, button, author bio, social handles

Each template: root frame `width: 'fill_container'`, `height: 'fill_container'`, `layout: 'vertical'`, all visual props using `$variable` refs.

#### 2.3 Template Instantiation

**Reuses** existing UIKit patterns entirely:

```typescript
// ~20 LOC — no new engine
function instantiateTemplate(templateId: string) {
  const node = findReusableNode(templateKit.document, templateId)
  const cloned = deepCloneNode(node)  // preserves all $variable refs
  addPage({ id: nanoid(), name: node.name, children: [cloned] })
  // Canvas sync picks up new page automatically
}
```

#### 2.4 Template Picker UI

**New file: `src/components/panels/template-picker-panel.tsx`**

**Reuses:** `component-browser-panel.tsx` pattern — resizable floating panel, category tabs, search, grid cards with click-to-insert. ~80 LOC wrapper that points at the template kit instead of UIKit.

**Research insight:** `component-browser-panel.tsx` already has: resize handles, localStorage position persistence, kit selector dropdown, category pills, search debouncing, import/export buttons. Template picker is this exact component with different data source.

#### Phase 2 Deliverables

- [x] `FormatPreset` type + `contentType` on `KitComponent` (`src/types/uikit.ts`)
- [x] 5 templates as PenNode trees with `$variable` refs (`src/vibekit/content-templates.ts`)
- [ ] Template picker (wraps `component-browser-panel` pattern) (`src/components/panels/template-picker-panel.tsx`)
- [x] Format presets for LinkedIn (`src/vibekit/format-presets.ts`)
- [x] Template instantiation (`src/vibekit/template-instantiation.ts`)
- [x] Tests (`src/vibekit/content-templates.test.ts`)

---

### Phase 3: Format Switching

Existing auto-layout engine does the heavy lifting. New code is just the format switch action and a toolbar dropdown.

**Existing infrastructure used as-is:**
- `computeLayoutPositions()` → resolves `fill_container` children
- `getNodeWidth()`/`getNodeHeight()` → sizing modes
- `forcePageResync()` → full page-aware re-sync
- `use-canvas-sync.ts` → re-renders on document changes

#### 3.1 Format-Aware Canvas

**File: `src/types/animation.ts`** — Remove hardcoded `CANVAS_WIDTH`/`CANVAS_HEIGHT`, derive from active format

**File: `src/stores/canvas-store.ts`** — Add `activeFormat: FormatPreset` state, `setActiveFormat(preset)` action

#### 3.2 Format Switch Action

**New file: `src/vibekit/format-switch.ts`**

```typescript
// ~60 LOC
function switchFormat(newFormat: FormatPreset) {
  const { startBatch, endBatch } = useHistoryStore.getState()
  const { activeFormat } = useCanvasStore.getState()
  const oldW = activeFormat.width, oldH = activeFormat.height
  const newW = newFormat.width, newH = newFormat.height
  const scaleX = newW / oldW, scaleY = newH / oldH

  startBatch()
  // 1. Update root frame dimensions
  updateNode(rootFrameId, { width: newW, height: newH })
  // 2. Scale fixed-size children proportionally
  walkAndScale(rootNode, scaleX, scaleY)  // only fixed numbers, skip fill_container
  // 3. fill_container children handled automatically by computeLayoutPositions()
  endBatch()

  // 4. Scale animation keyframes
  reflowKeyframes(oldW, oldH, newW, newH)
  // 5. Trigger re-render
  forcePageResync()
}
```

#### 3.3 Format Switcher UI

**New file: `src/components/editor/format-switcher.tsx`**

Toolbar dropdown using existing shadcn/ui `Select` component.

#### Phase 3 Deliverables

- [ ] Remove hardcoded canvas dimensions (`src/types/animation.ts`)
- [x] `activeFormat` + `setActiveFormat` on canvas store (`src/stores/canvas-store.ts`)
- [x] `switchFormat()` with proportional scaling (`src/vibekit/format-switch.ts`)
- [ ] `reflowKeyframes()` on timeline store (`src/stores/timeline-store.ts`)
- [ ] Format switcher UI (`src/components/editor/format-switcher.tsx`)
- [x] Tests (`src/vibekit/format-switch.test.ts`)

---

### Phase 4: Token Extraction

Server-side token extraction following existing Nitro API patterns.

**Existing infrastructure reused:**
- `server/api/ai/chat.ts` SSE streaming pattern → progress updates during extraction
- `server/api/ai/icon.ts` offline-first + async fallback pattern → bundled token presets + live extraction
- `src/components/shared/figma-import-dialog.tsx` → URL input → processing → preview modal pattern
- Native `fetch()` (Bun runtime) → no Playwright needed for V1

#### 4.1 Extraction API Endpoint

**New file: `server/api/vibekit/extract.ts`**

**Research insight:** Playwright is NOT in `package.json`. For V1, use native `fetch()` + HTML parsing for CSS extraction. Most brand sites serve static CSS that's parseable without JavaScript execution. Upgrade to Playwright later if needed for SPAs.

```typescript
// Follows server/api/ai/ H3 handler pattern
export default defineEventHandler(async (event) => {
  const { url } = await readBody(event)
  // 1. Fetch HTML + linked CSS
  const html = await fetch(url).then(r => r.text())
  // 2. Parse CSS custom properties, computed styles from elements
  const tokens = extractTokensFromHTML(html)
  // 3. Map to VIBE_KIT_SCHEMA
  const kit = mapTokensToVibeKit(tokens, url)
  return kit
})
```

#### 4.2 Style Extraction Logic

**New file: `server/utils/token-extractor.ts`**

Extraction heuristics (~150 LOC):
- **Colors:** Parse CSS custom properties (`--primary`, `--accent`, etc.), extract inline `color`/`background-color` from key selectors (h1-h6, body, button, a)
- **Typography:** Extract `font-family` from `@font-face` and computed styles
- **Spacing:** Extract padding/margin scales from common containers
- **Borders:** Extract border-radius and border-width patterns

**Research insight:** Follow the `icon-resolver.ts` pattern — bundled presets (Tailwind, Material, etc.) as offline fallback, async extraction for custom sites. Cache results in memory `Map<url, VibeKit>`.

#### 4.3 Extraction Preview UI

**New file: `src/components/shared/extraction-preview.tsx`**

**Reuses:** `figma-import-dialog.tsx` modal pattern — URL input, processing spinner, preview, apply/cancel. ~150 LOC.

Shows: color palette grid, typography samples, spacing scale. Uses existing `ColorPicker` and `NumberInput` for inline tweaking.

#### Phase 4 Deliverables

- [ ] Nitro endpoint (`server/api/vibekit/extract.ts`)
- [ ] Token extraction heuristics (`server/utils/token-extractor.ts`)
- [ ] Extraction preview modal (follows `figma-import-dialog` pattern) (`src/components/shared/extraction-preview.tsx`)
- [ ] Bundled token presets as offline fallback (`server/utils/token-presets.ts`)
- [ ] Tests (`server/utils/token-extractor.test.ts`)

---

### Phase 5: AI Asset Generation

On-demand texture, LUT, and SFX generation via Replicate/Fal.

**Existing infrastructure reused:**
- `src/services/ai/ai-service.ts` → async API call pattern with timeout management
- `src/services/ai/icon-resolver.ts` → cache-first resolution pattern
- `ai-chat-panel.tsx` → streaming progress UI with `FixedChecklist`

#### 5.1 Asset Generation API

**New file: `server/api/vibekit/generate-asset.ts`**

Nitro endpoint following `server/api/ai/` patterns:
1. Receives asset type + style description
2. Calls Replicate/Fal API
3. Returns generated asset URL
4. Caches locally

**Research insight:** Follow `ai-service.ts` timeout pattern — `hardTimeoutMs` for generation, activity reset on progress. Use `AbortSignal` for cancellation.

#### 5.2 Asset Storage

**Reuses:** `kit-import-export.ts` pattern for embedding assets in `.pen` files. Small assets (textures < 500KB) embed as base64 in document. Large assets (SFX) via local file path.

#### Phase 5 Deliverables

- [ ] Asset generation endpoint (`server/api/vibekit/generate-asset.ts`)
- [ ] Replicate/Fal client wrapper (`server/utils/replicate-client.ts`)
- [ ] Asset caching (follows `icon-resolver.ts` Map pattern) (`src/vibekit/asset-cache.ts`)
- [ ] Asset picker UI using existing streaming progress pattern
- [ ] Error handling with retry (follows `ai-service.ts` timeout pattern)

---

### Phase 6: Onboarding Flow

First-run experience: no default kit, user picks on first run.

**Existing infrastructure reused:**
- `figma-import-dialog.tsx` → modal with URL input + processing flow
- `agent-settings-dialog.tsx` → settings modal pattern with tabs
- `vibekit-store` → `activeKitId === null` detection

#### 6.1 Onboarding Modal

**New file: `src/components/shared/onboarding-modal.tsx`**

Two-path modal (~150 LOC):
1. **"Extract from your website"** → URL input → calls extraction endpoint → preview → apply
2. **"Pick a style"** → grid of starter kits → preview → apply

Triggered when `vibekit-store.activeKitId === null` on editor mount.

#### 6.2 Starter Kits

**New file: `src/vibekit/starter-kits.ts`**

5-10 built-in kits as data (Record<string, VibeKit>). Each kit is a Record of `VariableDefinition` entries matching the schema. Pure data, no logic. Styles: corporate, creative, minimal, bold, tech, editorial, warm, cool.

#### Phase 6 Deliverables

- [ ] Onboarding modal (`src/components/shared/onboarding-modal.tsx`)
- [ ] 5-10 starter kits as data (`src/vibekit/starter-kits.ts`)
- [ ] First-run detection in editor layout (`src/components/editor/editor-layout.tsx`)

---

### Phase 7: Export Pipeline

LinkedIn-ready export for all content types.

**Existing infrastructure reused:**
- `src/utils/export.ts` → `exportToPNG()`, `exportToRaster()`, `exportLayerToRaster()` with scale options
- `src/components/panels/export-section.tsx` → per-layer export UI with format/scale toggles
- `src/animation/playback-loop.ts` → frame interpolation at any time via `getInterpolatedProperties()`
- `src/animation/canvas-bridge.ts` → `applyAnimatedProperties()` direct Fabric mutation
- `src/components/shared/export-dialog.tsx` → modal with format options

#### 7.1 Multi-Page Export (Carousel)

LinkedIn carousels are uploaded as **multi-page PDFs**. This is the primary export format.

**File: `src/utils/export.ts`** — Extend with:

```typescript
// PDF carousel export (~60 LOC) using jspdf
async function exportCarouselPDF(pages: PenPage[], scale: number) {
  const pdf = new jsPDF({ orientation: 'portrait', unit: 'px', format: [width, height] })
  for (let i = 0; i < pages.length; i++) {
    if (i > 0) pdf.addPage([width, height])
    const imageData = await renderPageToDataURL(pages[i], 'png', scale)
    pdf.addImage(imageData, 'PNG', 0, 0, width, height)
  }
  pdf.save('carousel.pdf')
}

// Also support sequential images for other platforms
async function exportCarouselImages(pages: PenPage[], format: RasterFormat, scale: number) {
  const images: Blob[] = []
  for (const page of pages) {
    const blob = await renderPageToBlob(page, format, scale)
    images.push(blob)
  }
  return zipBlobs(images, 'carousel')
}
```

**Dependency:** Add `jspdf` to package.json (MIT license, ~300KB).

#### 7.2 Video Export

**New file: `src/utils/video-export.ts`**

**Research insight:** The existing video/animation extension plan (`docs/plans/2026-03-10-openpencil-video-animation-extension-plan.md`) already specifies raw RGBA piping to FFmpeg — NOT MediaRecorder API. This is 50-70% faster. The animation engine already has all frame-stepping machinery.

```typescript
// ~200 LOC total
async function exportVideo(canvas, duration, fps, outputPath) {
  const frameCount = Math.ceil(duration * fps / 1000)
  const ffmpeg = spawn(ffmpegPath, [
    '-f', 'rawvideo', '-pix_fmt', 'rgba',
    '-s', `${width}x${height}`, '-framerate', String(fps),
    '-i', 'pipe:0', '-c:v', 'libx264', '-preset', 'fast',
    outputPath
  ])

  for (let i = 0; i < frameCount; i++) {
    const time = (i / fps) * 1000
    // Reuse existing animation engine
    const props = getInterpolatedProperties(tracks, time)
    applyAnimatedProperties(canvas, props)  // existing canvas-bridge.ts
    canvas.renderAll()
    const imageData = ctx.getImageData(0, 0, width, height)
    ffmpeg.stdin.write(Buffer.from(imageData.data.buffer))
  }
  ffmpeg.stdin.end()
}
```

**Performance:** ~12-24ms per frame → 30s video at 30fps = ~11-22s export time.

**Dependency:** Add `ffmpeg-static` to package.json (provides prebuilt FFmpeg binary, MIT license).

#### 7.3 Format-Specific Export

**Reuses** existing `export-dialog.tsx` — extend with format-aware presets:
- Carousel: **multi-page PDF** (LinkedIn native) + sequential PNG/JPG fallback, 1080x1350
- Video: MP4 via FFmpeg at 1080x1920
- Post: single PNG at 1200x1200

#### Phase 7 Deliverables

- [ ] Multi-page PDF carousel export using `jspdf` (`src/utils/export.ts` — extend)
- [ ] Sequential image carousel export with zip (`src/utils/export.ts` — extend)
- [ ] Video export via FFmpeg piping (`src/utils/video-export.ts`)
- [ ] Add `ffmpeg-static` and `jspdf` dependencies (`package.json`)
- [ ] Export dialog with format-aware presets: PDF for LinkedIn carousel (`src/components/shared/export-dialog.tsx` — extend)
- [ ] Tests for PDF + image carousel export (`src/utils/export.test.ts`)

---

### Phase 8: Vibe Kit Editor UI

**The Vibe Kit editor IS the existing variables panel** with category grouping.

**Existing infrastructure reused:**
- `variables-panel.tsx` → resizable floating panel, theme tabs, variant columns, preset load/save, inline editing
- `variable-row.tsx` → ColorCell, NumberInput, TextInput per variable type
- `variable-picker.tsx` → variable binding UI with popover
- `color-picker.tsx` → color editing
- `toolbar.tsx` → toggle button pattern (like existing `variablesPanelOpen`)

#### 8.1 Kit Management Panel

**New file: `src/components/panels/vibekit-panel.tsx`**

**~100 LOC wrapper** around existing variables panel that:
1. Groups variables by `VibeCategory` (from schema) instead of flat list
2. Adds "Switch Kit" button → opens kit gallery
3. Adds "Extract from URL" button → opens extraction modal
4. Shows kit name/source in header

All actual editing (color pickers, number inputs, text inputs, theme variants) is handled by existing `variable-row.tsx`.

#### 8.2 Toolbar Integration

**File: `src/components/editor/toolbar.tsx`**

Add Vibe Kit toggle button following existing `variablesPanelOpen` pattern.

**Research insight:** `variables-panel.tsx` already has `handleLoadPreset()` which does exactly what kit switching needs — batch `setThemes()` + loop `setVariable()`. The existing "Import Preset" flow IS kit switching.

#### Phase 8 Deliverables

- [ ] Vibe Kit panel wrapper with category grouping (`src/components/panels/vibekit-panel.tsx`)
- [ ] Toolbar toggle (`src/components/editor/toolbar.tsx`)

---

## System-Wide Impact

### Interaction Graph

```
User selects "Switch Kit" → vibekit-store.setActiveKit()
  → kit-applicator.applyKit() calls startBatch()
    → setThemes() + N × setVariable() (existing document-store CRUD)
      → endBatch() — single history entry
        → use-canvas-sync detects variable changes
          → resolveNodeForCanvas() re-resolves all $refs
            → Fabric.js canvas re-renders

User selects "Switch Format" → format-switch.switchFormat()
  → startBatch() + updateNode(root, {width, height}) + walkAndScale()
    → endBatch() — single history entry
      → forcePageResync() → computeLayoutPositions() → re-render
```

### Error Propagation

- **Kit switch with missing variables:** `applyKit()` validates against `VIBE_KIT_SCHEMA`, fills missing with fallbacks. Existing `resolveVariableRef()` returns `undefined` for missing refs → `resolveNodeForCanvas()` keeps the raw value.
- **Extraction failure:** Endpoint returns partial results; modal shows what was found with option to complete manually using existing variable editors.
- **Asset generation failure:** Cache miss + API error → show placeholder with retry button.
- **Reflow overflow:** `computeLayoutPositions()` handles overflow naturally — `fill_container` children shrink, `fit_content` wraps.

### State Lifecycle Risks

- **Kit swap = single undo entry** via `startBatch()`/`endBatch()`. Already proven in existing batch operations.
- **Format switch = single undo entry** via same mechanism.
- **Extraction is atomic** — builds complete `VibeKit` object server-side, applies in one `applyKit()` call.

### API Surface Parity

- **MCP tools:** Extend existing `src/mcp/tools/variables.ts` with `handleApplyVibeKit()` — follows exact same `handleSetVariables()` pattern
- **Code generation:** Already handles `$variable` → `var(--name)` for all types — no changes needed
- **Electron menu:** Add "Switch Kit" / "Switch Format" items following existing menu IPC pattern in `electron/main.ts`

### Integration Test Scenarios

1. Extract kit from URL → apply → verify all template `$refs` resolve
2. Switch kit → undo → verify previous variables restore → redo → verify new kit
3. Build 5-slide carousel → switch format → verify content reflows without clipping
4. Generate texture → apply to background → export PNG → verify texture in output
5. Video export → verify frame count matches duration × fps

## Acceptance Criteria

### Functional Requirements

- [ ] User can create/apply/switch Vibe Kits with complete restyling on swap
- [ ] User can extract tokens from public websites and generate a Vibe Kit
- [ ] User can create LinkedIn carousel, video, and post from 5 templates
- [ ] Content auto-reflows between LinkedIn formats without loss
- [ ] User can generate textures, LUTs, SFX via AI on demand
- [ ] First-run onboarding guides user to pick/extract first kit
- [ ] Export: carousel as multi-page PDF (LinkedIn native) or sequential images, video as MP4, post as single image
- [ ] Escape hatch: break out to freeform PenNode editing at any time
- [ ] Undo/redo works across kit switches and format changes

### Non-Functional Requirements

- [ ] Kit switch re-renders in < 500ms (< 100 nodes)
- [ ] Format reflow in < 1s (< 50 nodes)
- [ ] Token extraction in < 15s (standard sites)
- [ ] Video export: 30fps × 30s in < 22s (per existing plan benchmarks)

### Quality Gates

- [ ] Unit tests for schema validation, kit application, format switch
- [ ] Integration tests for extraction → apply → reflow → export
- [ ] Type coverage: strict, no `any`
- [ ] UI components use shadcn/ui design tokens

## Dependencies & Risks

### Dependencies

- **`ffmpeg-static`** — For video export. Prebuilt binaries, MIT license.
- **`jspdf`** — For multi-page PDF carousel export. MIT license, ~300KB.
- **Replicate/Fal SDK** — For AI asset generation. Need API keys.
- **Phase 1 Animation Engine** — Must be merged (currently on `feat/animation-core` branch).

### Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| Token extraction quality varies across sites | High | Medium | Preview + manual tweak, bundled presets as fallback |
| Reflow edge cases for complex layouts | Medium | Medium | Templates use `fill_container` sizing, limiting edge cases |
| FFmpeg binary size in Electron bundle | Low | Medium | `ffmpeg-static` is ~70MB; consider lazy download |
| Vibe Kit schema needs iteration | Medium | Low | Start with 30 core variables, extend. Schema versioning. |

## Success Metrics

- **Time to first content:** < 5 minutes from first launch to exported carousel
- **Kit swap fidelity:** 100% of template props change on swap (no orphaned styling)
- **Reflow coverage:** All 5 templates reflow cleanly across 3 LinkedIn formats
- **Extraction success rate:** > 80% of public websites produce a usable kit

## Future Considerations

- **Multi-platform formats:** Instagram, X, YouTube, newsletter (V2)
- **Custom templates:** User designs saved as reusable templates
- **Kit marketplace:** Share/sell Vibe Kits
- **AI content generation:** Generate carousel text from a topic/brief
- **MCP tools for Jeans:** Full agent-driven content creation

## Sources & References

### Origin

- **Brainstorm:** [docs/brainstorms/2026-03-10-jeans-content-workflow-brainstorm.md](docs/brainstorms/2026-03-10-jeans-content-workflow-brainstorm.md) — PenNode extension, full-spectrum kit from V1, LinkedIn-only V1, AI-generated assets, block-first with escape hatch

### Internal References (Reuse Targets)

- Variables system: `src/types/variables.ts`, `src/variables/resolve-variables.ts`, `src/variables/replace-refs.ts`
- Variables panel: `src/components/panels/variables-panel.tsx`, `src/components/panels/variable-row.tsx`
- Component browser: `src/components/panels/component-browser-panel.tsx`
- UIKit system: `src/types/uikit.ts`, `src/uikit/kit-import-export.ts`, `src/uikit/kit-utils.ts`
- Theme presets: `src/stores/theme-preset-store.ts` (pattern for kit persistence)
- Layout engine: `src/canvas/canvas-layout-engine.ts`
- Canvas sync: `src/canvas/use-canvas-sync.ts`, `src/canvas/canvas-sync-utils.ts`
- Animation engine: `src/animation/playback-loop.ts`, `src/animation/canvas-bridge.ts`, `src/animation/interpolation.ts`
- Export: `src/utils/export.ts`, `src/components/shared/export-dialog.tsx`
- Figma import dialog: `src/components/shared/figma-import-dialog.tsx` (pattern for extraction preview)
- Document store: `src/stores/document-store.ts`, `src/stores/history-store.ts`
- Server API: `server/api/ai/chat.ts` (SSE pattern), `server/api/ai/icon.ts` (cache-first pattern)
- Video plan: `docs/plans/2026-03-10-openpencil-video-animation-extension-plan.md`

### Related Work

- Animation Phase 1: branch `feat/animation-core`, commit `dcbee8d`
