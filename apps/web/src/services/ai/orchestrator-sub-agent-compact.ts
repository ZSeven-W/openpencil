import { extractStyleGuideValues } from '@zseven-w/pen-ai-skills/style-guide';
import { styleGuideRegistry } from '@zseven-w/pen-ai-skills/_generated/style-guide-registry';

export function compactSubAgentSkills<T extends { meta: { name: string }; content: string }>(
  skills: T[],
  tier: 'full' | 'standard' | 'basic',
  isMobileScreen: boolean,
  hasExplicitStyleGuide: boolean,
  reducedComplexity = false,
): T[] {
  const filtered = skills.filter((skill) => {
    const name = skill.meta.name;
    if (
      isMobileScreen &&
      (name === 'landing-page' || name === 'copywriting' || name === 'anti-slop')
    ) {
      return false;
    }
    if (!isMobileScreen && name === 'mobile-app') return false;
    if (hasExplicitStyleGuide && name === 'design-system') return false;
    return true;
  });

  const hasSimplified = filtered.some((skill) => skill.meta.name === 'jsonl-format-simplified');
  let next = filtered;
  if (hasSimplified) {
    next = next.filter((skill) => skill.meta.name !== 'jsonl-format');
  }

  if (tier === 'basic') {
    const allowed = new Set([
      'schema',
      'jsonl-format-simplified',
      'jsonl-format',
      'layout',
      'overflow',
      'text-rules',
      'variables',
      'design-md',
      'mobile-app',
      'icon-catalog',
      'style-defaults',
      // N-tool element reference — gated by `hasMcpTools` at the
      // resolveSkills layer (see orchestrator-sub-agent.ts and the
      // elements.md frontmatter). When the flag is off, the skill
      // isn't in `skills` to begin with, so keeping it in the
      // allow-list is a no-op. When the flag is on, we MUST allow
      // it through or the basic-tier filter below silently drops
      // the element-tool docs and the feature can't fire.
      'elements',
    ]);
    next = next.filter((skill) => allowed.has(skill.meta.name));
    if (reducedComplexity) {
      const retryAllowed = new Set([
        'schema',
        'jsonl-format-simplified',
        'layout',
        'text-rules',
        'mobile-app',
        'style-defaults',
        'design-md',
        'variables',
        // elements.md deliberately OMITTED from the retry-allowed
        // set. Reduced-complexity retries are the last-ditch
        // fallback for weak models that already failed once; the
        // elements skill adds ~17k chars to the prompt and the
        // retry path wants the smallest possible kernel. When the
        // first attempt fails in treatment mode, we drop back to
        // legacy batch_design for the retry.
      ]);
      next = next.filter((skill) => retryAllowed.has(skill.meta.name));
    }
  }

  return next;
}

export function buildSubAgentStyleGuideInstruction(
  content: string,
  styleGuideName: string | undefined,
  tier: 'full' | 'standard' | 'basic',
): string {
  const values = extractStyleGuideValues(content);
  const tags = styleGuideName
    ? styleGuideRegistry
        .find((guide) => guide.name === styleGuideName)
        ?.tags.slice(0, 6)
        .join(', ')
    : undefined;

  // The palette below is SEEDED into doc.variables when the sub-agent runs (see
  // seedDocVariablesFromStyleGuide in orchestrator.ts), so emitting `$color-*`
  // refs in JSONL fills is both correct AND visually accurate — refs resolve to
  // the hex shown in parens.
  const refLine = (label: string, ref: string, hex: string | undefined) =>
    hex ? `- ${label}: \`${ref}\` (resolves to ${hex})` : null;

  const refLines = [
    refLine('Background', '$color-bg-deep', values.colors.background),
    refLine('Surface', '$color-surface', values.colors.surface),
    refLine('Accent', '$color-accent', values.colors.accent),
    refLine('Text', '$color-text-primary', values.colors.textPrimary),
    refLine('Secondary text', '$color-text-body', values.colors.textSecondary),
    refLine('Muted text', '$color-text-muted', values.colors.textMuted),
    refLine('Border', '$color-border', values.colors.border),
  ].filter(Boolean);

  if (tier === 'full') {
    return `VISUAL STYLE GUIDE (follow these specifications exactly):\n${content}\n\nDESIGN-SYSTEM REFS — these are SEEDED into doc.variables. Emit \`$color-*\` refs in JSONL fills (NOT hex) so the design respects the user's tokens at render time:\n${refLines.join('\n')}`;
  }

  const lines = [
    `VISUAL STYLE GUIDE SUMMARY${styleGuideName ? ` (${styleGuideName})` : ''}:`,
    tags ? `- Tags: ${tags}` : null,
    ...refLines,
    values.typography.displayFont ? `- Heading font: ${values.typography.displayFont}` : null,
    values.typography.bodyFont ? `- Body font: ${values.typography.bodyFont}` : null,
    values.radius.card != null ? `- Card radius: ${values.radius.card}` : null,
    values.radius.button != null ? `- Button radius: ${values.radius.button}` : null,
    '- These `$color-*` refs are SEEDED into doc.variables. EMIT REFS in your JSONL fills (NOT hex) — they resolve to the hex above at render time. Do not invent a conflicting palette.',
  ];

  return lines.filter(Boolean).join('\n');
}
