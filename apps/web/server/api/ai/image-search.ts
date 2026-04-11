import { defineEventHandler, readBody, setResponseHeaders } from 'h3';
import type { ImageSearchResult, ImageSearchResponse } from '../../../src/types/image-service';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface OpenverseImageResult {
  id: string;
  url: string;
  thumbnail: string;
  width: number;
  height: number;
  license: string;
  license_version: string;
  attribution: string;
}

interface OpenverseSearchResponse {
  results: OpenverseImageResult[];
}

interface WikimediaImageInfo {
  url: string;
  thumburl: string;
  width: number;
  height: number;
  mime: string;
  extmetadata?: {
    LicenseShortName?: { value: string };
  };
}

interface WikimediaPage {
  pageid: number;
  title: string;
  imageinfo?: WikimediaImageInfo[];
}

interface WikimediaQueryResponse {
  query?: {
    pages?: Record<string, WikimediaPage>;
  };
}

// ---------------------------------------------------------------------------
// OAuth token cache
// ---------------------------------------------------------------------------

let cachedToken: string | null = null;
let tokenExpiresAt = 0;

async function getOpenverseToken(clientId: string, clientSecret: string): Promise<string | null> {
  const now = Date.now();
  if (cachedToken && now < tokenExpiresAt) {
    return cachedToken;
  }

  try {
    const body = new URLSearchParams({
      grant_type: 'client_credentials',
      client_id: clientId,
      client_secret: clientSecret,
    });
    const res = await fetch('https://api.openverse.org/v1/auth_tokens/token/', {
      method: 'POST',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body: body.toString(),
    });
    if (!res.ok) return null;
    const data = (await res.json()) as { access_token: string; expires_in: number };
    cachedToken = data.access_token;
    // Refresh 60 seconds before expiry
    tokenExpiresAt = now + (data.expires_in - 60) * 1000;
    return cachedToken;
  } catch {
    return null;
  }
}

// ---------------------------------------------------------------------------
// Query simplification — convert verbose AI prompts to search keywords
// ---------------------------------------------------------------------------

const STOP_WORDS = new Set([
  'a',
  'an',
  'the',
  'and',
  'or',
  'but',
  'in',
  'on',
  'at',
  'to',
  'for',
  'of',
  'with',
  'by',
  'from',
  'is',
  'are',
  'was',
  'were',
  'be',
  'been',
  'being',
  'have',
  'has',
  'had',
  'do',
  'does',
  'did',
  'will',
  'would',
  'could',
  'should',
  'may',
  'might',
  'shall',
  'can',
  'that',
  'this',
  'these',
  'those',
  'it',
  'its',
  'very',
  'really',
  'just',
  'also',
  'about',
  'above',
  'after',
  'before',
  'between',
  'into',
  'through',
  'during',
  'each',
  'some',
  'such',
  'no',
  'not',
  'only',
  'same',
  'so',
  'than',
  'too',
  'up',
  'out',
  'if',
  'then',
  'once',
  'here',
  'there',
  'when',
  'where',
  'how',
  'all',
  'both',
  'few',
  'more',
  'most',
  'other',
  'any',
  'as',
  'while',
  'using',
  'showing',
  'featuring',
  'looking',
  'style',
  'styled',
  'inspired',
  'based',
]);

/**
 * Simplify a verbose image generation prompt into 2-4 search keywords.
 * "delicious burger with fries and fresh vegetables" → "burger fries vegetables"
 * "modern office workspace with natural lighting" → "modern office workspace"
 */
export function simplifySearchQuery(prompt: string): string {
  const words = prompt
    .toLowerCase()
    .replace(/[^a-z0-9\s-]/g, ' ')
    .split(/\s+/)
    .filter((w) => w.length > 2 && !STOP_WORDS.has(w));

  // Take up to 4 keywords
  const keywords = words.slice(0, 4);
  return keywords.join(' ') || prompt.slice(0, 30);
}

// ---------------------------------------------------------------------------
// Mapping helpers (exported for testing)
// ---------------------------------------------------------------------------

export function mapOpenverseResult(r: OpenverseImageResult): ImageSearchResult {
  return {
    id: r.id,
    url: r.url,
    thumbUrl: r.thumbnail,
    width: r.width,
    height: r.height,
    source: 'openverse',
    license: `${r.license} ${r.license_version}`.trim(),
    attribution: r.attribution,
  };
}

