// Engine
export type {
  Phase,
  SkillTrigger,
  SkillMeta,
  ResolvedSkill,
  ResolveOptions,
  AgentContext,
} from './engine/types';

export { resolveSkills } from './engine/resolve-skills';
export { getSkillRegistry, getSkillByName, getSkillsByPhase } from './engine/loader';

// Memory
export type { DesignContext } from './memory/document-context';
export type { HistoryEntry } from './memory/generation-history';

export {
  createDesignContext,
  extractDesignContext,
  mergePreference,
  contextToPromptString,
} from './memory/document-context';

export {
  createHistoryEntry,
  updateFeedback,
  getRecentEntries,
  historyToPromptString,
} from './memory/generation-history';

// Diagnostics (pure detectors for design quality issues)
export * from './diagnostics';
