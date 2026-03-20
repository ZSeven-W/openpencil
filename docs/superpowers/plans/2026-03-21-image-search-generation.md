# Image Search & Generation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Auto-fill design placeholder images via Openverse/Wikimedia search + manual AI image generation from the Property Panel.

**Architecture:** Server API endpoints proxy Openverse (primary) and Wikimedia (fallback) for zero-config image search. A separate endpoint handles multi-provider image generation (OpenAI, Gemini, Replicate, Custom). After AI design generation completes, `scanAndFillImages()` auto-populates image nodes. Users manually refine via Search/Generate popovers in the image property panel.

**Tech Stack:** Nitro server (existing), Zustand v5 (existing), React 19 (existing), Openverse REST API, Wikimedia Commons API, OpenAI/Gemini/Replicate image generation APIs.

**Spec:** `docs/superpowers/specs/2026-03-20-image-search-generation-design.md`

---

## File Map

| File | Responsibility | Status |
|------|---------------|--------|
| `src/types/image-service.ts` | ImageGenProvider, ImageGenConfig, ImageSearchResult types | Create |
| `src/types/pen.ts:186` | Add `imagePrompt` to ImageNode | Modify |
| `server/api/ai/image-search.ts` | Openverse + Wikimedia dual-source search endpoint | Create |
| `server/api/ai/image-generate.ts` | Multi-provider image generation endpoint | Create |
| `server/api/ai/image-service-test.ts` | API key validation endpoint | Create |
| `src/stores/agent-settings-store.ts:14-19,83-92,136-179` | Add imageGenConfig + openverseOAuth state | Modify |
| `src/stores/canvas-store.ts:23-57,65-82` | Add imageSearchStatuses map | Modify |
| `src/components/ui/popover.tsx` | shadcn Popover primitive (prerequisite) | Create |
| `src/components/shared/agent-settings-dialog.tsx:49,630-704` | Images tab (4th tab), import ImagesPage | Modify |
| `src/components/shared/agent-settings-images-page.tsx` | Extracted Images tab content (800-line limit) | Create |
| `src/components/panels/image-search-popover.tsx` | Search popover with result grid | Create |
| `src/components/panels/image-generate-popover.tsx` | Generate popover with prompt + preview | Create |
| `src/components/panels/image-section.tsx:35-51` | Add Search/Generate/Upload buttons | Modify |
| `src/services/ai/image-search-pipeline.ts` | scanAndFillImages, inferAspectRatio, collectImageNodes | Create |
| `src/services/ai/design-canvas-ops.ts:294-315,317-346` | Call scanAndFillImages after generation | Modify |
| `src/services/ai/ai-prompts.ts:14` | Add imagePrompt to image node schema | Modify |
| `src/services/ai/orchestrator-prompts.ts:63` | Add imagePrompt instruction | Modify |
| `src/mcp/tools/batch-design.ts:134-220` | Implement G() operation | Modify |
| `src/mcp/tools/design-refine.ts:95-107` | Call scanAndFillImages after refine | Modify |

---

## Task 1: Types & Data Model

**Files:**
- Create: `src/types/image-service.ts`
- Modify: `src/types/pen.ts:186`
- Test: `src/types/__tests__/image-service.test.ts`

- [ ] **Step 1: Create image service types**

Create `src/types/image-service.ts`:

```typescript
export type ImageGenProvider = 'openai' | 'gemini' | 'replicate' | 'custom'

export interface ImageGenConfig {
  provider: ImageGenProvider
  apiKey: string
  model: string
  baseUrl?: string
}

export interface ImageSearchResult {
  id: string
  url: string
  thumbUrl: string
  width: number
  height: number
  source: 'openverse' | 'wikimedia'
  license: string
  attribution?: string
}

export interface ImageSearchResponse {
  results: ImageSearchResult[]
  source: 'openverse' | 'wikimedia'
}

export const MODEL_PLACEHOLDERS: Record<ImageGenProvider, string> = {
  openai: 'dall-e-3',
  gemini: 'gemini-2.0-flash-preview-image-generation',
  replicate: 'black-forest-labs/flux-1.1-pro',
  custom: 'model-name',
}

export const DEFAULT_IMAGE_GEN_CONFIG: ImageGenConfig = {
  provider: 'openai',
  apiKey: '',
  model: '',
  baseUrl: undefined,
}
```

- [ ] **Step 2: Add imagePrompt to ImageNode**

In `src/types/pen.ts`, add `imagePrompt` before the closing brace of `ImageNode` (line 186):

```typescript
  shadows?: number     // -100 to 100
  imagePrompt?: string // Semantic description for image search/generation
}
```

- [ ] **Step 3: Write type tests**

Create `src/types/__tests__/image-service.test.ts`:

```typescript
import { describe, it, expect } from 'vitest'
import {
  DEFAULT_IMAGE_GEN_CONFIG,
  MODEL_PLACEHOLDERS,
  type ImageGenConfig,
  type ImageSearchResult,
  type ImageGenProvider,
} from '../image-service'

describe('image-service types', () => {
  it('DEFAULT_IMAGE_GEN_CONFIG has expected shape', () => {
    expect(DEFAULT_IMAGE_GEN_CONFIG.provider).toBe('openai')
    expect(DEFAULT_IMAGE_GEN_CONFIG.apiKey).toBe('')
    expect(DEFAULT_IMAGE_GEN_CONFIG.model).toBe('')
  })

  it('MODEL_PLACEHOLDERS covers all providers', () => {
    const providers: ImageGenProvider[] = ['openai', 'gemini', 'replicate', 'custom']
    for (const p of providers) {
      expect(MODEL_PLACEHOLDERS[p]).toBeTruthy()
    }
  })

  it('ImageSearchResult shape is correct', () => {
    const result: ImageSearchResult = {
      id: 'test',
      url: 'https://example.com/img.jpg',
      thumbUrl: 'https://example.com/thumb.jpg',
      width: 800,
      height: 600,
      source: 'openverse',
      license: 'CC BY 2.0',
    }
    expect(result.source).toBe('openverse')
  })
})
```

- [ ] **Step 4: Run tests**

Run: `bun --bun vitest run src/types/__tests__/image-service.test.ts`
Expected: All 3 tests pass.

- [ ] **Step 5: Type check**

Run: `npx tsc --noEmit`
Expected: No type errors.

- [ ] **Step 6: Commit**

