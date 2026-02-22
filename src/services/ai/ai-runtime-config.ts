export type ThinkingMode = 'adaptive' | 'disabled' | 'enabled'
export type ThinkingEffort = 'low' | 'medium' | 'high' | 'max'

export const DEFAULT_THINKING_MODE: ThinkingMode = 'adaptive'
export const DEFAULT_THINKING_EFFORT: ThinkingEffort = 'low'

export const DEFAULT_THINKING_CONFIG = {
  thinkingMode: DEFAULT_THINKING_MODE,
  effort: DEFAULT_THINKING_EFFORT,
} as const

export const CHAT_STREAM_THINKING_CONFIG = {
  ...DEFAULT_THINKING_CONFIG,
} as const

export const STREAM_TIMEOUT_MIN_MS = 10_000
export const DEFAULT_STREAM_HARD_TIMEOUT_MS = 600_000
export const DEFAULT_STREAM_NO_TEXT_TIMEOUT_MS = 300_000
export const DEFAULT_GENERATE_TIMEOUT_MS = 180_000

export const PROMPT_OPTIMIZER_LIMITS = {
  longPromptCharThreshold: 2200,
  maxPromptCharsForOrchestrator: 2400,
  maxPromptCharsForSubAgent: 3600,
  maxFeatureLines: 12,
  maxSectionLines: 14,
  maxFallbackSections: 8,
} as const

export const SUB_AGENT_TIMEOUT_BASE = {
  hardTimeoutMs: 420_000,
  noTextTimeoutMs: 210_000,
  thinkingResetsTimeout: true,
} as const

export const PROMPT_TIMEOUT_BUCKETS = {
  mediumPromptMaxChars: 4200,
} as const

export const SUB_AGENT_TIMEOUT_PROFILES = {
  short: {
    ...SUB_AGENT_TIMEOUT_BASE,
    pingResetsTimeout: false,
    firstTextTimeoutMs: 420_000,
    thinkingMode: DEFAULT_THINKING_MODE,
    effort: DEFAULT_THINKING_EFFORT,
  },
  medium: {
    ...SUB_AGENT_TIMEOUT_BASE,
    hardTimeoutMs: 600_000,
    noTextTimeoutMs: 300_000,
    pingResetsTimeout: false,
    firstTextTimeoutMs: 600_000,
    thinkingMode: DEFAULT_THINKING_MODE,
    effort: DEFAULT_THINKING_EFFORT,
  },
  long: {
    ...SUB_AGENT_TIMEOUT_BASE,
    hardTimeoutMs: 900_000,
    noTextTimeoutMs: 480_000,
    pingResetsTimeout: false,
    firstTextTimeoutMs: 900_000,
    thinkingMode: DEFAULT_THINKING_MODE,
    effort: DEFAULT_THINKING_EFFORT,
  },
} as const

export const ORCHESTRATOR_TIMEOUT_PROFILES = {
  short: {
    hardTimeoutMs: 300_000,
    noTextTimeoutMs: 150_000,
    thinkingResetsTimeout: true,
    pingResetsTimeout: false,
    firstTextTimeoutMs: 300_000,
    thinkingMode: DEFAULT_THINKING_MODE,
    effort: DEFAULT_THINKING_EFFORT,
  },
  medium: {
    hardTimeoutMs: 420_000,
    noTextTimeoutMs: 210_000,
    thinkingResetsTimeout: true,
    pingResetsTimeout: false,
    firstTextTimeoutMs: 420_000,
    thinkingMode: DEFAULT_THINKING_MODE,
    effort: DEFAULT_THINKING_EFFORT,
  },
  long: {
    hardTimeoutMs: 600_000,
    noTextTimeoutMs: 300_000,
    thinkingResetsTimeout: true,
    pingResetsTimeout: false,
    firstTextTimeoutMs: 600_000,
    thinkingMode: DEFAULT_THINKING_MODE,
    effort: DEFAULT_THINKING_EFFORT,
  },
} as const

export const DESIGN_STREAM_TIMEOUTS = {
  hardTimeoutMs: 900_000,
  noTextTimeoutMs: 480_000,
  thinkingResetsTimeout: true,
  pingResetsTimeout: false,
  firstTextTimeoutMs: 900_000,
  thinkingMode: DEFAULT_THINKING_MODE,
  effort: DEFAULT_THINKING_EFFORT,
} as const

export const RETRY_TIMEOUT_CONFIG = {
  multiplier: 2,
  hardTimeoutMaxMs: 1_200_000,
  noTextTimeoutMaxMs: 480_000,
  firstTextTimeoutMaxMs: 1_200_000,
} as const
