import type { Phase, ResolveOptions, AgentContext } from './types';
import { DEFAULT_BUDGETS } from './types';
import { getSkillsByPhase } from './loader';
import { filterByIntent, injectDynamicContent } from './resolver';
import { trimByBudget } from './budget';
import { getRecentEntries } from '../memory/generation-history';

export function resolveSkills(
  phase: Phase,
  userMessage: string,
  options?: ResolveOptions,
): AgentContext {
  const flags = options?.flags ?? {};
  const dynamicContent = options?.dynamicContent;
  const totalBudget = options?.budgetOverride ?? DEFAULT_BUDGETS[phase];

  // Step 1：Phase 过滤器
  const phaseSkills = getSkillsByPhase(phase);

  // Step 2：Intent + 标志匹配
  const matched = filterByIntent(phaseSkills, userMessage, flags);

  // Memory 每阶段加载规则（在 Step 3 之前移动，因此历史记录可用于注入）
  const memory: AgentContext['memory'] = {};
  if (options?.memory) {
    const { documentContext, generationHistory } = options.memory;
    const historyLimits: Record<Phase, number> = {
      planning: 5,
      generation: 3,
      validation: 0,
      maintenance: 3,
    };

    if (documentContext && phase !== 'validation') {
      memory.documentContext = documentContext;
    }

    const historyLimit = historyLimits[phase];
    if (generationHistory?.length && historyLimit > 0) {
      memory.generationHistory = getRecentEntries(
        generationHistory,
        historyLimit,
        options.documentPath,
      );
    }
  }

  // Build 合并动态内容：调用者提供 + recentHistory 用于防溢出
  let mergedDynamic = dynamicContent;
  if (phase === 'generation') {
    let historyStr = 'No recent history.';
    if (memory.generationHistory?.length) {
      historyStr = memory.generationHistory
        .map((h, i) => {
          const parts = [`Generation ${i + 1} (${h.timestamp}):`];
          if (h.output.headingFont) parts.push(`  Heading font: ${h.output.headingFont}`);
          if (h.output.palette) parts.push(`  Palette: ${h.output.palette}`);
          if (h.output.creativeVariant) parts.push(`  Variation: ${h.output.creativeVariant}`);
          return parts.join('\n');
        })
        .join('\n');
    }
    mergedDynamic = { ...dynamicContent, recentHistory: historyStr };
  }

  // Step 3: Dynamic content injection (all placeholders resolved in one pass)
  const injected = matched.map((skill) => ({
    ...skill,
    content: injectDynamicContent(skill.content, mergedDynamic),
  }));

  // Step 4: Budget trim (after injection, so token counts reflect actual content)
  const trimmed = trimByBudget(injected, totalBudget);
  const usedTokens = trimmed.reduce((sum, s) => sum + s.tokenCount, 0);

  return {
    role: 'general',
    phase,
    skills: trimmed,
    memory,
    budget: { used: usedTokens, max: totalBudget },
  };
}
