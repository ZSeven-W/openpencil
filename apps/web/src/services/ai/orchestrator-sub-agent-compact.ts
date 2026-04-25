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
  if (tier === 'full') {
    return `VISUAL STYLE GUIDE (follow these specifications exactly):\n${content}`;
  }

  const values = extractStyleGuideValues(content);
  const tags = styleGuideName
    ? styleGuideRegistry
        .find((guide) => guide.name === styleGuideName)
        ?.tags.slice(0, 6)
        .join(', ')
    : undefined;

  const lines = [
    `VISUAL STYLE GUIDE SUMMARY${styleGuideName ? ` (${styleGuideName})` : ''}:`,
    tags ? `- Tags: ${tags}` : null,
    values.colors.background ? `- Background: ${values.colors.background}` : null,
    values.colors.surface ? `- Surface: ${values.colors.surface}` : null,
    values.colors.accent ? `- Accent: ${values.colors.accent}` : null,
    values.colors.textPrimary ? `- Text: ${values.colors.textPrimary}` : null,
    values.colors.textSecondary ? `- Secondary text: ${values.colors.textSecondary}` : null,
    values.typography.displayFont ? `- Heading font: ${values.typography.displayFont}` : null,
    values.typography.bodyFont ? `- Body font: ${values.typography.bodyFont}` : null,
    values.radius.card != null ? `- Card radius: ${values.radius.card}` : null,
    values.radius.button != null ? `- Button radius: ${values.radius.button}` : null,
    '- Match the selected guide using these tokens; do not invent a conflicting palette.',
  ];

  return lines.filter(Boolean).join('\n');
}
