import { defineEventHandler, readBody } from 'h3'

interface ProviderModelsBody {
  baseURL: string
  apiKey?: string
}

interface ModelEntry {
  id: string
  name: string
}

/**
 * POST /api/ai/provider-models
 * Proxies model list requests to external providers to avoid CORS issues.
 * Body: { baseURL: string, apiKey?: string }
 * Returns: { models: Array<{ id: string, name: string }> }
 */
export default defineEventHandler(async (event) => {
  const body = await readBody<ProviderModelsBody>(event)
  if (!body?.baseURL) {
    return { models: [], error: 'baseURL is required' }
  }

  const url = body.baseURL.replace(/\/+$/, '') + '/models'
  const headers: Record<string, string> = {
    Accept: 'application/json',
  }
  if (body.apiKey) {
    headers.Authorization = `Bearer ${body.apiKey}`
  }

  try {
    const res = await fetch(url, { headers, signal: AbortSignal.timeout(10_000) })
    if (!res.ok) {
      const text = await res.text().catch(() => '')
      return { models: [], error: `Provider returned ${res.status}: ${text.slice(0, 200)}` }
    }

    const json = await res.json() as Record<string, unknown>
    // Handle different response formats: { data: [...] } (OpenAI), { models: [...] }, or [...]
    const rawModels = Array.isArray(json.data) ? json.data
      : Array.isArray(json.models) ? json.models
      : Array.isArray(json) ? json
      : null
    if (!rawModels) {
      return { models: [], error: 'Unexpected response format (no model array found)' }
    }

    const models: ModelEntry[] = (rawModels as Array<Record<string, unknown>>)
      .filter((m) => m.id)
      .map((m) => ({
        id: String(m.id),
        name: (typeof m.name === 'string' ? m.name : '') || String(m.id),
      }))
      .sort((a, b) => a.name.localeCompare(b.name))

    return { models }
  } catch (err) {
    const message = err instanceof Error ? err.message : 'Unknown error'
    return { models: [], error: message }
  }
})
