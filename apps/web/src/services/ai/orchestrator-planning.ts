import type { OrchestratorPlan, StyleGuide, SubTask } from './ai-types';
import type { DesignMdSpec } from '@/types/design-md';
import { detectDesignType } from './design-type-presets';
import {
  buildFallbackPlanFromPrompt,
  DESIGN_MD_STYLE_GUIDE_NAME,
  guessNeutralBackgroundFromTheme,
  inferDesignMdBackground,
} from './orchestrator-prompt-optimizer';

export function filterPlanningSkillsForPrompt<T extends { meta: { name: string } }>(
  skills: T[],
  prompt: string,
): T[] {
  const designType = detectDesignType(prompt).type;
  if (designType === 'landing-page') return skills;
  return skills.filter((skill) => skill.meta.name !== 'landing-page-predesign');
}

export function parseOrchestratorResponse(
  raw: string,
  prompt: string,
  designMd?: DesignMdSpec,
): { plan: OrchestratorPlan; repaired: boolean } | null {
  const trimmed = raw.trim();

  const direct = tryParsePlan(trimmed, designMd);
  if (direct) return { plan: direct, repaired: false };

  const repairedDirect = tryRepairPlan(trimmed, prompt, designMd);
  if (repairedDirect) return { plan: repairedDirect, repaired: true };

  const fenceMatch = trimmed.match(/```(?:json)?\s*\n?([\s\S]*?)\n?```/);
  if (fenceMatch) {
    const fencedText = fenceMatch[1].trim();
    const fenced = tryParsePlan(fencedText, designMd);
    if (fenced) return { plan: fenced, repaired: false };

    const repairedFenced = tryRepairPlan(fencedText, prompt, designMd);
    if (repairedFenced) return { plan: repairedFenced, repaired: true };
  }

  const firstBrace = trimmed.indexOf('{');
  const lastBrace = trimmed.lastIndexOf('}');
  if (firstBrace >= 0 && lastBrace > firstBrace) {
    const bracedText = trimmed.slice(firstBrace, lastBrace + 1);
    const braced = tryParsePlan(bracedText, designMd);
    if (braced) return { plan: braced, repaired: false };

    const repairedBraced = tryRepairPlan(bracedText, prompt, designMd);
    if (repairedBraced) return { plan: repairedBraced, repaired: true };
  }

  return null;
}

function tryParsePlan(text: string, designMd?: DesignMdSpec): OrchestratorPlan | null {
  try {
    const obj = JSON.parse(text) as Record<string, unknown>;
    if (!obj.rootFrame || typeof obj.rootFrame !== 'object') return null;
    if (!Array.isArray(obj.subtasks) || obj.subtasks.length === 0) return null;

    const rf = obj.rootFrame as Record<string, unknown>;
    if (!rf.id || !rf.width || rf.height == null) return null;

    for (const st of obj.subtasks as Record<string, unknown>[]) {
      if (!st.id || !st.region) return null;
    }

    return finalizePlan(obj as unknown as OrchestratorPlan, obj, designMd);
  } catch {
    return null;
  }
}

function tryRepairPlan(
  text: string,
  prompt: string,
  designMd?: DesignMdSpec,
): OrchestratorPlan | null {
  try {
    const obj = JSON.parse(text) as Record<string, unknown>;
    return repairPlanObject(obj, prompt, designMd);
  } catch {
    return null;
  }
}

