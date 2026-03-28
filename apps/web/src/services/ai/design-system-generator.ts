/**
 * Design System Generator (Stage 0 of visual reference pipeline).
 *
 * Generates structured design tokens (colors, typography, spacing) from
 * a user's design request. The tokens serve dual purpose:
 * 1. Guide HTML code generation for consistent design
 * 2. Map to PenDocument.variables for design system integration
 */

import type { DesignSystem } from './ai-types';
import type { AIProviderType } from '@/types/agent-settings';
import type { VariableDefinition } from '@/types/variables';
import { generateCompletion } from './ai-service';
import { getSkillByName } from '@zseven-w/pen-ai-skills';

/**
 * Generate a design system from a user's prompt.
 * Uses a fast model (Haiku-class) for speed since output is small JSON.
 */
export async function generateDesignSystem(
  prompt: string,
  model?: string,
  provider?: AIProviderType,
): Promise<DesignSystem> {
  const designSystemPrompt = getSkillByName('design-system')?.content ?? '';
  const response = await generateCompletion(designSystemPrompt, prompt, model, provider);

  return parseDesignSystem(response);
}

/**
 * Parse a design system from AI response text.
 * Tolerant of code fences and surrounding text.
 */
function parseDesignSystem(text: string): DesignSystem {
  const trimmed = text.trim();

  // Try direct parse
  const direct = tryParseDS(trimmed);
  if (direct) return direct;

  // Try extracting from code fences
  const fenceMatch = trimmed.match(/```(?:json)?\s*\n?([\s\S]*?)\n?```/);
  if (fenceMatch) {
    const fenced = tryParseDS(fenceMatch[1].trim());
    if (fenced) return fenced;
  }

  // Try extracting first { ... } block
  const firstBrace = trimmed.indexOf('{');
  const lastBrace = trimmed.lastIndexOf('}');
  if (firstBrace >= 0 && lastBrace > firstBrace) {
    const braced = tryParseDS(trimmed.slice(firstBrace, lastBrace + 1));
    if (braced) return braced;
  }

  // Fallback: return default design system
  return DEFAULT_DESIGN_SYSTEM;
}

function tryParseDS(json: string): DesignSystem | null {
  try {
    const obj = JSON.parse(json) as Record<string, unknown>;
    if (!obj.palette || typeof obj.palette !== 'object') return null;
    if (!obj.typography || typeof obj.typography !== 'object') return null;

    const p = obj.palette as Record<string, string>;
    const t = obj.typography as Record<string, unknown>;
    const s = (obj.spacing as Record<string, unknown>) ?? {
      unit: 8,
      scale: [8, 16, 24, 32, 48, 64],
    };

    return {
      palette: {
        background: p.background ?? '#F8FAFC',
        surface: p.surface ?? '#FFFFFF',
        text: p.text ?? '#0F172A',
        textSecondary: p.textSecondary ?? '#475569',
        primary: p.primary ?? '#2563EB',
        primaryLight: p.primaryLight ?? '#DBEAFE',
        accent: p.accent ?? '#0EA5E9',
        border: p.border ?? '#E2E8F0',
      },
      typography: {
        headingFont: (t.headingFont as string) ?? 'Space Grotesk',
        bodyFont: (t.bodyFont as string) ?? 'Inter',
        scale: Array.isArray(t.scale) ? (t.scale as number[]) : [14, 16, 20, 28, 40, 56],
      },
      spacing: {
        unit: (s.unit as number) ?? 8,
        scale: Array.isArray(s.scale) ? (s.scale as number[]) : [8, 16, 24, 32, 48, 64],
      },
      radius: Array.isArray(obj.radius) ? (obj.radius as number[]) : [8, 12, 16],
      aesthetic: (obj.aesthetic as string) ?? 'clean modern',
    };
  } catch {
    return null;
  }
}

const DEFAULT_DESIGN_SYSTEM: DesignSystem = {
  palette: {
    background: '#F8FAFC',
    surface: '#FFFFFF',
    text: '#0F172A',
    textSecondary: '#475569',
    primary: '#2563EB',
    primaryLight: '#DBEAFE',
    accent: '#0EA5E9',
    border: '#E2E8F0',
  },
  typography: {
    headingFont: 'Space Grotesk',
    bodyFont: 'Inter',
    scale: [14, 16, 20, 28, 40, 56],
  },
  spacing: {
    unit: 8,
    scale: [8, 16, 24, 32, 48, 64],
  },
  radius: [8, 12, 16],
  aesthetic: 'clean modern blue',
};

// ---------------------------------------------------------------------------
// Map design system → PenDocument.variables
// ---------------------------------------------------------------------------

/**
 * Convert a DesignSystem into PenDocument variable definitions.
 * These are stored in the document and referenced as $variable-name in nodes.
 */
export function designSystemToVariables(ds: DesignSystem): Record<string, VariableDefinition> {
  const vars: Record<string, VariableDefinition> = {};

  // Colors
  for (const [key, value] of Object.entries(ds.palette)) {
    const name = `color-${kebab(key)}`;
    vars[name] = { type: 'color', value };
  }

  // Spacing
  const spacingNames = ['xs', 'sm', 'md', 'lg', 'xl', '2xl', '3xl', '4xl', '5xl', '6xl'];
  for (let i = 0; i < ds.spacing.scale.length && i < spacingNames.length; i++) {
    vars[`spacing-${spacingNames[i]}`] = { type: 'number', value: ds.spacing.scale[i] };
  }

  // Radius
  const radiusNames = ['sm', 'md', 'lg', 'xl'];
  for (let i = 0; i < ds.radius.length && i < radiusNames.length; i++) {
    vars[`radius-${radiusNames[i]}`] = { type: 'number', value: ds.radius[i] };
  }

  return vars;
}

/**
 * Build a concise design system context string for AI prompts.
 */
export function designSystemToPromptContext(ds: DesignSystem): string {
  const p = ds.palette;
  return `DESIGN SYSTEM (use these values consistently):
Colors: bg ${p.background}, surface ${p.surface}, text ${p.text}, muted ${p.textSecondary}, primary ${p.primary}, primaryLight ${p.primaryLight}, accent ${p.accent}, border ${p.border}
Fonts: heading "${ds.typography.headingFont}", body "${ds.typography.bodyFont}"
Type scale: ${ds.typography.scale.join(', ')}px
Spacing: ${ds.spacing.scale.join(', ')}px (${ds.spacing.unit}px grid)
Radius: ${ds.radius.join(', ')}px
Style: ${ds.aesthetic}`;
}

function kebab(str: string): string {
  return str.replace(/([a-z])([A-Z])/g, '$1-$2').toLowerCase();
}
