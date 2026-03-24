import type { Phase, ResolveOptions, AgentContext } from './types'
import { DEFAULT_BUDGETS } from './types'
import { getSkillsByPhase } from './loader'
import { filterByIntent, injectDynamicContent } from './resolver'
import { trimByBudget } from './budget'
import { getRecentEntries } from '../memory/generation-history'

export function resolveSkills(
  phase: Phase,
  userMessage: string,
  options?: ResolveOptions
): AgentContext {
  const flags = options?.flags ?? {}
  const dynamicContent = options?.dynamicContent
  const totalBudget = options?.budgetOverride ?? DEFAULT_BUDGETS[phase]

  // Step 1: Phase filter
  const phaseSkills = getSkillsByPhase(phase)

  // Step 2: Intent + flag match
  const matched = filterByIntent(phaseSkills, userMessage, flags)

  // Step 3: Dynamic content injection
  const injected = matched.map(skill => ({
    ...skill,
    content: injectDynamicContent(skill.content, dynamicContent),
  }))

  // Step 4: Budget trim
  const trimmed = trimByBudget(injected, totalBudget)
  const usedTokens = trimmed.reduce((sum, s) => sum + s.tokenCount, 0)

  // Memory loading rules per phase
  const memory: AgentContext['memory'] = {}
  if (options?.memory) {
    const { documentContext, generationHistory } = options.memory
    const historyLimits: Record<Phase, number> = {
      planning: 5,
      generation: 0,
      validation: 0,
      maintenance: 3,
    }

    if (documentContext && phase !== 'validation') {
      memory.documentContext = documentContext
    }

    const historyLimit = historyLimits[phase]
    if (generationHistory?.length && historyLimit > 0) {
      memory.generationHistory = getRecentEntries(
        generationHistory,
        historyLimit,
        options.documentPath
      )
    }
  }

  return {
    role: 'general',
    phase,
    skills: trimmed,
    memory,
    budget: { used: usedTokens, max: totalBudget },
  }
}
