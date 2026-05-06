/**
 * Normalize 将
 *
 * Pencil.dev .pen 文档转换为 OpenPencil 的内部格式。 Handles 格式规范化 ONLY — NOT 是否解析
 * $variable
 * 引用： - 填充类型："color" → "solid" - 填充简写字符串 "#hex" → [{ type: "solid", color }]
 * - 渐变类型："gradient" →
 * "linear_gradient" / "radial_gradient" - 渐变停止 {
 * color,position } → { offset, color } - 大小调整"fit_content(N)" /
 * "fill_container(N)" → 后备编号 - 填充数组标准化 Variable 分辨率由
 *
 * `resolve-variables.ts` 在画布渲染时单独处理，保留文档中的 $variable 绑定。
 *
 */

import type { PenDocument, PenNode } from '@zseven-w/pen-types';
import type { PenFill, PenStroke, GradientStop } from '@zseven-w/pen-types';

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

export function normalizePenDocument(doc: PenDocument): PenDocument {
  const normalized = {
    ...doc,
    children: doc.children.map((n) => normalizeNode(n)),
  };
  // Normalize 也是所有页面的子页面
  if (normalized.pages && normalized.pages.length > 0) {
    normalized.pages = normalized.pages.map((p) => ({
      ...p,
      children: p.children.map((n) => normalizeNode(n)),
    }));
  }
  return normalized;
}

// ---------------------------------------------------------------------------
// Node 标准化器（递归）
// ---------------------------------------------------------------------------

function normalizeNode(node: PenNode): PenNode {
  const out: Record<string, unknown> = { ...node };

  // 填充
  if ('fill' in out && out.fill !== undefined) {
    out.fill = normalizeFills(out.fill);
  }

  // 中风
  if ('stroke' in out && out.stroke != null) {
    out.stroke = normalizeStroke(out.stroke as Record<string, unknown>);
  }

  // 效果 - 传递（无需更改格式）大小
  if ('width' in out) out.width = normalizeSizing(out.width);
  if ('height' in out) out.height = normalizeSizing(out.height);

  // gap — 传递（保留$变量字符串） padding — 仅规范化数组格式（不规范变量分辨率）
  if ('padding' in out) out.padding = normalizePadding(out.padding);

  // 不透明度 — 传递（保留 $variable 字符串）文本节点：将 `text` 字段规范化为
  // `content`（MCP/CLI 使用 `text`，渲染器需要 `content`）
  if (out.type === 'text' && !('content' in out) && typeof out.text === 'string') {
    out.content = out.text as string;
    delete out.text;
  }

  // icon_font：默认为 lucide 系列
  if (out.type === 'icon_font' && !out.iconFontFamily) {
    out.iconFontFamily = 'lucide';
  }

  // 儿童
  if ('children' in out && Array.isArray(out.children)) {
    out.children = (out.children as PenNode[]).map((c) => normalizeNode(c));
  }

  return out as unknown as PenNode;
}

// ---------------------------------------------------------------------------
// Fill 标准化
// ---------------------------------------------------------------------------

function normalizeFills(raw: unknown): PenFill[] {
  if (!raw) return [];

  // String 简写：“#hex”或“$variable”→实心填充
  if (typeof raw === 'string') {
    return [{ type: 'solid', color: raw }];
  }

  // Array 的填充
  if (Array.isArray(raw)) {
    return raw.map((f) => normalizeSingleFill(f)).filter(Boolean) as PenFill[];
  }

  // Single 填充对象
  if (typeof raw === 'object') {
    const f = normalizeSingleFill(raw as Record<string, unknown>);
    return f ? [f] : [];
  }

  return [];
}