function repairPlanObject(
  obj: Record<string, unknown>,
  prompt: string,
  designMd?: DesignMdSpec,
): OrchestratorPlan | null {
  const fallback = buildFallbackPlanFromPrompt(prompt, designMd);
  const rawSubtasks = extractSubtaskCandidates(obj);
  if (rawSubtasks.length === 0) return null;

  const rootSource = isRecord(obj.rootFrame) ? obj.rootFrame : obj;
  const fallbackHeights = buildFallbackHeights(fallback, rawSubtasks.length);

  // Shape the rootFrame first — finalizePlan will re-stamp rootFrame.fill
  // below when design.md is present, so we don't need to special-case the fill
  // here. For non-design.md flows, honor the model's fill when coercible.
  const rootFrame: OrchestratorPlan['rootFrame'] = {
    id: asString(rootSource.id) ?? fallback.rootFrame.id,
    name: asString(rootSource.name) ?? fallback.rootFrame.name,
    width: asPositiveNumber(rootSource.width) ?? fallback.rootFrame.width,
    height: asNonNegativeNumber(rootSource.height) ?? fallback.rootFrame.height,
    layout: asLayout(rootSource.layout) ?? fallback.rootFrame.layout ?? 'vertical',
    gap: asNonNegativeNumber(rootSource.gap) ?? fallback.rootFrame.gap,
    fill: coerceFill(rootSource.fill) ?? fallback.rootFrame.fill,
  };

  const subtasks: SubTask[] = rawSubtasks
    .map((candidate, index) =>
      coerceSubtask(candidate, index, rootFrame.width, fallbackHeights[index] ?? 160),
    )
    .filter((subtask): subtask is SubTask => subtask !== null);

  if (subtasks.length === 0) return null;

  const repaired: OrchestratorPlan = {
    rootFrame,
    // When design.md is present, force the design-md style guide name so a
    // stale catalog name left over from invalid planner JSON can't leak into
    // downstream sub-agent prompts.
    styleGuideName: designMd
      ? DESIGN_MD_STYLE_GUIDE_NAME
      : (asString(obj.styleGuideName) ?? asString(obj.style_guide) ?? fallback.styleGuideName),
    subtasks,
  };

  if (!designMd && isRecord(obj.styleGuide)) {
    repaired.styleGuide = obj.styleGuide as unknown as StyleGuide;
  }

  return finalizePlan(repaired, obj, designMd);
}

function finalizePlan(
  plan: OrchestratorPlan,
  rawObj?: Record<string, unknown>,
  designMd?: DesignMdSpec,
): OrchestratorPlan {
  // design.md always wins: strip any catalog styleGuideName/styleGuide the
  // model might have emitted, force the root background to the design.md
  // primary palette color (or a neutral default biased by visualTheme when no
  // palette entry is tagged as background — DO NOT trust the model's own
  // rootFrame.fill here because it often lifts a brand/CTA color from the
  // palette), and leave plan.styleGuide unset so downstream consumers fall
  // back to the buildDesignMdStylePolicy path.
  if (designMd) {
    plan.styleGuideName = DESIGN_MD_STYLE_GUIDE_NAME;
    plan.styleGuide = undefined;
    const bg =
      inferDesignMdBackground(designMd) ?? guessNeutralBackgroundFromTheme(designMd.visualTheme);
    plan.rootFrame.fill = [{ type: 'solid', color: bg }];
    return plan;
  }

  if (!plan.styleGuide && rawObj && isRecord(rawObj.styleGuide)) {
    plan.styleGuide = rawObj.styleGuide as unknown as StyleGuide;
  }

  if (!plan.styleGuide) {
    const bg =
      (plan.rootFrame.fill as Array<{ color?: string }> | undefined)?.[0]?.color ?? '#F8FAFC';
    plan.styleGuide = {
      palette: {
        background: bg,
        surface: '#FFFFFF',
        text: '#0F172A',
        secondary: '#64748B',
        accent: '#6366F1',
        accent2: '#8B5CF6',
        border: '#E2E8F0',
      },
      fonts: { heading: 'Space Grotesk', body: 'Inter' },
      aesthetic: 'clean modern',
    };
  }

  return plan;
}

function extractSubtaskCandidates(obj: Record<string, unknown>): unknown[] {
  if (Array.isArray(obj.subtasks)) return obj.subtasks;
  if (Array.isArray(obj.sections)) return obj.sections;
  if (Array.isArray(obj.tasks)) return obj.tasks;
  return [];
}

function buildFallbackHeights(fallback: OrchestratorPlan, count: number): number[] {
  if (count <= 0) return [];
  if (fallback.rootFrame.width <= 500) {
    const perSection = Math.floor((fallback.rootFrame.height || 812) / count);
    return Array.from({ length: count }, () => perSection);
  }

  const totalHeight = fallback.rootFrame.height || (count >= 4 ? 4000 : 800);
  return allocateSectionHeights(totalHeight, count);
}