```bash
git add src/types/image-service.ts src/types/pen.ts src/types/__tests__/image-service.test.ts
git commit -m "feat(types): add image service types and imagePrompt to ImageNode"
```

---

## Task 2: Server — Image Search Endpoint

**Files:**
- Create: `server/api/ai/image-search.ts`
- Test: `src/services/ai/__tests__/image-search-api.test.ts`

- [ ] **Step 1: Write the image search endpoint**

Create `server/api/ai/image-search.ts`. Follow the pattern from existing endpoints in `server/api/ai/` (e.g. `icon.ts`):

```typescript
import { defineEventHandler, readBody } from 'h3'
import type { ImageSearchResult, ImageSearchResponse } from '../../../src/types/image-service'

interface ImageSearchRequest {
  query: string
  count?: number
  aspectRatio?: 'wide' | 'tall' | 'square'
  openverseClientId?: string
  openverseClientSecret?: string
}

// Openverse OAuth token cache
let cachedToken: { token: string; expiresAt: number } | null = null

async function getOpenverseToken(
  clientId: string,
  clientSecret: string,
): Promise<string | null> {
  if (cachedToken && Date.now() < cachedToken.expiresAt) {
    return cachedToken.token
  }
  try {
    const res = await fetch('https://api.openverse.org/v1/auth_tokens/token/', {
      method: 'POST',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body: `grant_type=client_credentials&client_id=${clientId}&client_secret=${clientSecret}`,
    })
    if (!res.ok) return null
    const data = await res.json()
    cachedToken = {
      token: data.access_token,
      expiresAt: Date.now() + (data.expires_in - 60) * 1000,
    }
    return cachedToken.token
  } catch {
    return null
  }
}

async function searchOpenverse(
  query: string,
  count: number,
  aspectRatio?: string,
  token?: string | null,
): Promise<ImageSearchResponse | null> {
  const params = new URLSearchParams({
    q: query,
    page_size: String(count),
  })
  if (aspectRatio) params.set('aspect_ratio', aspectRatio)

  const headers: Record<string, string> = {}
  if (token) headers['Authorization'] = `Bearer ${token}`

  try {
    const res = await fetch(
      `https://api.openverse.org/v1/images/?${params}`,
      { headers },
    )
    if (res.status === 429) return null // Rate limited → fallback
    if (!res.ok) return null

    const data = await res.json()
    const results: ImageSearchResult[] = (data.results || []).map(
      (r: any) => ({
        id: r.id,
        url: r.url,
        thumbUrl: r.thumbnail,
        width: r.width,
        height: r.height,
        source: 'openverse' as const,
        license: `${r.license} ${r.license_version}`,
        attribution: r.attribution,
      }),
    )
    return { results, source: 'openverse' }
  } catch {
    return null
  }
}

async function searchWikimedia(
  query: string,
  count: number,
): Promise<ImageSearchResponse> {
  const params = new URLSearchParams({
    action: 'query',
    generator: 'search',
    gsrsearch: query,
    gsrnamespace: '6',
    gsrlimit: String(count),
    prop: 'imageinfo',
    iiprop: 'url|size|mime|extmetadata',
    iiurlwidth: '800',
    format: 'json',
    origin: '*',
  })

  try {
    const res = await fetch(
      `https://commons.wikimedia.org/w/api.php?${params}`,
    )
    if (!res.ok) return { results: [], source: 'wikimedia' }

    const data = await res.json()
    const pages = data?.query?.pages || {}
    const results: ImageSearchResult[] = Object.values(pages)
      .map((page: any) => {
        const ii = page.imageinfo?.[0]
        if (!ii) return null
        const license =
          ii.extmetadata?.LicenseShortName?.value || 'Unknown license'
        return {
          id: String(page.pageid),
          url: ii.url,
          thumbUrl: ii.thumburl || ii.url,
          width: ii.width,
          height: ii.height,
          source: 'wikimedia' as const,
          license,
          attribution: page.title,
        }
      })
      .filter(Boolean) as ImageSearchResult[]

    return { results, source: 'wikimedia' }
  } catch {
    return { results: [], source: 'wikimedia' }
  }
}

export default defineEventHandler(async (event) => {
  const body = (await readBody(event)) as ImageSearchRequest
  const { query, count = 5, aspectRatio, openverseClientId, openverseClientSecret } = body

  if (!query) {
    return { results: [], source: 'openverse' }
  }

  // Try Openverse (with optional OAuth)
  let token: string | null = null
  if (openverseClientId && openverseClientSecret) {
    token = await getOpenverseToken(openverseClientId, openverseClientSecret)
  }

  const openverseResult = await searchOpenverse(query, count, aspectRatio, token)
  if (openverseResult) return openverseResult

  // Fallback to Wikimedia
  return searchWikimedia(query, count)
})
```

- [ ] **Step 2: Write test for response mapping**

Create `src/services/ai/__tests__/image-search-api.test.ts`:

```typescript
import { describe, it, expect } from 'vitest'
import type { ImageSearchResult } from '../../../types/image-service'

// Test the mapping logic extracted from the endpoint
function mapOpenverseResult(r: any): ImageSearchResult {
  return {
    id: r.id,
    url: r.url,
    thumbUrl: r.thumbnail,
    width: r.width,
    height: r.height,
    source: 'openverse',
    license: `${r.license} ${r.license_version}`,
    attribution: r.attribution,
  }
}

function mapWikimediaPages(pages: Record<string, any>): ImageSearchResult[] {
  return Object.values(pages)
    .map((page: any) => {
      const ii = page.imageinfo?.[0]
      if (!ii) return null
      return {
        id: String(page.pageid),
        url: ii.url,
        thumbUrl: ii.thumburl || ii.url,
        width: ii.width,
        height: ii.height,
        source: 'wikimedia' as const,
        license: ii.extmetadata?.LicenseShortName?.value || 'Unknown license',
        attribution: page.title,
      }
    })
    .filter(Boolean) as ImageSearchResult[]
}

