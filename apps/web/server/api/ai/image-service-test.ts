import { defineEventHandler, readBody } from 'h3';

interface ImageServiceTestRequest {
  service: string;
  apiKey?: string;
  model?: string;
  baseUrl?: string;
  clientId?: string;
  clientSecret?: string;
}

interface ImageServiceTestResponse {
  valid: boolean;
  error?: string;
}

/**
 * POST /api/ai/image-service-test
 *
 * Validates API keys for image generation services.
 * Returns { valid: boolean, error?: string }
 */
export default defineEventHandler(async (event): Promise<ImageServiceTestResponse> => {
  const body = await readBody<ImageServiceTestRequest>(event);
  const { service, apiKey, baseUrl, clientId, clientSecret } = body ?? {};

  if (!service) {
    return { valid: false, error: 'Missing required field: service' };
  }

  try {
    switch (service) {
      case 'openverse': {
        if (!clientId || !clientSecret) {
          return { valid: false, error: 'Openverse requires clientId and clientSecret' };
        }
        const formData = new URLSearchParams();
        formData.set('grant_type', 'client_credentials');
        formData.set('client_id', clientId);
        formData.set('client_secret', clientSecret);
        const res = await fetch('https://api.openverse.org/v1/auth_tokens/token/', {
          method: 'POST',
          headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
          body: formData.toString(),
        });
        if (!res.ok) {
          const text = await res.text().catch(() => '');
          return { valid: false, error: `Openverse auth failed (${res.status}): ${text}` };
        }
        return { valid: true };
      }

      case 'openai':
      case 'custom': {
        if (!apiKey) {
          return { valid: false, error: 'Missing required field: apiKey' };
        }
        const origin = baseUrl ?? 'https://api.openai.com';
        const res = await fetch(`${origin}/v1/models`, {
          headers: { Authorization: `Bearer ${apiKey}` },
        });
        if (!res.ok) {
          const text = await res.text().catch(() => '');
          return { valid: false, error: `Models request failed (${res.status}): ${text}` };
        }
        return { valid: true };
      }

      case 'gemini': {
        if (!apiKey) {
          return { valid: false, error: 'Missing required field: apiKey' };
        }
        const origin = baseUrl ?? 'https://generativelanguage.googleapis.com';
        const res = await fetch(`${origin}/v1beta/models?key=${encodeURIComponent(apiKey)}`);
        if (!res.ok) {
          const text = await res.text().catch(() => '');
          return { valid: false, error: `Gemini models request failed (${res.status}): ${text}` };
        }
        return { valid: true };
      }

      case 'replicate': {
        if (!apiKey) {
          return { valid: false, error: 'Missing required field: apiKey' };
        }
        const origin = baseUrl ?? 'https://api.replicate.com';
        const res = await fetch(`${origin}/v1/models`, {
          headers: { Authorization: `Bearer ${apiKey}` },
        });
        if (!res.ok) {
          const text = await res.text().catch(() => '');
          return {
            valid: false,
            error: `Replicate models request failed (${res.status}): ${text}`,
          };
        }
        return { valid: true };
      }

      default:
        return { valid: false, error: `Unknown service: ${service}` };
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return { valid: false, error: message };
  }
});
