# Image Search & Generation for Design Placeholders

**Date:** 2026-03-20
**Status:** Draft

## Problem

When OpenPencil generates designs via AI, image nodes use colored placeholder rectangles or SVG phone mockups. Real images would significantly improve design fidelity. Users currently have no way to automatically populate images during generation or manually search/generate images afterward.

## Goals

1. Auto-fill image nodes with relevant photos after AI design generation (zero-config)
2. Allow manual image search and AI image generation from the Property Panel
3. Support multiple image generation providers with flexible configuration
4. Require zero setup for image search; optional setup only for AI image generation

## Non-Goals

- Image editing/cropping beyond existing adjustments (exposure, contrast, etc.)
- Building a full stock photo browser (this is a design tool, not a photo library)
- Bundling API keys or requiring Openverse OAuth registration by default

---

## Architecture Overview

```
AI Design Generation completes
        │
        ▼
scanAndFillImages()              ← NEW: auto-search pipeline
  │ For each image node:
  │   query = imagePrompt ?? name
  │   POST /api/ai/image-search
  │     → Openverse API (primary)
  │     → Wikimedia Commons API (fallback on 429)
  │   Update node.src with result URL
  │
  ▼
User selects image node
        │
        ▼
Property Panel — Image Section
  ├── [Search] → Search Popover (edit query, pick from results)
  ├── [Generate] → Generate Popover (edit prompt, call image gen API)
  └── [Upload] → Existing file upload (unchanged)
```

---

## 1. Data Model Changes

### 1.1 ImageNode Extension (`src/types/pen.ts`)

```typescript
export interface ImageNode extends PenNodeBase {
  type: 'image'
  src: string
  imagePrompt?: string   // Semantic description for search/generation. Persisted.
  // ... existing fields unchanged
}
```

- `imagePrompt`: Persisted to `.pen` files. AI fills this during design generation (e.g. `"modern office with natural lighting"`). Used as search query or generation prompt.

**Note:** `imageSearchStatus` is NOT stored on the node (would leak into undo/redo history snapshots). Instead, it is kept in `canvas-store.ts` as a `Map<nodeId, status>`, following the same pattern as selection/hover/viewport state.

### 1.2 Image Service Types (`src/types/image-service.ts`) — NEW FILE

Separate from `agent-settings.ts` to follow single-responsibility (like `variables.ts` is separate from `pen.ts`):

```typescript
export type ImageGenProvider = 'openai' | 'gemini' | 'replicate' | 'custom'

export interface ImageGenConfig {
  provider: ImageGenProvider
  apiKey: string
  model: string        // Free text input, placeholder suggests default per provider
  baseUrl?: string     // Optional, for proxies or custom endpoints
}

export interface ImageSearchResult {
  id: string
  url: string            // Full-size image URL
  thumbUrl: string       // Thumbnail URL for preview (CORS-friendly proxy)
  width: number
  height: number
  source: 'openverse' | 'wikimedia'
  license: string        // e.g. "CC BY 2.0"
  attribution?: string
}
```

---

## 2. Server API Endpoints

All new endpoints in `server/api/ai/`.

### 2.1 `POST /api/ai/image-search`

Dual-source image search with automatic fallback.

**Request:**
```typescript
{
  query: string
  count?: number                              // Default 5
  aspectRatio?: 'wide' | 'tall' | 'square'   // Optional, inferred from node dimensions
  openverseClientId?: string                  // Optional OAuth client_id
  openverseClientSecret?: string              // Optional OAuth client_secret
}
```

**Response:**
```typescript
{
  results: ImageSearchResult[]
  source: 'openverse' | 'wikimedia'
}
```

**Internal logic:**
1. If OAuth credentials provided → request Openverse OAuth token (cached), use authenticated request (10,000/day)
2. Else → use anonymous Openverse request (200/day per IP)
3. Call Openverse `GET /v1/images/?q={query}&page_size={count}&aspect_ratio={aspectRatio}`
4. On success → map results and return
5. On 429 (rate limited) → fall back to Wikimedia Commons API
6. On both failure → return empty `results`