describe('image search response mapping', () => {
  it('maps Openverse result correctly', () => {
    const raw = {
      id: 'abc-123',
      url: 'https://flickr.com/photo.jpg',
      thumbnail: 'https://api.openverse.org/v1/images/abc-123/thumb/',
      width: 1024,
      height: 768,
      license: 'by',
      license_version: '2.0',
      attribution: '"Photo" by Author is licensed under CC BY 2.0.',
    }
    const result = mapOpenverseResult(raw)
    expect(result.id).toBe('abc-123')
    expect(result.thumbUrl).toContain('openverse.org')
    expect(result.source).toBe('openverse')
    expect(result.license).toBe('by 2.0')
  })

  it('maps Wikimedia pages correctly', () => {
    const pages = {
      '12345': {
        pageid: 12345,
        title: 'File:Test.jpg',
        imageinfo: [
          {
            url: 'https://upload.wikimedia.org/test.jpg',
            thumburl: 'https://upload.wikimedia.org/thumb/test.jpg',
            width: 800,
            height: 600,
            extmetadata: { LicenseShortName: { value: 'CC BY-SA 3.0' } },
          },
        ],
      },
    }
    const results = mapWikimediaPages(pages)
    expect(results).toHaveLength(1)
    expect(results[0].source).toBe('wikimedia')
    expect(results[0].license).toBe('CC BY-SA 3.0')
    expect(results[0].thumbUrl).toContain('thumb')
  })

  it('handles pages with no imageinfo', () => {
    const pages = { '999': { pageid: 999, title: 'File:X.jpg' } }
    const results = mapWikimediaPages(pages)
    expect(results).toHaveLength(0)
  })
})
```

- [ ] **Step 3: Run tests**

Run: `bun --bun vitest run src/services/ai/__tests__/image-search-api.test.ts`
Expected: All 3 tests pass.

- [ ] **Step 4: Commit**

```bash
git add server/api/ai/image-search.ts src/services/ai/__tests__/image-search-api.test.ts
git commit -m "feat(server): add dual-source image search endpoint (Openverse + Wikimedia)"
```

---

## Task 3: Server — Image Generate Endpoint

**Files:**
- Create: `server/api/ai/image-generate.ts`

- [ ] **Step 1: Write the image generate endpoint**

Create `server/api/ai/image-generate.ts`:

```typescript
import { defineEventHandler, readBody, createError } from 'h3'
import type { ImageGenProvider } from '../../../src/types/image-service'

interface ImageGenerateRequest {
  prompt: string
  provider: ImageGenProvider
  model: string
  apiKey: string
  baseUrl?: string
  width?: number
  height?: number
}

function mapToOpenAISize(w?: number, h?: number): string {
  if (!w || !h) return '1024x1024'
  const ratio = w / h
  if (ratio > 1.3) return '1792x1024'   // landscape
  if (ratio < 0.77) return '1024x1792'  // portrait
  return '1024x1024'                     // square
}

async function generateOpenAI(
  req: ImageGenerateRequest,
): Promise<string> {
  const base = req.baseUrl || 'https://api.openai.com'
  // OpenAI only accepts specific size presets
  const size = mapToOpenAISize(req.width, req.height)

  const res = await fetch(`${base}/v1/images/generations`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${req.apiKey}`,
    },
    body: JSON.stringify({
      model: req.model || 'dall-e-3',
      prompt: req.prompt,
      n: 1,
      size,
      response_format: 'url',
    }),
  })

  if (!res.ok) {
    const err = await res.text()
    throw new Error(`OpenAI error: ${err}`)
  }

  const data = await res.json()
  return data.data?.[0]?.url || ''
}

async function generateGemini(
  req: ImageGenerateRequest,
): Promise<string> {
  const base = req.baseUrl || 'https://generativelanguage.googleapis.com'
  const model = req.model || 'gemini-2.0-flash-preview-image-generation'

  const res = await fetch(
    `${base}/v1beta/models/${model}:generateContent?key=${req.apiKey}`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        contents: [{ parts: [{ text: req.prompt }] }],
        generationConfig: { responseModalities: ['TEXT', 'IMAGE'] },
      }),
    },
  )

  if (!res.ok) {
    const err = await res.text()
    throw new Error(`Gemini error: ${err}`)
  }

  const data = await res.json()
  const parts = data.candidates?.[0]?.content?.parts || []
  for (const part of parts) {
    if (part.inlineData?.mimeType?.startsWith('image/')) {
      return `data:${part.inlineData.mimeType};base64,${part.inlineData.data}`
    }
  }
  throw new Error('Gemini returned no image')
}

async function generateReplicate(
  req: ImageGenerateRequest,
): Promise<string> {
  const base = req.baseUrl || 'https://api.replicate.com'
  const model = req.model || 'black-forest-labs/flux-1.1-pro'

  // Create prediction
  const createRes = await fetch(`${base}/v1/predictions`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${req.apiKey}`,
    },
    body: JSON.stringify({
      model,
      input: {
        prompt: req.prompt,
        ...(req.width && { width: req.width }),
        ...(req.height && { height: req.height }),
      },
    }),
  })

  if (!createRes.ok) {
    const err = await createRes.text()
    throw new Error(`Replicate error: ${err}`)
  }

  const prediction = await createRes.json()
  const pollUrl = prediction.urls?.get || `${base}/v1/predictions/${prediction.id}`

  // Poll for completion (max 120s)
  const deadline = Date.now() + 120_000
  while (Date.now() < deadline) {
    await new Promise((r) => setTimeout(r, 2000))
    const pollRes = await fetch(pollUrl, {
      headers: { Authorization: `Bearer ${req.apiKey}` },
    })
    const status = await pollRes.json()
    if (status.status === 'succeeded') {
      const output = status.output
      return Array.isArray(output) ? output[0] : output
    }
    if (status.status === 'failed' || status.status === 'canceled') {
      throw new Error(`Replicate ${status.status}: ${status.error || ''}`)
    }
  }
  throw new Error('Replicate generation timed out')
}

export default defineEventHandler(async (event) => {
  const body = (await readBody(event)) as ImageGenerateRequest

  if (!body.prompt || !body.provider || !body.apiKey) {
    throw createError({ statusCode: 400, message: 'Missing required fields: prompt, provider, apiKey' })
  }

  try {
    let url: string
    switch (body.provider) {
      case 'openai':
      case 'custom':
        url = await generateOpenAI(body)
        break
      case 'gemini':
        url = await generateGemini(body)
        break
      case 'replicate':
        url = await generateReplicate(body)
        break
      default:
        throw new Error(`Unknown provider: ${body.provider}`)
    }
    return { url }
  } catch (err: any) {
    throw createError({ statusCode: 502, message: err.message || 'Image generation failed' })
  }
})
```

- [ ] **Step 2: Type check**

Run: `npx tsc --noEmit`
Expected: No type errors.

- [ ] **Step 3: Commit**

```bash
git add server/api/ai/image-generate.ts
git commit -m "feat(server): add multi-provider image generation endpoint"
```

---

## Task 4: Server — Service Test Endpoint

**Files:**
- Create: `server/api/ai/image-service-test.ts`

- [ ] **Step 1: Write the test endpoint**

Create `server/api/ai/image-service-test.ts`:

```typescript
import { defineEventHandler, readBody } from 'h3'
import type { ImageGenProvider } from '../../../src/types/image-service'

