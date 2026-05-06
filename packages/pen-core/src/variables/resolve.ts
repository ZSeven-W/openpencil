/**
 * Variable
 *
 * 解析实用程序。 Resolves `$variableNam
 * e` 针对 Variab
 leDefinition 地图的引用，可选择将主题值与活动主题上下文匹配。
 */

import type { PenNode } from '@zseven-w/pen-types';
import type { PenFill, PenStroke, PenEffect } from '@zseven-w/pen-types';
import type { VariableDefinition, ThemedValue } from '@zseven-w/pen-types';

type Vars = Record<string, VariableDefinition>;
type Theme = Record<string, string>;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Check 值是否是 `$variable` 引用字符串。 */
export function isVariableRef(value: unknown): value is string {
  return typeof value === 'string' && value.startsWith('$');
}

/** Build 来自 PenDocument.themes 的默认主题图（每个轴的第一个值）。 */
export function getDefaultTheme(themes: Record<string, string[]> | undefined): Theme {
  const result: Theme = {};
  if (!themes) return result;
  for (const [key, values] of Object.entries(themes)) {
    if (values.length > 0) result[key] = values[0];
  }
  return result;
}

// ---------------------------------------------------------------------------
// Core 分辨率
// ---------------------------------------------------------------------------

/** Pick 给定主题的 `ThemedValue[]` 的具体值。 */
function resolveThemedValue(
  values: ThemedValue[],
  activeTheme?: Theme,
): string | number | boolean | undefined {
  if (activeTheme && Object.keys(activeTheme).length > 0) {
    const match = values.find((v) => {
      if (!v.theme) return false;
      return Object.entries(activeTheme).every(([key, expected]) => v.theme?.[key] === expected);
    });
    if (match) return match.value;
  }
  return values[0]?.value;
}

/**
 * Resolve 单个
 * `$variableName` 对其具体值的引用。 Returns `undefined` 如果变量不存在或具有不兼容的类型。
 */
export function resolveVariableRef(
  ref: string,
  variables: Vars,
  activeTheme?: Theme,
): string | number | boolean | undefined {
  if (!ref.startsWith('$')) return undefined;
  const name = ref.slice(1);
  const def = variables[name];
  if (!def) return undefined;

  const val = def.value;
  if (Array.isArray(val)) {
    const resolved = resolveThemedValue(val, activeTheme);
    // Circular 防护：如果解析值也是 $ref，则停止
    if (typeof resolved === 'string' && resolved.startsWith('$')) return undefined;
    return resolved;
  }
  // Circular 守卫
  if (typeof val === 'string' && val.startsWith('$')) return undefined;
  return val;
}

/**
 * Resolve
 * 颜色字符串，可能是 `$variable` 引用。 Returns 原始字符串（如果不是引用）或解析的颜色。
 */
export function resolveColorRef(
  color: string | undefined,
  variables: Vars,
  activeTheme?: Theme,
): string | undefined {
  if (color === undefined) return undefined;
  if (!isVariableRef(color)) return color;
  const resolved = resolveVariableRef(color, variables, activeTheme);
  return typeof resolved === 'string' ? resolved : undefined;
}

/**
 * Resolve 可能是
 * `$variable` 引用的数值。 Returns 如果不是参考号，则为原始号码。
 */
export function resolveNumericRef(
  value: unknown,
  variables: Vars,
  activeTheme?: Theme,
): number | undefined {
  if (typeof value === 'number') return value;
  if (typeof value === 'string') {
    if (isVariableRef(value)) {
      const resolved = resolveVariableRef(value, variables, activeTheme);
      return typeof resolved === 'number' ? resolved : undefined;
    }
    const num = parseFloat(value);
    return isNaN(num) ? undefined : num;
  }
  return undefined;
}

// ---------------------------------------------------------------------------
// Fill / 描边 / 效果分辨率
// ---------------------------------------------------------------------------

function resolveFillsForCanvas(
  fills: PenFill[] | string | undefined,
  vars: Vars,
  theme?: Theme,
): PenFill[] | string | undefined {
  if (!fills) return fills;
  if (typeof fills === 'string') return fills;
  return fills.map((fill) => {
    if (fill.type === 'solid') {
      const color = resolveColorRef(fill.color, vars, theme);
      return color !== fill.color ? { ...fill, color: color ?? '#000000' } : fill;
    }
    if (fill.type === 'linear_gradient' || fill.type === 'radial_gradient') {
      const newStops = fill.stops.map((stop) => {
        const color = resolveColorRef(stop.color, vars, theme);
        return color !== stop.color ? { ...stop, color: color ?? '#000000' } : stop;
      });
      return newStops !== fill.stops ? { ...fill, stops: newStops } : fill;
    }
    return fill;
  });
}