**Openverse response mapping:**
```typescript
{
  id: result.id,
  url: result.url,                    // Direct image URL (e.g. Flickr)
  thumbUrl: result.thumbnail,         // Openverse proxy thumbnail
  width: result.width,
  height: result.height,
  source: 'openverse',
  license: `${result.license} ${result.license_version}`,
  attribution: result.attribution,
}
```

**Wikimedia fallback query:**
```
GET https://commons.wikimedia.org/w/api.php
  ?action=query
  &generator=search
  &gsrsearch={query}
  &gsrnamespace=6
  &gsrlimit={count}
  &prop=imageinfo
  &iiprop=url|size|mime|extmetadata
  &iiurlwidth=800
  &format=json
```

### 2.2 `POST /api/ai/image-generate`

Multi-provider image generation.

**Request:**
```typescript
{
  prompt: string
  provider: ImageGenProvider
  model: string
  apiKey: string
  baseUrl?: string
  width?: number      // Reference size (provider may approximate)
  height?: number
}
```

**Response:**
```typescript
{
  url: string          // Generated image URL or data URL
}
```

**Provider implementations:**

| Provider | API Call |
|----------|---------|
| `openai` | `POST {baseUrl ?? 'https://api.openai.com'}/v1/images/generations` with `{ model, prompt, size }` |
| `gemini` | Gemini SDK `generateContent()` with `responseModalities: ['IMAGE']` |
| `replicate` | `POST {baseUrl ?? 'https://api.replicate.com'}/v1/predictions` with `{ model, input: { prompt } }`, poll for result |
| `custom` | OpenAI-compatible format: `POST {baseUrl}/v1/images/generations` |

### 2.3 `POST /api/ai/image-service-test`

Validates API key and connectivity.

**Request:**
```typescript
{
  service: 'openverse' | ImageGenProvider
  apiKey: string
  model?: string
  baseUrl?: string
  clientId?: string       // For Openverse OAuth
  clientSecret?: string
}
```

**Response:**
```typescript
{ valid: boolean, error?: string }
```

---

## 3. Settings System

### 3.1 Store Changes (`src/stores/agent-settings-store.ts`)

Add `imageGenConfig` and `openverseOAuth` to state:

```typescript
interface AgentSettingsState {
  // ... existing fields
  imageGenConfig: ImageGenConfig
  openverseOAuth: { clientId: string; clientSecret: string } | null
}

const DEFAULT_IMAGE_GEN_CONFIG: ImageGenConfig = {
  provider: 'openai',
  apiKey: '',
  model: '',
  baseUrl: undefined,
}
```

Persisted via existing `persist()`/`hydrate()` flow (localStorage / Electron preferences).

### 3.2 Settings UI — Images Tab (`src/components/shared/agent-settings-dialog.tsx`)

New fourth tab "Images" in Agent Settings Dialog:

**Image Search section:**
- Status indicator: "Ready (Openverse + Wikimedia)" (always shown)
- Collapsible "Advanced" section for optional Openverse OAuth:
  - `clientId` and `clientSecret` text inputs
  - Link to Openverse registration page
  - Status showing authenticated rate limit

**Image Generation section:**
- Provider dropdown: OpenAI / Google Gemini / Replicate / Custom
- API Key: password input with mask + Test button
- Model: free text input with provider-specific placeholder:
  - OpenAI → `dall-e-3`
  - Gemini → `gemini-2.0-flash-preview-image-generation`
  - Replicate → `black-forest-labs/flux-1.1-pro`
  - Custom → `model-name`
- Collapsible "Advanced" for optional `baseUrl`

### 3.3 Model Placeholder Defaults

```typescript
const MODEL_PLACEHOLDERS: Record<ImageGenProvider, string> = {
  openai: 'dall-e-3',
  gemini: 'gemini-2.0-flash-preview-image-generation',
  replicate: 'black-forest-labs/flux-1.1-pro',
  custom: 'model-name',
}
```

No hardcoded model lists. User types freely; placeholder serves as suggestion.

---

## 4. Property Panel Integration

### 4.1 Image Section Extension (`src/components/panels/image-section.tsx`)

Add three action buttons below image preview: **Search**, **Generate**, **Upload**.