interface ServiceTestRequest {
  service: 'openverse' | ImageGenProvider
  apiKey: string
  model?: string
  baseUrl?: string
  clientId?: string
  clientSecret?: string
}

export default defineEventHandler(async (event) => {
  const body = (await readBody(event)) as ServiceTestRequest

  try {
    switch (body.service) {
      case 'openverse': {
        if (!body.clientId || !body.clientSecret) {
          return { valid: false, error: 'Client ID and secret are required' }
        }
        const res = await fetch('https://api.openverse.org/v1/auth_tokens/token/', {
          method: 'POST',
          headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
          body: `grant_type=client_credentials&client_id=${body.clientId}&client_secret=${body.clientSecret}`,
        })
        if (!res.ok) return { valid: false, error: 'Invalid OAuth credentials' }
        return { valid: true }
      }

      case 'openai':
      case 'custom': {
        const base = body.baseUrl || 'https://api.openai.com'
        const res = await fetch(`${base}/v1/models`, {
          headers: { Authorization: `Bearer ${body.apiKey}` },
        })
        return res.ok
          ? { valid: true }
          : { valid: false, error: 'Invalid API key' }
      }

      case 'gemini': {
        const base = body.baseUrl || 'https://generativelanguage.googleapis.com'
        const res = await fetch(`${base}/v1beta/models?key=${body.apiKey}`)
        return res.ok
          ? { valid: true }
          : { valid: false, error: 'Invalid API key' }
      }

      case 'replicate': {
        const base = body.baseUrl || 'https://api.replicate.com'
        const res = await fetch(`${base}/v1/models`, {
          headers: { Authorization: `Bearer ${body.apiKey}` },
        })
        return res.ok
          ? { valid: true }
          : { valid: false, error: 'Invalid API key' }
      }

      default:
        return { valid: false, error: `Unknown service: ${body.service}` }
    }
  } catch (err: any) {
    return { valid: false, error: err.message || 'Connection failed' }
  }
})
```

- [ ] **Step 2: Commit**

```bash
git add server/api/ai/image-service-test.ts
git commit -m "feat(server): add image service API key validation endpoint"
```

---

## Task 5: Settings Store

**Files:**
- Modify: `src/stores/agent-settings-store.ts`

- [ ] **Step 1: Add imports and state**

At the top of `agent-settings-store.ts`, add import:

```typescript
import type { ImageGenConfig } from '@/types/image-service'
import { DEFAULT_IMAGE_GEN_CONFIG } from '@/types/image-service'
```

In the `PersistedState` interface (around line 14), add:

```typescript
  imageGenConfig: ImageGenConfig
  openverseOAuth: { clientId: string; clientSecret: string } | null
```

In the `AgentSettingsState` interface (around line 21), add the actions:

```typescript
  setImageGenConfig: (config: Partial<ImageGenConfig>) => void
  setOpenverseOAuth: (oauth: { clientId: string; clientSecret: string } | null) => void
```

- [ ] **Step 2: Add defaults and actions to store creation**

In the initial state (around line 83), add:

```typescript
  imageGenConfig: DEFAULT_IMAGE_GEN_CONFIG,
  openverseOAuth: null,
```

Add actions:

```typescript
  setImageGenConfig: (updates) =>
    set((s) => ({
      imageGenConfig: { ...s.imageGenConfig, ...updates },
    })),
  setOpenverseOAuth: (oauth) => set({ openverseOAuth: oauth }),
```

- [ ] **Step 3: Update persist/hydrate**

In the `persist()` method (around line 136), add `imageGenConfig` and `openverseOAuth` to the serialized object:

```typescript
  imageGenConfig: state.imageGenConfig,
  openverseOAuth: state.openverseOAuth,
```

In the `hydrate()` method (around line 148), add restoration:

```typescript
  if (parsed.imageGenConfig) set({ imageGenConfig: parsed.imageGenConfig })
  if (parsed.openverseOAuth !== undefined) set({ openverseOAuth: parsed.openverseOAuth })
```

- [ ] **Step 4: Type check**

Run: `npx tsc --noEmit`
Expected: No type errors.

- [ ] **Step 5: Commit**

```bash
git add src/stores/agent-settings-store.ts
git commit -m "feat(store): add image generation config and Openverse OAuth to agent settings"
```

---

## Task 6: Canvas Store — Image Search Status

**Files:**
- Modify: `src/stores/canvas-store.ts`

- [ ] **Step 1: Add imageSearchStatuses to state**

In `CanvasStoreState` interface (around line 23), add:

```typescript
  imageSearchStatuses: Map<string, 'pending' | 'found' | 'failed'>
  setImageSearchStatus: (nodeId: string, status: 'pending' | 'found' | 'failed') => void
  clearImageSearchStatuses: () => void
```

In the initial state (around line 65), add:

```typescript
  imageSearchStatuses: new Map(),
  setImageSearchStatus: (nodeId, status) =>
    set((s) => {
      const next = new Map(s.imageSearchStatuses)
      next.set(nodeId, status)
      return { imageSearchStatuses: next }
    }),
  clearImageSearchStatuses: () => set({ imageSearchStatuses: new Map() }),
