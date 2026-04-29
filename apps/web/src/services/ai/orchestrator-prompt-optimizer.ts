import type { OrchestratorPlan } from './ai-types';
import type { DesignMdSpec } from '@/types/design-md';
import {
  ORCHESTRATOR_TIMEOUT_PROFILES,
  PROMPT_TIMEOUT_BUCKETS,
  PROMPT_OPTIMIZER_LIMITS,
  SUB_AGENT_TIMEOUT_PROFILES,
} from './ai-runtime-config';
import { detectDesignType } from './design-type-presets';
import { getSkillByName } from '@zseven-w/pen-ai-skills';
import { extractStyleGuideValues, selectStyleGuide } from '@zseven-w/pen-ai-skills/style-guide';
import { styleGuideRegistry } from '@zseven-w/pen-ai-skills/_generated/style-guide-registry';
import { resolveModelProfile, applyProfileToTimeouts, type ModelTier } from './model-profiles';
import { buildDesignMdStylePolicy } from './ai-prompts';

export const DESIGN_MD_STYLE_GUIDE_NAME = 'design-md-custom';

export interface PreparedDesignPrompt {
  original: string;
  orchestratorPrompt: string;
  subAgentPrompt: string;
  wasCompressed: boolean;
  originalLength: number;
  /** Selectively loaded design principles for sub-agent context */
  designPrinciples: string;
}

type StreamTimeoutProfile = {
  hardTimeoutMs: number;
  noTextTimeoutMs: number;
  thinkingResetsTimeout: boolean;
  pingResetsTimeout: boolean;
  firstTextTimeoutMs: number;
  thinkingMode: 'adaptive' | 'disabled' | 'enabled';
  effort: 'low' | 'medium' | 'high' | 'max';
};

export interface CompactPlanningPrompt {
  systemPrompt: string;
  userPrompt: string;
  selectedStyleGuideName?: string;
}

export function getSubAgentTimeouts(promptLength: number, model?: string): StreamTimeoutProfile {
  const profile = resolveModelProfile(model);
  let base: StreamTimeoutProfile;
  if (promptLength < PROMPT_OPTIMIZER_LIMITS.longPromptCharThreshold) {
    base = { ...SUB_AGENT_TIMEOUT_PROFILES.short };
  } else if (promptLength < PROMPT_TIMEOUT_BUCKETS.mediumPromptMaxChars) {
    base = { ...SUB_AGENT_TIMEOUT_PROFILES.medium };
  } else {
    base = { ...SUB_AGENT_TIMEOUT_PROFILES.long };
  }
  const timeouts = applyProfileToTimeouts(base, profile);

  // Basic models are much more likely to stall after emitting a small amount
  // of reasoning or while the server only sends keepalive pings. Fail faster.
  if (profile.tier === 'basic') {
    timeouts.pingResetsTimeout = false;
    timeouts.noTextTimeoutMs = Math.min(timeouts.noTextTimeoutMs, 45_000);
    timeouts.firstTextTimeoutMs = Math.min(timeouts.firstTextTimeoutMs, 75_000);
  }

  return timeouts;
}

export function getOrchestratorTimeouts(
  promptLength: number,
  model?: string,
): StreamTimeoutProfile {
  let base: StreamTimeoutProfile;
  if (promptLength < PROMPT_OPTIMIZER_LIMITS.longPromptCharThreshold) {
    base = { ...ORCHESTRATOR_TIMEOUT_PROFILES.short };
  } else if (promptLength < PROMPT_TIMEOUT_BUCKETS.mediumPromptMaxChars) {
    base = { ...ORCHESTRATOR_TIMEOUT_PROFILES.medium };
  } else {
    base = { ...ORCHESTRATOR_TIMEOUT_PROFILES.long };
  }
  return applyProfileToTimeouts(base, resolveModelProfile(model));
}