**Search button** → opens `ImageSearchPopover`:
- Text input pre-filled with `imagePrompt ?? name`
- Search button triggers `/api/ai/image-search`
- Results displayed as thumbnail grid (5 images)
- Click thumbnail → update `node.src` with selected `url`
- "Load more" button for pagination
- Footer shows license info and source

**Generate button** → opens `ImageGeneratePopover`:
- Text input pre-filled with `imagePrompt ?? name`
- If image gen not configured → show warning + "Open Settings" button that opens Agent Settings Dialog on Images tab
- If configured → show "Generate" button + model info
- Loading state during generation (~5-10s)
- Preview generated image → click to apply to `node.src`

**Upload button** → existing file upload behavior (unchanged).

### 4.2 New Components

- `src/components/panels/image-search-popover.tsx` — Search popover with query input + result grid
- `src/components/panels/image-generate-popover.tsx` — Generation popover with prompt input + preview

---

## 5. Auto-Search Pipeline

### 5.1 `scanAndFillImages()` (`src/services/ai/image-search-pipeline.ts`) — NEW FILE

Extracted to a separate file to keep `design-canvas-ops.ts` under the 800-line limit (currently 714 lines). Called from `design-canvas-ops.ts` after generation completes.

```typescript
async function scanAndFillImages(
  rootId: string,
  signal?: AbortSignal,
): Promise<void> {
  const imageNodes = collectImageNodes(rootId)
  const needsFill = imageNodes.filter(n =>
    !n.src || n.src.startsWith('data:image/svg+xml')
  )

  const { setImageSearchStatus } = useCanvasStore.getState()

  for (const node of needsFill) {
    if (signal?.aborted) return  // Cancel if new generation started

    const query = node.imagePrompt ?? node.name ?? 'placeholder'
    const aspect = inferAspectRatio(node)

    try {
      const { results } = await fetchImageSearch(query, 1, aspect)
      if (results.length > 0) {
        updateNode(node.id, { src: results[0].thumbUrl })  // Use proxy thumbnail (CORS-safe)
        setImageSearchStatus(node.id, 'found')
      } else {
        setImageSearchStatus(node.id, 'failed')
      }
    } catch {
      setImageSearchStatus(node.id, 'failed')
    }

    await sleep(200) // Respect Openverse 20/min burst limit
  }

  forcePageResync()
}
```

**Key design decisions:**
- **AbortController**: A new generation cancels any in-flight `scanAndFillImages` from the previous generation, preventing concurrent document mutations.
- **`thumbUrl` over `url`**: Auto-fill uses the Openverse proxy thumbnail (CORS-friendly) rather than the original source URL (Flickr etc. may block cross-origin canvas access). Users can switch to full-size via Search popover.
- **`imageSearchStatus` in canvas-store**: Runtime status stored in `canvas-store.ts` as `imageSearchStatuses: Map<string, 'pending' | 'found' | 'failed'>`, not on the node (avoids undo/redo pollution).

### 5.2 Aspect Ratio Inference

```typescript
function inferAspectRatio(node: ImageNode): 'wide' | 'tall' | 'square' | undefined {
  const w = typeof node.width === 'number' ? node.width : 0
  const h = typeof node.height === 'number' ? node.height : 0
  if (!w || !h) return undefined
  const ratio = w / h
  if (ratio > 1.3) return 'wide'
  if (ratio < 0.77) return 'tall'
  return 'square'
}
```

### 5.3 Integration Point

In `design-canvas-ops.ts`, after the existing generation completion logic:

```typescript
// After all nodes are inserted and heuristics applied:
await scanAndFillImages(rootFrameId)
```

---

## 6. AI Prompt Changes

### 6.1 Design Generation Prompts (`src/services/ai/ai-prompts.ts`)

Update image node schema guidance to include `imagePrompt`:

```
- image: Raster image. Props: src (URL string), width, height, cornerRadius, effects,
  imagePrompt (recommended: descriptive English phrase for image content,
  e.g. "modern office workspace with natural lighting", "smiling woman portrait headshot").
  Omit imagePrompt for purely decorative images (backgrounds, patterns).
  Do NOT include src — images are auto-populated after generation.
```

### 6.2 Orchestrator Prompts (`src/services/ai/orchestrator-prompts.ts`)

