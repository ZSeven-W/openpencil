import type { ParsedStyleGuide, StyleGuideMeta } from './style-guide-types';

/** Parse YAML frontmatter from a style guide markdown file */
function parseFrontmatter(content: string): { meta: StyleGuideMeta; body: string } | null {
  const match = content.match(/^---\n([\s\S]*?)\n---\n([\s\S]*)$/);
  if (!match) return null;

  const [, yaml, body] = match;
  const meta: Record<string, unknown> = {};
  for (const line of yaml.split('\n')) {
    const kv = line.match(/^(\w+):\s*(.+)$/);
    if (!kv) continue;
    const [, key, value] = kv;
    if (value.startsWith('[')) {
      meta[key] = value
        .slice(1, -1)
        .split(',')
        .map((s) => s.trim());
    } else {
      meta[key] = value.replace(/^["']|["']$/g, '');
    }
  }

  return {
    meta: {
      name: (meta.name as string) ?? '',
      tags: (meta.tags as string[]) ?? [],
      platform: (meta.platform as StyleGuideMeta['platform']) ?? 'webapp',
    },
    body: body.trim(),
  };
}

/** Score how well a style guide matches the requested tags (Jaccard-like) */
function matchScore(guideTags: string[], requestTags: string[]): number {
  if (requestTags.length === 0) return 0;
  const intersection = requestTags.filter((t) => guideTags.includes(t));
  return intersection.length / requestTags.length;
}

/** Select best-matching style guide from registry */
export function selectStyleGuide(
  guides: ParsedStyleGuide[],
  options: { tags?: string[]; name?: string; platform?: string },
): ParsedStyleGuide | null {
  if (options.name) {
    if (options.platform) {
      // When platform is specified, only return a guide that matches both name AND platform
      return guides.find((g) => g.name === options.name && g.platform === options.platform) ?? null;
    }
    // No platform constraint — match by name only (MCP tool without platform param)
    return guides.find((g) => g.name === options.name) ?? null;
  }

  let candidates = guides;
  if (options.platform) {
    const platformFiltered = candidates.filter((g) => g.platform === options.platform);
    if (platformFiltered.length > 0) candidates = platformFiltered;
  }

  if (!options.tags || options.tags.length === 0) {
    // No tags and no name: return null to trigger fallback to style-defaults.md
    return null;
  }

  const scored = candidates
    .map((g) => ({ guide: g, score: matchScore(g.tags, options.tags!) }))
    .filter((s) => s.score > 0)
    .sort((a, b) => b.score - a.score);

  return scored[0]?.guide ?? null;
}

/** Parse a raw markdown string into a ParsedStyleGuide */
export function parseStyleGuideFile(raw: string): ParsedStyleGuide | null {
  const parsed = parseFrontmatter(raw);
  if (!parsed) return null;
  return { ...parsed.meta, content: raw };
}