export function mapWikimediaPages(pages: Record<string, WikimediaPage>): ImageSearchResult[] {
  const results: ImageSearchResult[] = [];
  for (const page of Object.values(pages)) {
    const info = page.imageinfo?.[0];
    if (!info) continue;
    results.push({
      id: String(page.pageid),
      url: info.url,
      thumbUrl: info.thumburl ?? info.url,
      width: info.width,
      height: info.height,
      source: 'wikimedia',
      license: info.extmetadata?.LicenseShortName?.value ?? '',
    });
  }
  return results;
}

// ---------------------------------------------------------------------------
// Source fetchers
// ---------------------------------------------------------------------------

async function fetchFromOpenverse(
  query: string,
  count: number,
  aspectRatio: string | undefined,
  clientId: string | undefined,
  clientSecret: string | undefined,
): Promise<ImageSearchResult[] | null> {
  const url = new URL('https://api.openverse.org/v1/images/');
  url.searchParams.set('q', query);
  url.searchParams.set('page_size', String(count));
  if (aspectRatio) {
    url.searchParams.set('aspect_ratio', aspectRatio);
  }

  const headers: Record<string, string> = {};
  if (clientId && clientSecret) {
    const token = await getOpenverseToken(clientId, clientSecret);
    if (token) {
      headers['Authorization'] = `Bearer ${token}`;
    }
  }

  const res = await fetch(url.toString(), { headers });
  if (res.status === 429) {
    // Rate limited — signal fallback
    return null;
  }
  if (!res.ok) {
    return null;
  }

  const data = (await res.json()) as OpenverseSearchResponse;
  return (data.results ?? []).map(mapOpenverseResult);
}

async function fetchFromWikimedia(query: string, count: number): Promise<ImageSearchResult[]> {
  const url = new URL('https://commons.wikimedia.org/w/api.php');
  url.searchParams.set('action', 'query');
  url.searchParams.set('generator', 'search');
  url.searchParams.set('gsrsearch', query);
  url.searchParams.set('gsrnamespace', '6');
  url.searchParams.set('gsrlimit', String(count));
  url.searchParams.set('prop', 'imageinfo');
  url.searchParams.set('iiprop', 'url|size|mime|extmetadata');
  url.searchParams.set('iiurlwidth', '800');
  url.searchParams.set('format', 'json');
  url.searchParams.set('origin', '*');

  const res = await fetch(url.toString());
  if (!res.ok) return [];

  const data = (await res.json()) as WikimediaQueryResponse;
  const pages = data.query?.pages;
  if (!pages) return [];

  return mapWikimediaPages(pages);
}

// ---------------------------------------------------------------------------
// Endpoint
// ---------------------------------------------------------------------------

/**
 * POST /api/ai/image-search
 *
 * Searches for freely-licensed images.
 * Primary source: Openverse. Falls back to Wikimedia Commons on 429.
 *
 * Body: { query, count?, aspectRatio?, openverseClientId?, openverseClientSecret? }
 */
export default defineEventHandler(async (event) => {
  setResponseHeaders(event, { 'Content-Type': 'application/json' });

  const body = (await readBody(event)) as {
    query?: string;
    count?: number;
    aspectRatio?: string;
    openverseClientId?: string;
    openverseClientSecret?: string;
  };

  const rawQuery = body?.query?.trim() ?? '';
  if (!rawQuery) {
    return { error: 'Missing required field: query' };
  }

  // Simplify verbose AI prompts into search-friendly keywords
  const query = simplifySearchQuery(rawQuery);

  const count = Math.min(Math.max(Number(body?.count ?? 10), 1), 50);
  const aspectRatio = body?.aspectRatio;
  const clientId = body?.openverseClientId;
  const clientSecret = body?.openverseClientSecret;

  // Try Openverse first
  const openverseResults = await fetchFromOpenverse(
    query,
    count,
    aspectRatio,
    clientId,
    clientSecret,
  );

  if (openverseResults !== null) {
    return {
      results: openverseResults,
      source: 'openverse',
    } satisfies ImageSearchResponse;
  }

  // Openverse returned 429 or failed — fall back to Wikimedia
  const wikimediaResults = await fetchFromWikimedia(query, count);
  return {
    results: wikimediaResults,
    source: 'wikimedia',
  } satisfies ImageSearchResponse;
});