function normalizeSingleFill(raw: Record<string, unknown> | string): PenFill | null {
  // String 数组内简写：“#hex”或“$variable”→实心填充
  if (typeof raw === 'string') {
    return raw ? { type: 'solid', color: raw } : null;
  }
  if (!raw || typeof raw !== 'object') return null;
  const t = raw.type as string | undefined;

  // Pencil“颜色”→OpenPencil“纯色”
  if (t === 'color' || t === 'solid') {
    return {
      type: 'solid',
      color: typeof raw.color === 'string' ? raw.color : '#000000',
    };
  }

  // Pencil“梯度”→被 gradientType 分割
  if (t === 'gradient') {
    const gt = (raw.gradientType as string) ?? 'linear';
    const stops = normalizeGradientStops(raw.colors as unknown[]);

    if (gt === 'radial') {
      const center = raw.center as Record<string, unknown> | undefined;
      return {
        type: 'radial_gradient',
        cx: typeof center?.x === 'number' ? center.x : 0.5,
        cy: typeof center?.y === 'number' ? center.y : 0.5,
        radius: 0.5,
        stops,
      };
    }
    // 线性或角度
    return {
      type: 'linear_gradient',
      angle: typeof raw.rotation === 'number' ? raw.rotation : 0,
      stops,
    };
  }

  // Already 我们的格式
  if (t === 'linear_gradient' || t === 'radial_gradient') {
    const stops =
      'stops' in raw
        ? normalizeGradientStops(raw.stops as unknown[])
        : 'colors' in raw
          ? normalizeGradientStops(raw.colors as unknown[])
          : [];
    return { ...(raw as unknown as PenFill), stops } as PenFill;
  }

  // Image fill — 通过
  if (t === 'image') return raw as unknown as PenFill;

  // Fallback：如果有色域，则视为实心
  if ('color' in raw) {
    return {
      type: 'solid',
      color: typeof raw.color === 'string' ? raw.color : '#000000',
    };
  }

  return null;
}

function normalizeGradientStops(raw: unknown[] | undefined): GradientStop[] {
  if (!Array.isArray(raw) || raw.length === 0) return [];

  // First pass：解析偏移量，收集显式设置的偏移量
  const parsed = raw.map((s: unknown) => {
    const stop = s as Record<string, unknown>;
    const rawOffset =
      typeof stop.offset === 'number' && Number.isFinite(stop.offset)
        ? stop.offset
        : typeof stop.position === 'number' && Number.isFinite(stop.position)
          ? stop.position
          : null;
    // Normalize 百分比格式偏移量（AI 有时输出 0-100 而不是 0-1）
    const offset = rawOffset !== null && rawOffset > 1 ? rawOffset / 100 : rawOffset;
    return {
      offset,
      color: typeof stop.color === 'string' ? stop.color : '#000000',
    };
  });

  // Second pass：自动分配任何缺少偏移量的停靠点
  const n = parsed.length;
  return parsed.map((s, i) => ({
    color: s.color,
    offset: s.offset !== null ? Math.max(0, Math.min(1, s.offset!)) : i / Math.max(n - 1, 1),
  }));
}

// ---------------------------------------------------------------------------
// Stroke 标准化
// ---------------------------------------------------------------------------

function normalizeStroke(raw: Record<string, unknown>): PenStroke | undefined {
  if (!raw) return undefined;
  const out = { ...raw };

  // Normalize 填充内部描边
  if ('fill' in out) {
    out.fill = normalizeFills(out.fill);
  }

  // Pencil 可以直接在笔画上使用“颜色”
  if ('color' in out && typeof out.color === 'string') {
    out.fill = [{ type: 'solid', color: out.color as string }];
    delete out.color;
  }

  // Thickness：按原样保留 $variable 字符串，规范化纯数字字符串
  if (typeof out.thickness === 'string') {
    const str = out.thickness as string;
    if (!str.startsWith('$')) {
      const num = parseFloat(str);
      out.thickness = isNaN(num) ? 1 : num;
    }
  }

  return out as unknown as PenStroke;
}

// ---------------------------------------------------------------------------
// Sizing 标准化
// ---------------------------------------------------------------------------

function normalizeSizing(value: unknown): number | string {
  if (typeof value === 'number') return value;
  if (typeof value !== 'string') return 0;

  // $variable——传递
  if (value.startsWith('$')) return value;

  // fill_container must always resolve dynamically from parent dimensions
  if (value.startsWith('fill_container')) return 'fill_container';

  // fit_content with a hint value: use the hint (more accurate than our estimation)
  if (value.startsWith('fit_content')) {
    const match = value.match(/\((\d+(?:\.\d+)?)\)/);
    if (match) return parseFloat(match[1]);
    return 'fit_content';
  }

  // Try 作为纯数字字符串
  const num = parseFloat(value);
  return isNaN(num) ? 0 : num;
}

function normalizePadding(
  value: unknown,
): number | [number, number] | [number, number, number, number] | string | undefined {
  if (typeof value === 'number') return value;
  if (typeof value === 'string') {
    // $variable——传递
    if (value.startsWith('$')) return value;
    const num = parseFloat(value);
    return isNaN(num) ? 0 : num;
  }
  if (Array.isArray(value)) {
    return value.map((v) => {
      if (typeof v === 'number') return v;
      if (typeof v === 'string') {
        const num = parseFloat(v);
        return isNaN(num) ? 0 : num;
      }
      return 0;
    }) as [number, number] | [number, number, number, number];
  }
  return undefined;
}
