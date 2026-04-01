import type { OrchestratorPlan } from './ai-types';
import {
  ORCHESTRATOR_TIMEOUT_PROFILES,
  PROMPT_TIMEOUT_BUCKETS,
  PROMPT_OPTIMIZER_LIMITS,
  SUB_AGENT_TIMEOUT_PROFILES,
} from './ai-runtime-config';
import { detectDesignType } from './design-type-presets';
import { getSkillByName } from '@zseven-w/pen-ai-skills';
import { selectStyleGuide } from '@zseven-w/pen-ai-skills/style-guide';
import { styleGuideRegistry } from '@zseven-w/pen-ai-skills/_generated/style-guide-registry';
import { resolveModelProfile, applyProfileToTimeouts } from './model-profiles';

export interface PreparedDesignPrompt {
  original: string;
  orchestratorPrompt: string;
  subAgentPrompt: string;
  wasCompressed: boolean;
  originalLength: number;
  /** Selectively loaded design principles for sub-agent context */
  designPrinciples: string;
}

export function getSubAgentTimeouts(
  promptLength: number,
  model?: string,
): {
  hardTimeoutMs: number;
  noTextTimeoutMs: number;
  thinkingResetsTimeout: boolean;
  pingResetsTimeout: boolean;
  firstTextTimeoutMs: number;
  thinkingMode: 'adaptive' | 'disabled' | 'enabled';
  effort: 'low' | 'medium' | 'high' | 'max';
} {
  let base;
  if (promptLength < PROMPT_OPTIMIZER_LIMITS.longPromptCharThreshold) {
    base = { ...SUB_AGENT_TIMEOUT_PROFILES.short };
  } else if (promptLength < PROMPT_TIMEOUT_BUCKETS.mediumPromptMaxChars) {
    base = { ...SUB_AGENT_TIMEOUT_PROFILES.medium };
  } else {
    base = { ...SUB_AGENT_TIMEOUT_PROFILES.long };
  }
  return applyProfileToTimeouts(base, resolveModelProfile(model));
}

export function getOrchestratorTimeouts(
  promptLength: number,
  model?: string,
): {
  hardTimeoutMs: number;
  noTextTimeoutMs: number;
  thinkingResetsTimeout: boolean;
  pingResetsTimeout: boolean;
  firstTextTimeoutMs: number;
  thinkingMode: 'adaptive' | 'disabled' | 'enabled';
  effort: 'low' | 'medium' | 'high' | 'max';
} {
  let base;
  if (promptLength < PROMPT_OPTIMIZER_LIMITS.longPromptCharThreshold) {
    base = { ...ORCHESTRATOR_TIMEOUT_PROFILES.short };
  } else if (promptLength < PROMPT_TIMEOUT_BUCKETS.mediumPromptMaxChars) {
    base = { ...ORCHESTRATOR_TIMEOUT_PROFILES.medium };
  } else {
    base = { ...ORCHESTRATOR_TIMEOUT_PROFILES.long };
  }
  return applyProfileToTimeouts(base, resolveModelProfile(model));
}

/**
 * Prepare a user prompt for the orchestrator and sub-agents.
 * Simply normalizes whitespace and truncates if too long.
 * No lossy "intelligent" extraction — the user's original intent is preserved.
 */
export function prepareDesignPrompt(prompt: string): PreparedDesignPrompt {
  const normalized = normalizePromptText(prompt);

  return {
    original: prompt,
    orchestratorPrompt: truncateByCharCount(
      normalized,
      PROMPT_OPTIMIZER_LIMITS.maxPromptCharsForOrchestrator,
    ),
    subAgentPrompt: truncateByCharCount(
      normalized,
      PROMPT_OPTIMIZER_LIMITS.maxPromptCharsForSubAgent,
    ),
    wasCompressed: normalized.length > PROMPT_OPTIMIZER_LIMITS.maxPromptCharsForOrchestrator,
    originalLength: normalized.length,
    designPrinciples: getSkillByName('design-principles')?.content ?? '',
  };
}