```

- [ ] **Step 2: Type check**

Run: `npx tsc --noEmit`
Expected: No type errors.

- [ ] **Step 3: Commit**

```bash
git add src/stores/canvas-store.ts
git commit -m "feat(store): add imageSearchStatuses to canvas store for runtime status tracking"
```

---

## Task 7: Popover Primitive + Settings UI — Images Tab

**Files:**
- Create: `src/components/ui/popover.tsx` (shadcn primitive)
- Create: `src/components/shared/agent-settings-images-page.tsx`
- Modify: `src/components/shared/agent-settings-dialog.tsx`

- [ ] **Step 0: Add shadcn Popover primitive**

The project uses shadcn/ui but is missing the Popover component (Tasks 8-9 need it). Run:

```bash
npx shadcn@latest add popover
```

Or manually create `src/components/ui/popover.tsx` from the shadcn Popover recipe using `@radix-ui/react-popover`.

- [ ] **Step 1: Update SettingsTab type**

At line 49 of `agent-settings-dialog.tsx`, change:
```typescript
type SettingsTab = 'agents' | 'mcp' | 'system'
```
to:
```typescript
type SettingsTab = 'agents' | 'mcp' | 'images' | 'system'
```

- [ ] **Step 2: Add Images tab button in sidebar**

In the tab navigation section (around line 630-647), add an "Images" tab button between MCP and System, following the existing button pattern.

- [ ] **Step 3: Create ImagesPage in a separate file**

Create `src/components/shared/agent-settings-images-page.tsx` (extracting to a separate file keeps `agent-settings-dialog.tsx` under 800 lines):

**Image Search section:**
- Status text: "Ready (Openverse + Wikimedia)" with green indicator
- Collapsible "Advanced" section:
  - Two text inputs: Client ID, Client Secret
  - "Register at Openverse" external link
  - Test button calling `/api/ai/image-service-test` with `service: 'openverse'`

**Image Generation section:**
- Provider Select dropdown: OpenAI / Google Gemini / Replicate / Custom
- API Key password input + Test button
- Model text input with `placeholder` from `MODEL_PLACEHOLDERS[provider]`
- Collapsible "Advanced": Base URL text input

Wire up to `useAgentSettingsStore()` — `imageGenConfig`, `setImageGenConfig`, `openverseOAuth`, `setOpenverseOAuth`.

- [ ] **Step 4: Import and render in dialog**

In `agent-settings-dialog.tsx`, import the extracted component:
```typescript
import { ImagesPage } from './agent-settings-images-page'
```

In the tab content section (around line 666-704), add:
```typescript
{activeTab === 'images' && <ImagesPage />}
```

- [ ] **Step 5: Verify in browser**

Run: `bun --bun run dev`
Open Settings dialog → confirm 4 tabs visible, Images tab renders correctly, provider dropdown switches placeholder.

- [ ] **Step 6: Commit**

```bash
git add src/components/ui/popover.tsx src/components/shared/agent-settings-images-page.tsx src/components/shared/agent-settings-dialog.tsx
git commit -m "feat(editor): add Images tab to agent settings dialog"
```

---

## Task 8: Image Search Popover

**Files:**
- Create: `src/components/panels/image-search-popover.tsx`

- [ ] **Step 1: Create the search popover component**

Create `src/components/panels/image-search-popover.tsx`:

```typescript
import { useState, useCallback } from 'react'
import { Search, Loader2, ImageIcon } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import type { ImageSearchResult } from '@/types/image-service'
import { useAgentSettingsStore } from '@/stores/agent-settings-store'

interface ImageSearchPopoverProps {
  initialQuery: string
  onSelect: (url: string) => void
  children: React.ReactNode
}

export function ImageSearchPopover({
  initialQuery,
  onSelect,
  children,
}: ImageSearchPopoverProps) {
  const [open, setOpen] = useState(false)
  const [query, setQuery] = useState(initialQuery)
  const [results, setResults] = useState<ImageSearchResult[]>([])
  const [loading, setLoading] = useState(false)
  const [source, setSource] = useState<'openverse' | 'wikimedia'>('openverse')
  const openverseOAuth = useAgentSettingsStore((s) => s.openverseOAuth)

  const handleSearch = useCallback(async () => {
    if (!query.trim()) return
    setLoading(true)
    try {
      const res = await fetch('/api/ai/image-search', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          query: query.trim(),
          count: 5,
          ...(openverseOAuth && {
            openverseClientId: openverseOAuth.clientId,
            openverseClientSecret: openverseOAuth.clientSecret,
          }),
        }),
      })
      const data = await res.json()
      setResults(data.results || [])
      setSource(data.source || 'openverse')
    } catch {
      setResults([])
    } finally {
      setLoading(false)
    }
  }, [query, openverseOAuth])

  const handleSelect = (result: ImageSearchResult) => {
    onSelect(result.thumbUrl)
    setOpen(false)
  }

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>{children}</PopoverTrigger>
      <PopoverContent className="w-80 p-3" side="left" align="start">
        <div className="flex gap-1.5 mb-2">
          <input
            className="flex-1 rounded border border-border bg-background px-2 py-1 text-xs"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
            placeholder="Search images..."
          />
          <Button size="sm" variant="ghost" onClick={handleSearch} disabled={loading}>
            {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Search className="h-3.5 w-3.5" />}
          </Button>
        </div>

        {results.length > 0 ? (
          <>
            <div className="grid grid-cols-3 gap-1.5">
              {results.map((r) => (
                <button
                  key={r.id}
                  className="aspect-square overflow-hidden rounded border border-border hover:border-primary cursor-pointer"
                  onClick={() => handleSelect(r)}
                >
                  <img
                    src={r.thumbUrl}
                    alt=""
                    className="h-full w-full object-cover"
                    loading="lazy"
                  />
                </button>
              ))}
            </div>
            <p className="mt-1.5 text-[10px] text-muted-foreground">
              {results[0]?.license} · {source === 'openverse' ? 'Openverse' : 'Wikimedia'}
            </p>
          </>
        ) : (
          !loading && (
            <div className="flex flex-col items-center py-4 text-muted-foreground">
              <ImageIcon className="h-6 w-6 mb-1" />
              <p className="text-xs">Search for images</p>
            </div>
          )
        )}
      </PopoverContent>
    </Popover>
  )
}
```

- [ ] **Step 2: Type check**

Run: `npx tsc --noEmit`
Expected: No type errors.

- [ ] **Step 3: Commit**

```bash
git add src/components/panels/image-search-popover.tsx
git commit -m "feat(panels): add image search popover with Openverse/Wikimedia results grid"
```

---

## Task 9: Image Generate Popover

**Files:**
- Create: `src/components/panels/image-generate-popover.tsx`

- [ ] **Step 1: Create the generate popover component**

Create `src/components/panels/image-generate-popover.tsx`:

```typescript
import { useState, useCallback } from 'react'
import { Sparkles, Loader2, Settings, ImageIcon } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { useAgentSettingsStore } from '@/stores/agent-settings-store'

