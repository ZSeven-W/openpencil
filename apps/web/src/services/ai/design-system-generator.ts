/**
 * 设计系统生成器（视觉参考管道的 Stage 0）。
 *
 * 它会先从用户提示里提炼出结构化设计 token，
 * 比如颜色、排版、间距和圆角。
 * 这些 token 有两个用途：
 * 1. 约束 HTML 参考代码的生成风格
 * 2. 映射到 `PenDocument.variables`，接入文档级设计系统
 */

import type { DesignSystem } from './ai-types';
import type { AIProviderType } from '@/types/agent-settings';
import type { VariableDefinition } from '@/types/variables';
import { generateCompletion } from './ai-service';
import { getSkillByName } from '@zseven-w/pen-ai-skills';

/**
 * 根据用户提示生成一个设计系统。
 * 这里通常可以使用更快的模型，因为输出体积很小，本质上只是一份结构化 JSON。
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
 * 从 AI 的响应文本里解析设计系统。
 * 兼容代码围栏、解释性文字包裹等常见返回形式。
 */
function parseDesignSystem(text: string): DesignSystem {
  const trimmed = text.trim();

  // 先尝试直接按 JSON 解析
  const direct = tryParseDS(trimmed);
  if (direct) return direct;

  // 再尝试从代码围栏中提取 JSON
  const fenceMatch = trimmed.match(/```(?:json)?\s*\n?([\s\S]*?)\n?```/);
  if (fenceMatch) {
    const fenced = tryParseDS(fenceMatch[1].trim());
    if (fenced) return fenced;
  }

  // 最后尝试截取第一个 `{ ... }` 代码块
  const firstBrace = trimmed.indexOf('{');
  const lastBrace = trimmed.lastIndexOf('}');
  if (firstBrace >= 0 && lastBrace > firstBrace) {
    const braced = tryParseDS(trimmed.slice(firstBrace, lastBrace + 1));
    if (braced) return braced;
  }

  // 兜底：返回默认设计系统
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
// 设计系统 → PenDocument.variables
// ---------------------------------------------------------------------------

/**
 * 把 `DesignSystem` 转换为 `PenDocument` 的变量定义。
 * 这些变量会存进文档，并在节点里通过 `$variable-name` 的形式引用。
 */
export function designSystemToVariables(ds: DesignSystem): Record<string, VariableDefinition> {
  const vars: Record<string, VariableDefinition> = {};

  // 颜色变量
  for (const [key, value] of Object.entries(ds.palette)) {
    const name = `color-${kebab(key)}`;
    vars[name] = { type: 'color', value };
  }

  // 间距变量
  const spacingNames = ['xs', 'sm', 'md', 'lg', 'xl', '2xl', '3xl', '4xl', '5xl', '6xl'];
  for (let i = 0; i < ds.spacing.scale.length && i < spacingNames.length; i++) {
    vars[`spacing-${spacingNames[i]}`] = { type: 'number', value: ds.spacing.scale[i] };
  }

  // 圆角变量
  const radiusNames = ['sm', 'md', 'lg', 'xl'];
  for (let i = 0; i < ds.radius.length && i < radiusNames.length; i++) {
    vars[`radius-${radiusNames[i]}`] = { type: 'number', value: ds.radius[i] };
  }

  return vars;
}

/** 构造一段简洁的设计系统上下文文本，供 AI prompt 使用。 */
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