function resolveStrokeForCanvas(
  stroke: PenStroke | undefined,
  vars: Vars,
  theme?: Theme,
): PenStroke | undefined {
  if (!stroke) return stroke;
  let changed = false;
  const out: Record<string, unknown> = { ...stroke };

  // Resolve 厚度
  if (typeof stroke.thickness === 'string' && isVariableRef(stroke.thickness)) {
    out.thickness = resolveNumericRef(stroke.thickness, vars, theme) ?? 1;
    changed = true;
  }

  // Resolve 描边填充颜色
  if (stroke.fill) {
    const resolved = resolveFillsForCanvas(stroke.fill, vars, theme);
    if (resolved !== stroke.fill) {
      out.fill = resolved;
      changed = true;
    }
  }

  return changed ? (out as unknown as PenStroke) : stroke;
}

function resolveEffectsForCanvas(
  effects: PenEffect[] | undefined,
  vars: Vars,
  theme?: Theme,
): PenEffect[] | undefined {
  if (!effects) return effects;
  return effects.map((effect) => {
    if (effect.type !== 'shadow') return effect;
    let changed = false;
    const out: Record<string, unknown> = { ...effect };

    if (typeof effect.color === 'string' && isVariableRef(effect.color)) {
      out.color = resolveColorRef(effect.color, vars, theme) ?? '#000000';
      changed = true;
    }
    for (const key of ['blur', 'offsetX', 'offsetY', 'spread'] as const) {
      const val = effect[key];
      if (typeof val === 'string' && isVariableRef(val)) {
        out[key] = resolveNumericRef(val, vars, theme) ?? 0;
        changed = true;
      }
    }

    return changed ? (out as unknown as PenEffect) : effect;
  });
}

// ---------------------------------------------------------------------------
// Full 画布渲染的节点分辨率
// ---------------------------------------------------------------------------

/**
 * Resolve
 * PenNode 中的所有 `$variable` 引用，返回一个具有适合 Fabric.js
 *
 * 渲染的具体值的新节点。当不存在变量时，Returns 相同的对象引用。
 */
export function resolveNodeForCanvas(node: PenNode, variables: Vars, activeTheme?: Theme): PenNode {
  if (!variables || Object.keys(variables).length === 0) return node;

  let changed = false;
  const out: Record<string, unknown> = { ...node };

  // Opacity
  if (typeof node.opacity === 'string' && isVariableRef(node.opacity)) {
    out.opacity = resolveNumericRef(node.opacity, variables, activeTheme) ?? 1;
    changed = true;
  }

  // Gap
  if ('gap' in node && typeof (node as unknown as Record<string, unknown>).gap === 'string') {
    const gap = (node as unknown as Record<string, unknown>).gap as string;
    if (isVariableRef(gap)) {
      out.gap = resolveNumericRef(gap, variables, activeTheme) ?? 0;
      changed = true;
    }
  }

  // Padding
  if ('padding' in node) {
    const padding = (node as unknown as Record<string, unknown>).padding;
    if (typeof padding === 'string' && isVariableRef(padding)) {
      out.padding = resolveNumericRef(padding, variables, activeTheme) ?? 0;
      changed = true;
    }
  }

  // Fill
  if ('fill' in node && (node as unknown as Record<string, unknown>).fill) {
    const fills = (node as unknown as Record<string, unknown>).fill as PenFill[] | string;
    const resolved = resolveFillsForCanvas(fills, variables, activeTheme);
    if (resolved !== fills) {
      out.fill = resolved;
      changed = true;
    }
  }

  // Stroke
  if ('stroke' in node && (node as unknown as Record<string, unknown>).stroke) {
    const stroke = (node as unknown as Record<string, unknown>).stroke as PenStroke;
    const resolved = resolveStrokeForCanvas(stroke, variables, activeTheme);
    if (resolved !== stroke) {
      out.stroke = resolved;
      changed = true;
    }
  }

  // Effects
  if ('effects' in node && (node as unknown as Record<string, unknown>).effects) {
    const effects = (node as unknown as Record<string, unknown>).effects as PenEffect[];
    const resolved = resolveEffectsForCanvas(effects, variables, activeTheme);
    if (resolved !== effects) {
      out.effects = resolved;
      changed = true;
    }
  }

  // Text 内容
  if (node.type === 'text' && typeof node.content === 'string' && isVariableRef(node.content)) {
    const resolved = resolveVariableRef(node.content, variables, activeTheme);
    if (typeof resolved === 'string') {
      out.content = resolved;
      changed = true;
    }
  }

  // Recurse 进入儿童
  if ('children' in node && node.children) {
    const children = node.children;
    const resolvedChildren = children.map((child) =>
      resolveNodeForCanvas(child, variables, activeTheme),
    );
    // Only 如果任何子项实际发生更改，则分配新数组
    if (resolvedChildren.some((rc, i) => rc !== children[i])) {
      out.children = resolvedChildren;
      changed = true;
    }
  }

  return changed ? (out as unknown as PenNode) : node;
}
