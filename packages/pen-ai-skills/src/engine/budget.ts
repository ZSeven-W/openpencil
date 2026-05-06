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
  // Step 1：Apply 每技能预算上限
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

  // Step 2：Always 保持基础技能
  const base = withTokens.filter((s) => s.meta.category === 'base');
  const domain = withTokens.filter((s) => s.meta.category === 'domain');
  const knowledge = withTokens.filter((s) => s.meta.category === 'knowledge');

  let usedTokens = base.reduce((sum, s) => sum + s.tokenCount, 0);
  const result: ResolvedSkill[] = [...base];

  // Step 3：Add 领域技能，如果需要则截断最后一个
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

  // Step 4：仅在预算充足的情况下才需要 Add 知识技能
  for (const skill of knowledge) {
    const remaining = totalBudget - usedTokens;
    if (remaining <= 0 || skill.tokenCount > remaining) break;
    result.push(skill);
    usedTokens += skill.tokenCount;
  }

  return result;
}