function coerceSubtask(
  candidate: unknown,
  index: number,
  rootWidth: number,
  defaultHeight: number,
): SubTask | null {
  if (typeof candidate === 'string') {
    const label = candidate.trim();
    if (!label) return null;
    return {
      id: makeSafeSectionId(label, index),
      label,
      region: { width: rootWidth, height: defaultHeight },
      idPrefix: '',
      parentFrameId: null,
    };
  }

  if (!isRecord(candidate)) return null;

  const label =
    asString(candidate.label) ??
    asString(candidate.name) ??
    asString(candidate.title) ??
    asString(candidate.section) ??
    `Section ${index + 1}`;
  const regionSource = isRecord(candidate.region) ? candidate.region : candidate;
  const width = asPositiveNumber(regionSource.width) ?? rootWidth;
  const height =
    asPositiveNumber(regionSource.height) ?? asPositiveNumber(candidate.height) ?? defaultHeight;

  return {
    id: asString(candidate.id) ?? makeSafeSectionId(label, index),
    label,
    elements: asElements(candidate.elements ?? candidate.scope ?? candidate.description),
    region: { width, height },
    idPrefix: '',
    parentFrameId: null,
    screen: asString(candidate.screen) ?? asString(candidate.page),
  };
}

function asElements(value: unknown): string | undefined {
  if (typeof value === 'string') {
    const trimmed = value.trim();
    return trimmed.length > 0 ? trimmed : undefined;
  }
  if (Array.isArray(value)) {
    const parts = value
      .filter((item): item is string => typeof item === 'string')
      .map((item) => item.trim())
      .filter(Boolean);
    return parts.length > 0 ? parts.join(', ') : undefined;
  }
  return undefined;
}

function asString(value: unknown): string | undefined {
  return typeof value === 'string' && value.trim().length > 0 ? value.trim() : undefined;
}

function asPositiveNumber(value: unknown): number | undefined {
  return typeof value === 'number' && Number.isFinite(value) && value > 0 ? value : undefined;
}

function asNonNegativeNumber(value: unknown): number | undefined {
  return typeof value === 'number' && Number.isFinite(value) && value >= 0 ? value : undefined;
}

function asLayout(value: unknown): OrchestratorPlan['rootFrame']['layout'] | undefined {
  return value === 'none' || value === 'vertical' || value === 'horizontal' ? value : undefined;
}

function coerceFill(value: unknown): Array<{ type: string; color: string }> | undefined {
  if (Array.isArray(value)) {
    const solids = value
      .filter(isRecord)
      .map((entry) => {
        const color = asString(entry.color);
        const type = asString(entry.type) ?? 'solid';
        return color ? { type, color } : null;
      })
      .filter((entry): entry is { type: string; color: string } => entry !== null);
    return solids.length > 0 ? solids : undefined;
  }

  const color = asString(value);
  return color ? [{ type: 'solid', color }] : undefined;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === 'object' && !Array.isArray(value);
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
  const weights = Array.from({ length: count }, (_, i) => {
    if (i === 0) return 1.4;
    if (i === count - 1 && count >= 3) return 0.6;
    return 1.0;
  });
  const totalWeight = weights.reduce((sum, weight) => sum + weight, 0);
  const heights = weights.map((weight) =>
    Math.max(minHeight, Math.round((totalHeight * weight) / totalWeight)),
  );

  let allocated = heights.reduce((sum, height) => sum + height, 0);
  let index = Math.floor(count / 2);
  while (allocated < totalHeight) {
    heights[index] += 1;
    allocated += 1;
    index = (index + 1) % count;
  }

  index = count - 1;
  while (allocated > totalHeight) {
    if (heights[index] > minHeight) {
      heights[index] -= 1;
      allocated -= 1;
    }
    index -= 1;
    if (index < 0) index = count - 1;
  }

  return heights;
}
