import type { SkillRegistryEntry, ResolvedSkill } from './types';

export function estimateTokens(text: string): number {
  return Math.ceil(text.length / 4);
}

function truncateContent(content: string, maxTokens: number): string {
  const maxChars = maxTokens * 4;
  if (content.length <= maxChars) return content;
  const truncated = content.slice(0, maxChars);
  const lastNewline = truncated.lastIndexOf('\n');
  return lastNewline > maxChars * 0.5 ? truncated.slice(0, lastNewline) : truncated;
}

export function trimByBudget(skills: SkillRegistryEntry[], totalBudget: number): ResolvedSkill[] {
  // Step 1: Apply per-skill budget caps
  const withTokens = skills.map((skill) => {
    const perSkillBudget = skill.meta.budget;
    const rawTokens = estimateTokens(skill.content);
    const needsTruncate = rawTokens > perSkillBudget;
    const content = needsTruncate ? truncateContent(skill.content, perSkillBudget) : skill.content;
    return {
      meta: skill.meta,
      content,
      tokenCount: needsTruncate ? estimateTokens(content) : rawTokens,
      truncated: needsTruncate,
    };
  });

  // Step 2: Always keep base skills
  const base = withTokens.filter((s) => s.meta.category === 'base');
  const domain = withTokens.filter((s) => s.meta.category === 'domain');
  const knowledge = withTokens.filter((s) => s.meta.category === 'knowledge');

  let usedTokens = base.reduce((sum, s) => sum + s.tokenCount, 0);
  const result: ResolvedSkill[] = [...base];

  // Step 3: Add domain skills, truncating last if needed
  for (const skill of domain) {
    const remaining = totalBudget - usedTokens;
    if (remaining <= 0) break;
    if (skill.tokenCount <= remaining) {
      result.push(skill);
      usedTokens += skill.tokenCount;
    } else {
      const truncatedContent = truncateContent(skill.content, remaining);
      result.push({
        ...skill,
        content: truncatedContent,
        tokenCount: estimateTokens(truncatedContent),
        truncated: true,
      });
      usedTokens += estimateTokens(truncatedContent);
      break;
    }
  }

  // Step 4: Add knowledge skills only if budget remains
  for (const skill of knowledge) {
    const remaining = totalBudget - usedTokens;
    if (remaining <= 0 || skill.tokenCount > remaining) break;
    result.push(skill);
    usedTokens += skill.tokenCount;
  }

  return result;
}
