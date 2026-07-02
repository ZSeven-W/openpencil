export type Phase = 'planning' | 'generation' | 'validation' | 'maintenance';

export type SkillTrigger = null | { keywords: string[] } | { flags: string[] };

export type SkillCategory = 'base' | 'domain' | 'knowledge';

export interface SkillMeta {
  name: string;
  description: string;
  phase: Phase[];
  trigger: SkillTrigger;
  priority: number;
  budget: number;
  category: SkillCategory;
}

export interface SkillRegistryEntry {
  meta: SkillMeta;
  content: string;
}

export interface ResolvedSkill {
  meta: SkillMeta;
  content: string;
  tokenCount: number;
  truncated: boolean;
}

export interface ResolveOptions {
  flags?: Record<string, boolean>;
  dynamicContent?: Record<string, string>;
  documentPath?: string;
  budgetOverride?: number;
  memory?: {
    documentContext?: DesignContext;
    generationHistory?: HistoryEntry[];
  };
}

export interface DesignContext {
  documentPath: string | null;
  createdAt: string;
  updatedAt: string;
  designSystem: {
    palette?: string[];
    typography?: string;
    spacing?: string;
    aesthetic?: string;
  };
  structure: {
    pageType?: string;
    sections?: string[];
    componentPatterns?: string[];
  };
  preferences: {
    overrides?: Array<{ what: string; from: string; to: string }>;
  };
}

export interface HistoryEntry {
  id: string;
  timestamp: string;
  documentPath: string;
  input: {
    prompt: string;
    phase: Phase;
    skillsUsed: string[];
  };
  output: {
    nodeCount: number;
    sectionTypes: string[];
    validationScore?: number;
    validationRounds?: number;
    headingFont?: string;
    palette?: string;
    creativeVariant?: string;
  };
  feedback?: 'accepted' | 'modified' | 'regenerated' | 'deleted';
}

export interface AgentContext {
  role: string;
  phase: Phase;
  skills: ResolvedSkill[];
  memory: {
    documentContext?: DesignContext;
    generationHistory?: HistoryEntry[];
  };
  budget: { used: number; max: number };
}

export const DEFAULT_BUDGETS: Record<Phase, number> = {
  planning: 4000,
  generation: 8000,
  validation: 3000,
  maintenance: 5000,
};