export function buildFallbackPlanFromPrompt(prompt: string): OrchestratorPlan {
  const preset = detectDesignType(prompt);

  // Try to select a style guide based on prompt keywords
  const platform = preset.width <= 500 ? 'mobile' : 'webapp';
  const tags = inferTagsFromPrompt(prompt);
  const guide = selectStyleGuide(styleGuideRegistry, { tags, platform });

  // Extract background color from selected guide, or use default
  let bgColor = '#F8FAFC';
  if (guide) {
    const bgMatch = guide.content.match(/(#[0-9A-Fa-f]{6})\s*[—–-]\s*(?:Page )?Background/i)
      ?? guide.content.match(/Background[^#]*(#[0-9A-Fa-f]{6})/i);
    if (bgMatch) bgColor = bgMatch[1];
  }

  // Use preset's default sections — don't parse bullet points from prompt
  // (bullet parsing caused duplicate elements like triple status bars)
  const labels = preset.defaultSections;

  const sectionCount = Math.max(1, labels.length);

  // Mobile: split height evenly (no weighted allocation — sub-agent decides actual proportions)
  // Desktop: use standard weighted allocation
  let heights: number[];
  if (preset.type === 'mobile-screen') {
    const perSection = Math.floor(preset.height / sectionCount);
    heights = labels.map(() => perSection);
  } else {
    const totalHeight = preset.height || (sectionCount >= 4 ? 4000 : 800);
    heights = allocateSectionHeights(totalHeight, sectionCount);
  }

  const plan: OrchestratorPlan = {
    rootFrame: {
      id: 'page',
      name: 'Page',
      width: preset.width,
      height: preset.rootHeight || 0,
      layout: 'vertical',
      fill: [{ type: 'solid', color: bgColor }],
    },
    subtasks: labels.map((label, index) => ({
      id: makeSafeSectionId(label, index),
      label,
      region: { width: preset.width, height: heights[index] ?? 120 },
      idPrefix: '',
      parentFrameId: null,
    })),
  };

  // Attach selected style guide for downstream injection
  if (guide) {
    plan.styleGuideName = guide.name;
    plan.selectedStyleGuideContent = guide.content;
  }

  return plan;
}

/** Infer style guide tags from user prompt keywords */
function inferTagsFromPrompt(prompt: string): string[] {
  const tags: string[] = [];
  const lower = prompt.toLowerCase();

  // tone
  if (/dark|暗[色黑]?|cyber|terminal|neon/.test(lower)) tags.push('dark-mode');
  else tags.push('light-mode');

  // visual
  if (/minimal|极简|clean|简洁/.test(lower)) tags.push('minimal');
  if (/brutal|粗犷/.test(lower)) tags.push('brutalist');
  if (/elegant|优雅|luxury|奢华/.test(lower)) tags.push('elegant');
  if (/playful|活泼|fun|趣味/.test(lower)) tags.push('playful');
  if (/modern|现代/.test(lower)) tags.push('modern');

  // industry
  if (/food|餐|美食|delivery|外卖/.test(lower)) tags.push('warm-tones', 'friendly');
  if (/finance|金融|fintech/.test(lower)) tags.push('fintech');
  if (/developer|开发|code|terminal/.test(lower)) tags.push('developer', 'monospace');
  if (/wellness|健康|health/.test(lower)) tags.push('wellness');

  // accent
  if (/coral|珊瑚|orange|橙/.test(lower)) tags.push('orange-accent');
  if (/blue|蓝/.test(lower)) tags.push('blue-accent');
  if (/green|绿/.test(lower)) tags.push('sage-green');
  if (/gold|金/.test(lower)) tags.push('gold-accent');
  if (/red|红/.test(lower)) tags.push('red-accent');

  // technique
  if (/rounded|圆角/.test(lower)) tags.push('rounded');
  if (/gradient|渐变/.test(lower)) tags.push('gradient');

  return tags.length > 0 ? tags : ['minimal', 'light-mode'];
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

function normalizePromptText(text: string): string {
  return text
    .replace(/\r/g, '')
    .replace(/[ \t]+\n/g, '\n')
    .replace(/\n{3,}/g, '\n\n')
    .trim();
}

function truncateByCharCount(text: string, maxChars: number): string {
  if (text.length <= maxChars) return text;
  const truncated = text.slice(0, maxChars);
  const lastBoundary = Math.max(
    truncated.lastIndexOf('\n'),
    truncated.lastIndexOf('。'),
    truncated.lastIndexOf('.'),
  );
  if (lastBoundary > Math.floor(maxChars * 0.7)) {
    return `${truncated.slice(0, lastBoundary).trim()}\n\n[truncated]`;
  }
  return `${truncated.trim()}\n\n[truncated]`;
}

function makeSafeSectionId(label: string, index: number): string {
  const ascii = label
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
  if (ascii.length > 0) return ascii;
  return `section-${index + 1}`;
}

function allocateSectionHeights(totalHeight: number, count: number): number[] {
  if (count <= 0) return [];
  if (count === 1) return [totalHeight];

  const minHeight = 80;
  // Weighted allocation: first section (hero/header) gets 1.4×, last (footer) gets 0.6×, rest even
  const weights = Array.from({ length: count }, (_, i) => {
    if (i === 0) return 1.4; // hero/header
    if (i === count - 1 && count >= 3) return 0.6; // footer
    return 1.0;
  });
  const totalWeight = weights.reduce((sum, w) => sum + w, 0);
  const heights = weights.map((w) =>
    Math.max(minHeight, Math.round((totalHeight * w) / totalWeight)),
  );

  // Adjust to match total exactly
  let allocated = heights.reduce((sum, h) => sum + h, 0);
  let idx = Math.floor(count / 2); // adjust middle sections first
  while (allocated < totalHeight) {
    heights[idx] += 1;
    allocated += 1;
    idx = (idx + 1) % count;
  }
  idx = count - 1;
  while (allocated > totalHeight) {
    if (heights[idx] > minHeight) {
      heights[idx] -= 1;
      allocated -= 1;
    }
    idx = idx - 1;
    if (idx < 0) idx = count - 1;
  }

  return heights;
}