interface ImageGeneratePopoverProps {
  initialPrompt: string
  onGenerated: (url: string) => void
  children: React.ReactNode
}

export function ImageGeneratePopover({
  initialPrompt,
  onGenerated,
  children,
}: ImageGeneratePopoverProps) {
  const [open, setOpen] = useState(false)
  const [prompt, setPrompt] = useState(initialPrompt)
  const [previewUrl, setPreviewUrl] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const imageGenConfig = useAgentSettingsStore((s) => s.imageGenConfig)
  const setDialogOpen = useAgentSettingsStore((s) => s.setDialogOpen)

  const isConfigured = imageGenConfig.apiKey.length > 0

  const handleGenerate = useCallback(async () => {
    if (!prompt.trim() || !isConfigured) return
    setLoading(true)
    setError(null)
    setPreviewUrl(null)
    try {
      const res = await fetch('/api/ai/image-generate', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          prompt: prompt.trim(),
          provider: imageGenConfig.provider,
          model: imageGenConfig.model,
          apiKey: imageGenConfig.apiKey,
          baseUrl: imageGenConfig.baseUrl,
        }),
      })
      if (!res.ok) {
        const data = await res.json().catch(() => ({}))
        throw new Error(data.message || 'Generation failed')
      }
      const data = await res.json()
      setPreviewUrl(data.url)
    } catch (err: any) {
      setError(err.message || 'Generation failed')
    } finally {
      setLoading(false)
    }
  }, [prompt, imageGenConfig, isConfigured])

  const handleApply = () => {
    if (previewUrl) {
      onGenerated(previewUrl)
      setOpen(false)
    }
  }

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>{children}</PopoverTrigger>
      <PopoverContent className="w-72 p-3" side="left" align="start">
        {!isConfigured ? (
          <div className="flex flex-col items-center gap-2 py-3">
            <Settings className="h-6 w-6 text-muted-foreground" />
            <p className="text-xs text-muted-foreground text-center">
              Image generation not configured
            </p>
            <Button
              size="sm"
              variant="outline"
              onClick={() => {
                setDialogOpen(true)
                setOpen(false)
              }}
            >
              Open Settings
            </Button>
          </div>
        ) : (
          <>
            <textarea
              className="w-full rounded border border-border bg-background px-2 py-1.5 text-xs resize-none"
              rows={2}
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              placeholder="Describe the image..."
            />

            {previewUrl ? (
              <div className="mt-2">
                <img
                  src={previewUrl}
                  alt=""
                  className="w-full rounded border border-border"
                />
                <Button size="sm" className="w-full mt-1.5" onClick={handleApply}>
                  Apply
                </Button>
              </div>
            ) : loading ? (
              <div className="flex flex-col items-center py-6">
                <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                <p className="text-xs text-muted-foreground mt-2">Generating...</p>
              </div>
            ) : (
              <div className="mt-2">
                {error && (
                  <p className="text-xs text-destructive mb-1.5">{error}</p>
                )}
                <Button
                  size="sm"
                  className="w-full"
                  onClick={handleGenerate}
                  disabled={!prompt.trim()}
                >
                  <Sparkles className="h-3.5 w-3.5 mr-1" />
                  Generate
                </Button>
                <p className="text-[10px] text-muted-foreground mt-1 text-center">
                  {imageGenConfig.provider} · {imageGenConfig.model || 'default'}
                </p>
              </div>
            )}
          </>
        )}
      </PopoverContent>
    </Popover>
  )
}
```

- [ ] **Step 2: Type check**

Run: `npx tsc --noEmit`
Expected: No type errors.

- [ ] **Step 3: Commit**

```bash
git add src/components/panels/image-generate-popover.tsx
git commit -m "feat(panels): add image generate popover with multi-provider support"
```

---

## Task 10: Image Section Extension

**Files:**
- Modify: `src/components/panels/image-section.tsx`

- [ ] **Step 1: Add Search/Generate buttons to image section**

Import the new popovers at the top:

```typescript
import { ImageSearchPopover } from './image-search-popover'
import { ImageGeneratePopover } from './image-generate-popover'
import { Search, Sparkles } from 'lucide-react'
```

After the existing image preview button (around line 51) and before the popover, add a row of action buttons:

```typescript
<div className="flex gap-1 mt-1.5">
  <ImageSearchPopover
    initialQuery={node.imagePrompt ?? node.name ?? ''}
    onSelect={(url) => onUpdate({ src: url })}
  >
    <Button size="sm" variant="outline" className="flex-1 h-7 text-xs">
      <Search className="h-3 w-3 mr-1" />
      Search
    </Button>
  </ImageSearchPopover>

  <ImageGeneratePopover
    initialPrompt={node.imagePrompt ?? node.name ?? ''}
    onGenerated={(url) => onUpdate({ src: url })}
  >
    <Button size="sm" variant="outline" className="flex-1 h-7 text-xs">
      <Sparkles className="h-3 w-3 mr-1" />
      Generate
    </Button>
  </ImageGeneratePopover>
</div>
```

- [ ] **Step 2: Verify in browser**

Run: `bun --bun run dev`
Select an image node → confirm Search and Generate buttons appear in the property panel, popovers open correctly.

- [ ] **Step 3: Commit**

```bash
git add src/components/panels/image-section.tsx
git commit -m "feat(panels): add Search and Generate buttons to image property section"
```

---

## Task 11: Auto-Search Pipeline

**Files:**
- Create: `src/services/ai/image-search-pipeline.ts`
- Test: `src/services/ai/__tests__/image-search-pipeline.test.ts`

- [ ] **Step 1: Write the pipeline module**

Create `src/services/ai/image-search-pipeline.ts`:

```typescript
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import { useAgentSettingsStore } from '@/stores/agent-settings-store'
import { forcePageResync } from '@/canvas/canvas-sync-utils'
import type { PenNode, ImageNode } from '@/types/pen'

export function inferAspectRatio(
  node: PenNode,
): 'wide' | 'tall' | 'square' | undefined {
  const w = typeof node.width === 'number' ? node.width : 0
  const h = typeof node.height === 'number' ? node.height : 0
  if (!w || !h) return undefined
  const ratio = w / h
  if (ratio > 1.3) return 'wide'
  if (ratio < 0.77) return 'tall'
  return 'square'
}

