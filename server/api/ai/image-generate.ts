import { defineEventHandler, readBody, setResponseHeaders, createError } from 'h3'

interface ImageGenerateBody {
  prompt: string
  provider: 'openai' | 'custom' | 'gemini' | 'replicate'
  model: string
  apiKey: string
  baseUrl?: string
  width?: number
  height?: number
}

/**
 * POST /api/ai/image-generate
 *
 * Multi-provider image generation endpoint.
 * Supports OpenAI (dall-e-3/dall-e-2), Gemini (imagen), and Replicate.
 * Returns { url: string } — either a remote URL or a base64 data URL.
 */
export default defineEventHandler(async (event) => {
  setResponseHeaders(event, { 'Content-Type': 'application/json' })

  const body = await readBody<ImageGenerateBody>(event)

  if (!body?.prompt?.trim()) {
    throw createError({ statusCode: 400, message: 'Missing required field: prompt' })
  }
  if (!body?.provider) {
    throw createError({ statusCode: 400, message: 'Missing required field: provider' })
  }
  if (!body?.apiKey?.trim()) {
    throw createError({ statusCode: 400, message: 'Missing required field: apiKey' })
  }

  const { prompt, provider, model, apiKey, baseUrl, width, height } = body

  if (provider === 'openai' || provider === 'custom') {
    return await generateOpenAI({ prompt, model, apiKey, baseUrl, width, height })
  }

  if (provider === 'gemini') {
    return await generateGemini({ prompt, model, apiKey, baseUrl })
  }

  if (provider === 'replicate') {
    return await generateReplicate({ prompt, model, apiKey, baseUrl, width, height })
  }

  throw createError({ statusCode: 400, message: `Unsupported provider: ${provider}` })
})

// ---------------------------------------------------------------------------
// Size mapping
// ---------------------------------------------------------------------------

function mapToOpenAISize(w?: number, h?: number): string {
  if (!w || !h) return '1024x1024'
  const ratio = w / h
  if (ratio > 1.3) return '1792x1024'
  if (ratio < 0.77) return '1024x1792'
  return '1024x1024'
}

// ---------------------------------------------------------------------------
// OpenAI / custom OpenAI-compatible provider
// ---------------------------------------------------------------------------