export function getBuiltinPlanningTimeouts(model?: string): StreamTimeoutProfile {
  const profile = resolveModelProfile(model);
  const base: StreamTimeoutProfile = {
    hardTimeoutMs: 60_000,
    noTextTimeoutMs: 30_000,
    thinkingResetsTimeout: true,
    pingResetsTimeout: false,
    firstTextTimeoutMs: 30_000,
    thinkingMode: 'adaptive',
    effort: 'low',
  };
  const timeouts = applyProfileToTimeouts(base, profile);

  if (profile.tier === 'basic') {
    timeouts.hardTimeoutMs = Math.max(timeouts.hardTimeoutMs, 150_000);
    timeouts.noTextTimeoutMs = Math.max(timeouts.noTextTimeoutMs, 75_000);
    timeouts.firstTextTimeoutMs = Math.max(timeouts.firstTextTimeoutMs ?? 0, 75_000);
    timeouts.thinkingMode = 'disabled';
  }

  return timeouts;
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

export function buildFallbackPlanFromPrompt(
  prompt: string,
  designMd?: DesignMdSpec,
): OrchestratorPlan {
  const preset = detectDesignType(prompt);

  // If the user has a design.md, seed the fallback plan from it so basic-tier
  // models that fall through to this path still honor the user's custom theme.
  const designMdBg = designMd ? inferDesignMdBackground(designMd) : null;

  // Try to select a style guide based on prompt keywords (only when no design.md)
  const platform = preset.width <= 500 ? 'mobile' : 'webapp';
  const tags = inferTagsFromPrompt(prompt);
  const guide = designMd ? null : selectStyleGuide(styleGuideRegistry, { tags, platform });

  // Extract background color from selected guide, or use default.
  // When design.md is present but has no palette entry tagged as background,
  // fall back to a neutral color derived from visualTheme rather than the
  // catalog — picking any palette color here risks painting the page with a
  // brand/CTA/accent color.
  let bgColor = designMd ? guessNeutralBackgroundFromTheme(designMd.visualTheme) : '#FFFFFF';
  if (designMdBg) {
    bgColor = designMdBg;
  } else if (guide) {
    const bgMatch =
      guide.content.match(/(#[0-9A-Fa-f]{6})\s*[—–-]\s*(?:Page )?Background/i) ??
      guide.content.match(/Background[^#]*(#[0-9A-Fa-f]{6})/i);
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
      elements: getMobileSectionElements(preset.type, label),
      region: { width: preset.width, height: heights[index] ?? 120 },
      idPrefix: '',
      parentFrameId: null,
    })),
  };

  // Attach selected style guide for downstream injection.
  // When design.md is present, we intentionally skip catalog content so it
  // doesn't override the user's custom design system downstream.
  if (designMd) {
    plan.styleGuideName = DESIGN_MD_STYLE_GUIDE_NAME;
  } else if (guide) {
    plan.styleGuideName = guide.name;
    plan.selectedStyleGuideContent = guide.content;
  }

  return plan;
}

/** Assign distinct element hints to each mobile section to prevent duplicate content. */
function getMobileSectionElements(type: string, label: string): string | undefined {
  if (type !== 'mobile-screen') return undefined;
  switch (label) {
    case 'Top Summary':
      return 'Top-of-screen summary only: greeting/title/avatar/hero metric strip. A status bar is already pre-inserted — do NOT generate one. Do NOT include charts, long lists, workout cards, or bottom navigation here.';
    case 'Main Content':
      return 'All remaining main UI content for this screen: cards, charts, lists, forms, actions, and bottom navigation if requested. Do NOT repeat the top greeting/title/avatar summary block here.';
    default:
      return undefined;
  }
}

type PlanningContextMode = 'rich' | 'minimal';

const STYLE_GUIDE_METADATA_TAG_LIMIT = 4;
const STYLE_GUIDE_SNIPPET_LIMITS: Record<ModelTier, number> = {
  basic: 4,
  standard: 6,
  full: 8,
};

export interface PlanningStyleGuideContext {
  availableStyleGuides: string;
  metadataCount: number;
  snippetCount: number;
  topGuideNames: string[];
  snippetGuideNames: string[];
}

export function buildPlanningStyleGuideContext(
  prompt: string,
  model?: string,
  mode: PlanningContextMode = 'rich',
  designMd?: DesignMdSpec,
): PlanningStyleGuideContext {
  if (designMd) {
    const policy = buildDesignMdStylePolicy(designMd);
    const bgHint = inferDesignMdBackground(designMd);
    // When design.md has no palette entry explicitly marked as background/
    // surface/canvas, do NOT ask the model to "pick" from the palette — it will
    // happily pick a brand/CTA color and paint the whole page that color. Give
    // it a neutral default instead, biased by visualTheme keywords.
    const neutralDefault = guessNeutralBackgroundFromTheme(designMd.visualTheme);
    const lines = [
      `The user has a custom design system (design.md). DO NOT pick a style guide from a catalog.`,
      `Use the rules below for all style decisions:`,
      '',
      policy || '(design.md is present but has no extractable policy; use project defaults)',
      '',
      `Output directives:`,
      `- Set "styleGuideName": "${DESIGN_MD_STYLE_GUIDE_NAME}" (exact string).`,
      bgHint
        ? `- Set rootFrame.fill color to "${bgHint}" (the primary background color from the design.md palette).`
        : `- Set rootFrame.fill color to "${neutralDefault}" (neutral page background — design.md has no palette entry tagged as background, so DO NOT pick a brand/CTA/accent/text color from the palette for the page background).`,
    ];
    return {
      availableStyleGuides: lines.join('\n'),
      metadataCount: 0,
      snippetCount: 0,
      topGuideNames: [DESIGN_MD_STYLE_GUIDE_NAME],
      snippetGuideNames: [],
    };
  }

  const preset = detectDesignType(prompt);
  const platform = preset.width <= 500 ? 'mobile' : 'webapp';
  const tags = inferTagsFromPrompt(prompt);
  const tier = resolveModelProfile(model).tier;
  const ranked = rankStyleGuidesForPrompt(tags, platform);

  const metadataLines = ranked.map((guide) => formatGuideMetadataLine(guide, mode));
  const snippetLimit = mode === 'rich' ? STYLE_GUIDE_SNIPPET_LIMITS[tier] : 0;
  const snippetGuides = ranked.slice(0, snippetLimit);

  const parts = [
    'Available style guides (compact catalog; all candidates are listed below):',
    ...metadataLines,
  ];

  if (snippetGuides.length > 0) {
    parts.push(
      '',
      'Detailed references for the best-matching candidates (prefer these before inventing a styleGuideName):',
      ...snippetGuides.map((guide) => formatGuideSnippet(guide)),
    );
  }

  return {
    availableStyleGuides: parts.join('\n'),
    metadataCount: metadataLines.length,
    snippetCount: snippetGuides.length,
    topGuideNames: ranked.slice(0, 12).map((guide) => guide.name),
    snippetGuideNames: snippetGuides.map((guide) => guide.name),
  };
}

/**
 * Pick the color in design.md most likely to be the root/app background.
 * Returns null when no palette entry has a role that clearly marks it as a
 * background — guessing at the first palette color is dangerous because it is
 * often a brand/accent color, which would turn the entire page that color.
 */
/**
 * Pick a safe neutral page background when design.md lacks an explicit
 * background role. Biases toward dark when the visual theme hints at dark mode,
 * otherwise defaults to a light page color.
 */
export function guessNeutralBackgroundFromTheme(theme?: string): string {
  if (!theme) return '#FFFFFF';
  if (/\b(dark|night|noir|cyber|neon|terminal|midnight|obsidian|onyx)\b/i.test(theme)) {
    return '#111111';
  }
  return '#FFFFFF';
}

export function inferDesignMdBackground(spec: DesignMdSpec): string | null {
  const palette = spec.colorPalette;
  if (!palette?.length) return null;
  const scoreRole = (role: string): number => {
    const r = role.toLowerCase();
    // A surface/card role is NOT a page background — e.g. dark-mode palettes
    // often use #0A0F1C for the page and #1A1F2E for cards, and painting the
    // whole page with the card color produces a flat result that hides the
    // card boundaries. Only accept roles that explicitly describe a page/app
    // background or the root canvas.
    if (/primary.*background|app background|main background|page background|canvas/.test(r)) {
      return 3;
    }
    if (/\bbackground\b/.test(r) && !/surface|card|tile|chip|panel/.test(r)) return 2;
    return 0;
  };
  let best: (typeof palette)[number] | null = null;
  let bestScore = 0;
  for (const c of palette) {
    const s = scoreRole(c.role || '');
    if (s > bestScore) {
      best = c;
      bestScore = s;
    }
  }
  return best ? best.hex : null;
}

export function buildCompactPlanningPrompt(
  prompt: string,
  _model?: string,
  designMd?: DesignMdSpec,
): CompactPlanningPrompt {
  const preset = detectDesignType(prompt);
  const platform = preset.width <= 500 ? 'mobile' : 'webapp';
  const tags = inferTagsFromPrompt(prompt);
  const designMdBg = designMd ? inferDesignMdBackground(designMd) : null;
  const selectedGuide = designMd ? null : selectStyleGuide(styleGuideRegistry, { tags, platform });
  const guideValues = selectedGuide ? extractStyleGuideValues(selectedGuide.content) : null;
  const backgroundColor =
    designMdBg ??
    (designMd ? guessNeutralBackgroundFromTheme(designMd.visualTheme) : null) ??
    guideValues?.colors.background ??
    (preset.type === 'mobile-screen' ? '#111827' : '#F8FAFC');
  const defaultGap = preset.type === 'mobile-screen' || preset.type === 'desktop-screen' ? 20 : 0;
  const subtaskHint =
    preset.type === 'mobile-screen'
      ? 'Create 2-4 cohesive subtasks for one mobile app screen. Group related UI together.'
      : preset.type === 'desktop-screen'
        ? 'Create 2-5 cohesive workspace sections. Keep related dashboard panels together.'
        : 'Create 4-8 scrollable page sections in top-to-bottom order.';
  const mobileRules =
    preset.type === 'mobile-screen'
      ? [
          'This is a direct mobile screen, not a phone mockup.',
          'Do NOT create a status bar section. The status bar is inserted separately.',
          'Use width=375 and height=812 on the root frame.',
        ]
      : ['Use width=1200 and height=0 on the root frame.'];
  const styleRule = designMd
    ? `Use styleGuideName="${DESIGN_MD_STYLE_GUIDE_NAME}" and rootFrame background ${backgroundColor} (from the user's design.md — overrides any catalog default).`
    : selectedGuide
      ? `Use styleGuideName="${selectedGuide.name}" and rootFrame background ${backgroundColor}.`
      : `Pick a suitable styleGuideName for platform=${platform} and set rootFrame background to ${backgroundColor}.`;

  const designMdPolicy = designMd ? buildDesignMdStylePolicy(designMd) : '';
  const designMdBlock = designMdPolicy
    ? [
        '',
        'USER DESIGN SYSTEM (design.md — follow these EXACTLY; they OVERRIDE any default):',
        designMdPolicy,
      ]
    : [];

  return {
    systemPrompt: [
      'You are a UI planning assistant. Output ONLY one JSON object.',
      'Schema: {"rootFrame":{"id":"page","name":"Page","width":375,"height":812,"layout":"vertical","gap":20,"fill":[{"type":"solid","color":"#111827"}]},"styleGuideName":"guide-name","subtasks":[{"id":"section-id","label":"Section Label","elements":"comma-separated owned UI elements","region":{"width":375,"height":240}}]}',
      'Every subtask MUST include: id, label, elements, region.width, region.height.',
      'Elements must not overlap between subtasks.',
      'Keep form controls and their submit action in the same subtask.',
      'Start the response with { and end with }. No prose. No markdown. No tool calls.',
      subtaskHint,
      styleRule,
      ...mobileRules,
      `Always set rootFrame layout="vertical" and gap=${defaultGap}.`,
      ...designMdBlock,
    ].join('\n'),
    userPrompt: prompt,
    selectedStyleGuideName: designMd ? DESIGN_MD_STYLE_GUIDE_NAME : selectedGuide?.name,
  };
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

function rankStyleGuidesForPrompt(tags: string[], platform: string) {
  return [...styleGuideRegistry].sort((a, b) => {
    const scoreA = styleGuidePromptScore(a.tags, tags, a.platform === platform);
    const scoreB = styleGuidePromptScore(b.tags, tags, b.platform === platform);
    if (scoreB !== scoreA) return scoreB - scoreA;
    if (a.platform === platform && b.platform !== platform) return -1;
    if (b.platform === platform && a.platform !== platform) return 1;
    return a.name.localeCompare(b.name);
  });
}

function styleGuidePromptScore(
  guideTags: string[],
  requestTags: string[],
  platformMatch: boolean,
): number {
  const overlap = requestTags.filter((tag) => guideTags.includes(tag)).length;
  return overlap * 10 + (platformMatch ? 3 : 0);
}

function formatGuideMetadataLine(
  guide: (typeof styleGuideRegistry)[number],
  mode: PlanningContextMode,
): string {
  const values = extractStyleGuideValues(guide.content);
  const bg = values.colors.background ? ` bg:${values.colors.background}` : '';
  const tags = guide.tags.slice(0, mode === 'rich' ? STYLE_GUIDE_METADATA_TAG_LIMIT : 3).join(', ');
  return `- ${guide.name} [${guide.platform}]${bg} :: ${tags}`;
}

function formatGuideSnippet(guide: (typeof styleGuideRegistry)[number]): string {
  const values = extractStyleGuideValues(guide.content);
  const tags = guide.tags.slice(0, 6).join(', ');
  const colors = [
    values.colors.background ? `bg=${values.colors.background}` : null,
    values.colors.surface ? `surface=${values.colors.surface}` : null,
    values.colors.accent ? `accent=${values.colors.accent}` : null,
  ]
    .filter(Boolean)
    .join(', ');
  const fonts = [
    values.typography.displayFont ? `display=${values.typography.displayFont}` : null,
    values.typography.bodyFont ? `body=${values.typography.bodyFont}` : null,
  ]
    .filter(Boolean)
    .join(', ');
  const radius = [
    values.radius.card != null ? `card=${values.radius.card}` : null,
    values.radius.button != null ? `button=${values.radius.button}` : null,
  ]
    .filter(Boolean)
    .join(', ');

  return [
    `### ${guide.name} [${guide.platform}]`,
    `tags: ${tags}`,
    colors ? `colors: ${colors}` : null,
    fonts ? `fonts: ${fonts}` : null,
    radius ? `radius: ${radius}` : null,
  ]
    .filter(Boolean)
    .join('\n');
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