export function collectImageNodes(rootId: string): ImageNode[] {
  const { getNodeById } = useDocumentStore.getState()
  const root = getNodeById(rootId)
  if (!root) return []

  const images: ImageNode[] = []
  const walk = (node: PenNode) => {
    if (node.type === 'image') images.push(node)
    if ('children' in node && Array.isArray(node.children)) {
      for (const child of node.children) walk(child)
    }
  }
  walk(root)
  return images
}

// Only treat as placeholder if no src or matches known placeholder pattern.
// Do NOT match user-uploaded SVGs (they won't have the phone placeholder prefix).
const PHONE_PLACEHOLDER_PREFIX = 'data:image/svg+xml;charset=utf-8,%3Csvg'

function isPlaceholderSrc(src?: string): boolean {
  return !src || src.startsWith(PHONE_PLACEHOLDER_PREFIX)
}

// Module-level abort controller for cancellation
let currentAbort: AbortController | null = null

export async function scanAndFillImages(rootId: string): Promise<void> {
  // Cancel any previous scan
  currentAbort?.abort()
  const abort = new AbortController()
  currentAbort = abort

  const imageNodes = collectImageNodes(rootId)
  const needsFill = imageNodes.filter((n) => isPlaceholderSrc(n.src))

  if (needsFill.length === 0) return

  const { setImageSearchStatus } = useCanvasStore.getState()
  const { updateNode } = useDocumentStore.getState()
  const { openverseOAuth } = useAgentSettingsStore.getState()

  // Mark all as pending
  for (const node of needsFill) {
    setImageSearchStatus(node.id, 'pending')
  }

  for (const node of needsFill) {
    if (abort.signal.aborted) return

    const query =
      (node.type === 'image' ? node.imagePrompt : undefined) ??
      node.name ??
      'placeholder'
    const aspect = inferAspectRatio(node)

    try {
      const res = await fetch('/api/ai/image-search', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          query,
          count: 1,
          aspectRatio: aspect,
          ...(openverseOAuth && {
            openverseClientId: openverseOAuth.clientId,
            openverseClientSecret: openverseOAuth.clientSecret,
          }),
        }),
        signal: abort.signal,
      })
      const data = await res.json()
      if (data.results?.length > 0) {
        updateNode(node.id, { src: data.results[0].thumbUrl })
        setImageSearchStatus(node.id, 'found')
      } else {
        setImageSearchStatus(node.id, 'failed')
      }
    } catch {
      if (!abort.signal.aborted) {
        setImageSearchStatus(node.id, 'failed')
      }
    }

    // Rate limit: 3s between requests to stay under Openverse 20/min burst limit
    if (!abort.signal.aborted) {
      await new Promise((r) => setTimeout(r, 3000))
    }
  }

  if (!abort.signal.aborted) {
    forcePageResync()
  }
}
```

- [ ] **Step 2: Write unit tests**

Create `src/services/ai/__tests__/image-search-pipeline.test.ts`:

```typescript
import { describe, it, expect } from 'vitest'
import { inferAspectRatio } from '../image-search-pipeline'
import type { PenNode } from '@/types/pen'

function makeImageNode(w: number, h: number): PenNode {
  return { id: 'test', type: 'image', src: '', width: w, height: h } as PenNode
}

describe('inferAspectRatio', () => {
  it('returns wide for landscape images', () => {
    expect(inferAspectRatio(makeImageNode(1200, 600))).toBe('wide')
  })

  it('returns tall for portrait images', () => {
    expect(inferAspectRatio(makeImageNode(400, 800))).toBe('tall')
  })

  it('returns square for roughly equal dimensions', () => {
    expect(inferAspectRatio(makeImageNode(500, 500))).toBe('square')
    expect(inferAspectRatio(makeImageNode(600, 500))).toBe('square')
  })

  it('returns undefined when dimensions missing', () => {
    expect(inferAspectRatio({ id: 'x', type: 'image', src: '' } as PenNode)).toBeUndefined()
  })
})
```

- [ ] **Step 3: Run tests**

Run: `bun --bun vitest run src/services/ai/__tests__/image-search-pipeline.test.ts`
Expected: All 4 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/services/ai/image-search-pipeline.ts src/services/ai/__tests__/image-search-pipeline.test.ts
git commit -m "feat(ai): add auto-search pipeline with Openverse/Wikimedia fallback"
```

---

## Task 12: Integration — Design Generation

**Files:**
- Modify: `src/services/ai/design-canvas-ops.ts`

**Note:** The primary in-app generation completion path is `animateNodesToCanvas()` (called from `ai-chat-handlers.ts:258`). All three completion functions must be hooked. Use `getGenerationRootFrameId()` (exported from this module, line ~48) to get the root ID since the completion functions don't have `rootId` as a local variable.

- [ ] **Step 1: Import and call scanAndFillImages**

At the top of `design-canvas-ops.ts`, add:

```typescript
import { scanAndFillImages } from './image-search-pipeline'
```

In `applyNodesToCanvas()` (around line 315, after `resolveAllPendingIcons()`), add:

```typescript
const rootId = getGenerationRootFrameId()
if (rootId) scanAndFillImages(rootId).catch(() => {})
```

In `upsertNodesToCanvas()` (around line 346, after `adjustRootFrameHeightToContent()`), add the same.

In `animateNodesToCanvas()` (around line 395, after `resolveAllPendingIcons()`), add the same. **This is the most important one** — it's the primary path for AI chat-based design generation.

- [ ] **Step 2: Type check**

Run: `npx tsc --noEmit`
Expected: No type errors.

- [ ] **Step 3: Commit**

```bash
git add src/services/ai/design-canvas-ops.ts
git commit -m "feat(ai): trigger auto image search after design generation completes"
```

---

## Task 13: AI Prompt Changes

**Files:**
- Modify: `src/services/ai/ai-prompts.ts:14`
- Modify: `src/services/ai/orchestrator-prompts.ts:63`

- [ ] **Step 1: Update ai-prompts.ts**

At line 14, change the image node description from:
```
- image: Raster image. Props: src (URL string), width, height, cornerRadius, effects
```
to:
```
- image: Raster image. Props: width, height, cornerRadius, effects, imagePrompt (recommended: descriptive English phrase for image content, e.g. "modern office workspace", "smiling woman headshot"). Do NOT include src — images are auto-populated after generation. Omit imagePrompt for purely decorative images.
```