async function generateOpenAI(opts: {
  prompt: string
  model: string
  apiKey: string
  baseUrl?: string
  width?: number
  height?: number
}): Promise<{ url: string }> {
  const { prompt, model, apiKey, baseUrl, width, height } = opts
  const size = mapToOpenAISize(width, height)
  const endpoint = `${baseUrl ?? 'https://api.openai.com'}/v1/images/generations`

  let res: Response
  try {
    res = await fetch(endpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${apiKey}`,
      },
      body: JSON.stringify({ model, prompt, n: 1, size, response_format: 'url' }),
    })
  } catch (err) {
    throw createError({ statusCode: 502, message: `OpenAI request failed: ${String(err)}` })
  }

  if (!res.ok) {
    const text = await res.text().catch(() => '')
    throw createError({
      statusCode: 502,
      message: `OpenAI returned ${res.status}: ${text.slice(0, 200)}`,
    })
  }

  const data = (await res.json()) as { data?: { url?: string }[] }
  const url = data?.data?.[0]?.url
  if (!url) {
    throw createError({ statusCode: 502, message: 'OpenAI response missing image URL' })
  }

  return { url }
}

// ---------------------------------------------------------------------------
// Gemini image generation
// ---------------------------------------------------------------------------

async function generateGemini(opts: {
  prompt: string
  model: string
  apiKey: string
  baseUrl?: string
}): Promise<{ url: string }> {
  const { prompt, model, apiKey, baseUrl } = opts
  const base = baseUrl ?? 'https://generativelanguage.googleapis.com'
  const endpoint = `${base}/v1beta/models/${model}:generateContent?key=${apiKey}`

  let res: Response
  try {
    res = await fetch(endpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        contents: [{ parts: [{ text: prompt }] }],
        generationConfig: { responseModalities: ['TEXT', 'IMAGE'] },
      }),
    })
  } catch (err) {
    throw createError({ statusCode: 502, message: `Gemini request failed: ${String(err)}` })
  }

  if (!res.ok) {
    const text = await res.text().catch(() => '')
    throw createError({
      statusCode: 502,
      message: `Gemini returned ${res.status}: ${text.slice(0, 200)}`,
    })
  }

  const data = (await res.json()) as {
    candidates?: {
      content?: {
        parts?: {
          inlineData?: { mimeType?: string; data?: string }
          text?: string
        }[]
      }
    }[]
  }

  const parts = data?.candidates?.[0]?.content?.parts ?? []
  const imagePart = parts.find((p) => p.inlineData?.mimeType?.startsWith('image/'))

  if (!imagePart?.inlineData?.data || !imagePart.inlineData.mimeType) {
    throw createError({ statusCode: 502, message: 'Gemini response missing inline image data' })
  }

  const { mimeType, data: base64data } = imagePart.inlineData
  return { url: `data:${mimeType};base64,${base64data}` }
}

// ---------------------------------------------------------------------------
// Replicate
// ---------------------------------------------------------------------------

async function generateReplicate(opts: {
  prompt: string
  model: string
  apiKey: string
  baseUrl?: string
  width?: number
  height?: number
}): Promise<{ url: string }> {
  const { prompt, model, apiKey, baseUrl, width, height } = opts
  const base = baseUrl ?? 'https://api.replicate.com'

  // Start prediction
  let createRes: Response
  try {
    const input: Record<string, unknown> = { prompt }
    if (width) input.width = width
    if (height) input.height = height

    createRes = await fetch(`${base}/v1/predictions`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${apiKey}`,
      },
      body: JSON.stringify({ model, input }),
    })
  } catch (err) {
    throw createError({ statusCode: 502, message: `Replicate request failed: ${String(err)}` })
  }

  if (!createRes.ok) {
    const text = await createRes.text().catch(() => '')
    throw createError({
      statusCode: 502,
      message: `Replicate returned ${createRes.status}: ${text.slice(0, 200)}`,
    })
  }

  const prediction = (await createRes.json()) as { id?: string; status?: string }
  const predictionId = prediction?.id
  if (!predictionId) {
    throw createError({ statusCode: 502, message: 'Replicate response missing prediction ID' })
  }

  // Poll until succeeded or failed (max 120s, polling every 2s)
  const maxAttempts = 60
  for (let attempt = 0; attempt < maxAttempts; attempt++) {
    await new Promise((resolve) => setTimeout(resolve, 2000))

    let pollRes: Response
    try {
      pollRes = await fetch(`${base}/v1/predictions/${predictionId}`, {
        headers: { Authorization: `Bearer ${apiKey}` },
      })
    } catch (err) {
      throw createError({
        statusCode: 502,
        message: `Replicate poll request failed: ${String(err)}`,
      })
    }

    if (!pollRes.ok) {
      const text = await pollRes.text().catch(() => '')
      throw createError({
        statusCode: 502,
        message: `Replicate poll returned ${pollRes.status}: ${text.slice(0, 200)}`,
      })
    }

    const status = (await pollRes.json()) as {
      id?: string
      status?: string
      output?: string | string[]
      error?: string
    }

    if (status.status === 'succeeded') {
      const output = status.output
      if (Array.isArray(output)) {
        const first = output[0]
        if (!first) {
          throw createError({
            statusCode: 502,
            message: 'Replicate succeeded but output array is empty',
          })
        }
        return { url: first }
      }
      if (typeof output === 'string') {
        return { url: output }
      }
      throw createError({ statusCode: 502, message: 'Replicate succeeded but output is missing' })
    }

    if (status.status === 'failed' || status.status === 'canceled') {
      throw createError({
        statusCode: 502,
        message: `Replicate prediction ${status.status}: ${status.error ?? 'unknown error'}`,
      })
    }

    // Still starting/processing — keep polling
  }

  throw createError({
    statusCode: 502,
    message: 'Replicate prediction timed out after 120 seconds',
  })
}
