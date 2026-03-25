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

    const json = (await res.json()) as { data?: Array<{ id: string; name?: string }> }
    if (!json.data || !Array.isArray(json.data)) {
      return { models: [], error: 'Unexpected response format (missing data array)' }
    }

    const models: ModelEntry[] = json.data
      .filter((m) => m.id)
      .map((m) => ({
        id: m.id,
        name: (m as Record<string, unknown>).name as string || m.id,
      }))
      .sort((a, b) => a.name.localeCompare(b.name))

    return { models }
  } catch (err) {
    const message = err instanceof Error ? err.message : 'Unknown error'
    return { models: [], error: message }
  }
})