- [ ] **Step 2: Update orchestrator-prompts.ts**

At line 63, update the image type in the TYPES listing from:
```
image (src,width,height)
```
to:
```
image (width,height,imagePrompt)
```

- [ ] **Step 3: Commit**

```bash
git add src/services/ai/ai-prompts.ts src/services/ai/orchestrator-prompts.ts
git commit -m "feat(ai): update prompts to use imagePrompt instead of src for image nodes"
```

---

## Task 14: MCP — G() Operation

**Files:**
- Modify: `src/mcp/tools/batch-design.ts:134-220`

**IMPORTANT:** MCP tools run in a Node.js process (stdio/HTTP server), NOT in the browser. They have no access to Zustand stores or relative `fetch()` URLs. Use `getSyncUrl()` from `document-manager.ts` to get the Nitro server base URL for API calls. Image generation config must be passed as MCP tool arguments (the MCP process cannot read browser localStorage).

- [ ] **Step 1: Update DSL regex**

At line 134, change:
```typescript
/^(\w+)\s*=\s*([ICRM])\((.+)\)$/
```
to:
```typescript
/^(\w+)\s*=\s*([ICRMG])\((.+)\)$/
```

- [ ] **Step 2: Add G case in switch**

In the first switch block (after the `case 'M'` ending around line 218), add a new case. Note: use absolute URL via `getSyncUrl()`:

```typescript
case 'G': {
  // G("parentId", "search|generate", "prompt text")
  const gArgs = args.match(/^"([^"]+)"\s*,\s*"(search|generate)"\s*,\s*"([^"]+)"$/)
  if (!gArgs) throw new Error(`Invalid G() syntax: ${args}`)
  const [, gParent, gMode, gPrompt] = gArgs
  const resolvedParent = resolveRef(gParent)

  const imageNode = {
    id: generateId(),
    type: 'image' as const,
    name: gPrompt.slice(0, 40),
    imagePrompt: gPrompt,
    src: '',
    width: 400,
    height: 300,
  }

  // MCP runs in Node.js — must use absolute URL via getSyncUrl()
  const syncUrl = getSyncUrl()
  if (gMode === 'search' && syncUrl) {
    try {
      const searchRes = await fetch(`${syncUrl}/api/ai/image-search`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ query: gPrompt, count: 1 }),
      })
      const searchData = await searchRes.json()
      if (searchData.results?.length > 0) {
        imageNode.src = searchData.results[0].thumbUrl
      }
    } catch { /* keep empty src — user fills manually via Property Panel */ }
  }
  // Note: "generate" mode is NOT supported in MCP context because
  // image gen API keys are stored in browser localStorage, not available
  // to the MCP process. The node is created with imagePrompt set;
  // the user generates via the Property Panel.

  insertNodeInTree(doc, resolvedParent, imageNode)
  vars[varName] = imageNode.id
  break
}
```

- [ ] **Step 3: Make executeLine async-aware**

The `executeLine` function and the loop calling it need to handle the async `G()` operation. Change its signature to `async function executeLine(...)` and `await` it in the calling `handleBatchDesign` loop.

- [ ] **Step 4: Import getSyncUrl**

Add at top of `batch-design.ts`:
```typescript
import { getSyncUrl } from '../document-manager'
```

- [ ] **Step 5: Type check**

Run: `npx tsc --noEmit`
Expected: No type errors.

- [ ] **Step 6: Commit**

```bash
git add src/mcp/tools/batch-design.ts
git commit -m "feat(mcp): implement G() operation for image search in batch design DSL"
```

---

## Task 15: MCP — Layered Design Integration

**Files:**
- Modify: `src/mcp/tools/design-refine.ts:95-107`

**IMPORTANT:** `design-refine.ts` runs in the MCP Node.js process. It CANNOT import `scanAndFillImages` (which uses browser-only Zustand stores). Instead, iterate image nodes in the PenDocument directly and call the search API via `getSyncUrl()`.

- [ ] **Step 1: Add image search after refinement**

Import at top:
```typescript
import { getSyncUrl } from '../document-manager'
```

After the refinement processing is complete (around line 97), add a helper that walks the doc tree, finds image nodes with placeholder src, and calls the search API:

```typescript
// Auto-fill placeholder images via server API
const syncUrl = getSyncUrl()
if (syncUrl) {
  const walk = async (node: any) => {
    if (node.type === 'image' && (!node.src || node.src.startsWith('data:image/svg+xml;charset=utf-8,%3Csvg'))) {
      const query = node.imagePrompt ?? node.name ?? 'placeholder'
      try {
        const res = await fetch(`${syncUrl}/api/ai/image-search`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ query, count: 1 }),
        })
        const data = await res.json()
        if (data.results?.length > 0) {
          node.src = data.results[0].thumbUrl
        }
      } catch { /* non-fatal */ }
      await new Promise(r => setTimeout(r, 3000)) // rate limit
    }
    if (node.children) {
      for (const child of node.children) await walk(child)
    }
  }
  // Walk children of the active page or root
  const children = getDocChildren(doc)
  for (const child of children) await walk(child)
}
```

- [ ] **Step 2: Type check**

Run: `npx tsc --noEmit`
Expected: No type errors.

- [ ] **Step 3: Commit**

```bash
git add src/mcp/tools/design-refine.ts
git commit -m "feat(mcp): auto-fill images after design refinement in layered pipeline"
```

---

## Task 16: Full Integration Test

- [ ] **Step 1: Run all tests**

Run: `bun --bun run test`
Expected: All existing + new tests pass.

- [ ] **Step 2: Type check entire project**

Run: `npx tsc --noEmit`
Expected: No type errors.

- [ ] **Step 3: Build check**

Run: `bun --bun run build`
Expected: Build succeeds.

- [ ] **Step 4: Manual E2E verification**

Run: `bun --bun run dev`

1. Open editor, generate a design with image nodes → verify auto-search fills images
2. Select an image node → verify Search/Generate buttons in property panel
3. Click Search → verify popover shows, search returns results, click applies image
4. Click Generate (unconfigured) → verify "Open Settings" prompt
5. Configure image gen in Settings → Images → verify provider/model/key fields
6. Click Generate (configured) → verify image generates and applies
7. Open Settings → Images → verify Openverse Advanced section works

- [ ] **Step 5: Final commit**

```bash
git commit --allow-empty -m "feat: image search & generation for design placeholders — complete"
```