Add instruction for sub-agents:
```
For image nodes: always include imagePrompt with a descriptive English phrase.
Do NOT include a src URL — images will be auto-populated after generation.
```

---

## 7. MCP `G()` Operation

### 7.1 Batch Design Extension (`src/mcp/tools/batch-design.ts`)

Implement the `G()` (Generate image) operation in the DSL parser:

```
img1=G("parentId", "search", "modern office workspace")
img2=G("parentId", "generate", "futuristic city skyline at dusk")
```

**Syntax:** `varName=G(parentId, mode, prompt)`
- `mode`: `"search"` or `"generate"`
- `prompt`: descriptive text

**Parser changes required:**
1. Update `assignMatch` regex (line ~134) from `[ICRM]` to `[ICRMG]` to recognize `G` as a valid operation letter
2. Add `case 'G'` branch in the `executeLine()` switch statement
3. `G()` is **async** (calls server API), unlike existing sync operations. The `handleBatchDesign` loop must `await` the `executeLine` result for `G` operations.

**Implementation:**
- `"search"` → call `/api/ai/image-search` with prompt, create image node with `src` from first result
- `"generate"` → call `/api/ai/image-generate` with prompt (requires image gen config), create image node with generated `src`
- Both modes set `imagePrompt` on the created node

### 7.2 MCP Layered Design Integration

The layered pipeline (`design-skeleton` → `design-content` → `design-refine`) also generates image nodes. `scanAndFillImages()` should run at the end of `design_refine` to auto-fill images in the completed design, same as the in-app generation path.

---

## 8. File Changes Summary

| File | Change |
|------|--------|
| `src/types/pen.ts` | Add `imagePrompt` to `ImageNode` |
| `src/types/image-service.ts` | **New**: `ImageGenProvider`, `ImageGenConfig`, `ImageSearchResult` types |
| `src/stores/agent-settings-store.ts` | Add `imageGenConfig`, `openverseOAuth` state + actions |
| `src/stores/canvas-store.ts` | Add `imageSearchStatuses` map + `setImageSearchStatus` action |
| `src/components/shared/agent-settings-dialog.tsx` | Add Images tab (4th tab), update `SettingsTab` type to include `'images'` |
| `src/components/panels/image-section.tsx` | Add Search/Generate/Upload buttons |
| `src/components/panels/image-search-popover.tsx` | **New**: Search popover component |
| `src/components/panels/image-generate-popover.tsx` | **New**: Generate popover component |
| `server/api/ai/image-search.ts` | **New**: Dual-source image search endpoint |
| `server/api/ai/image-generate.ts` | **New**: Multi-provider image generation endpoint |
| `server/api/ai/image-service-test.ts` | **New**: API key validation endpoint |
| `src/services/ai/image-search-pipeline.ts` | **New**: `scanAndFillImages()`, `inferAspectRatio()`, `collectImageNodes()` |
| `src/services/ai/design-canvas-ops.ts` | Call `scanAndFillImages()` after generation completes |
| `src/services/ai/ai-prompts.ts` | Add `imagePrompt` to image node schema |
| `src/services/ai/orchestrator-prompts.ts` | Add `imagePrompt` instruction for sub-agents |
| `src/mcp/tools/batch-design.ts` | Implement `G()` operation, update DSL regex |
| `src/mcp/tools/design-refine.ts` | Call `scanAndFillImages()` after refinement |

---

## 9. Testing Plan

- [ ] Unit: Openverse API response mapping
- [ ] Unit: Wikimedia API response mapping and fallback
- [ ] Unit: `inferAspectRatio()` logic
- [ ] Unit: `scanAndFillImages()` with mock API responses
- [ ] Unit: Image generation request formatting per provider
- [ ] Integration: Settings persistence (imageGenConfig, openverseOAuth)
- [ ] Integration: Image search popover renders results and updates node.src
- [ ] Integration: Image generate popover shows unconfigured state + settings link
- [ ] Integration: MCP `G()` operation creates image nodes
- [ ] E2E: Generate design → auto-search fills images → manual search/generate works
- [ ] E2E: Openverse rate limit → Wikimedia fallback activates
