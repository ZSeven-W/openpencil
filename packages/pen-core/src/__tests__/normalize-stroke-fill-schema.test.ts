import { describe, it, expect } from 'vitest';
import type { PenNode } from '@zseven-w/pen-types';
import { normalizeStrokeFillSchema } from '../normalize/normalize-stroke-fill-schema';

// Test 帮助器接受无类型属性包，因为这里的大多数断言故意将格式错误（违反架构）的数据放在节点上 -
// 这正是规范化器旨在修复的输入形状。强类型签名会拒绝我们需要涵盖的情况。
type NodeProps = Record<string, unknown>;

const path = (props: NodeProps = {}): PenNode =>
  ({
    id: 'p',
    type: 'path',
    d: 'M0 0 L100 0',
    width: 100,
    height: 50,
    ...props,
  }) as unknown as PenNode;

const ellipse = (props: NodeProps = {}): PenNode =>
  ({
    id: 'e',
    type: 'ellipse',
    width: 100,
    height: 100,
    ...props,
  }) as unknown as PenNode;

const frame = (props: NodeProps & { children?: PenNode[] } = {}): PenNode =>
  ({
    id: 'f',
    type: 'frame',
    width: 200,
    height: 200,
    ...props,
  }) as unknown as PenNode;

const text = (props: NodeProps = {}): PenNode =>
  ({
    id: 't',
    type: 'text',
    content: 'Hello',
    fontSize: 16,
    ...props,
  }) as unknown as PenNode;

const iconFont = (props: NodeProps = {}): PenNode =>
  ({
    id: 'i',
    type: 'icon_font',
    iconFontName: 'heart',
    width: 16,
    height: 16,
    ...props,
  }) as unknown as PenNode;

const validFill = [{ type: 'solid' as const, color: '#C4F82A' }];

describe('normalizeStrokeFillSchema — stroke array unwrap', () => {
  it('unwraps a stroke that is an array of one proper PenStroke object', () => {
    const node = ellipse({
      stroke: [{ thickness: 12, fill: validFill }],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { stroke?: { thickness?: number; fill?: unknown } };
    expect(Array.isArray(rec.stroke)).toBe(false);
    expect(rec.stroke?.thickness).toBe(12);
    expect(rec.stroke?.fill).toEqual(validFill);
  });

  it('leaves a stroke that is already a proper object alone', () => {
    const node = ellipse({
      stroke: { thickness: 4, fill: validFill },
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { stroke?: { thickness?: number; fill?: unknown } };
    expect(rec.stroke?.thickness).toBe(4);
  });

  it('converts a fill-shaped stroke entry into a proper PenStroke', () => {
    // Real M2.7 失败：中风是一个数组，内部对象有
// {type, color} (fill shape) instead of {thickness, fill}. strokeWidth
    // 作为顶级节点字段。
    const node = path({
      stroke: [{ type: 'solid', color: '#C4F82A' }],
      strokeWidth: 2.5,
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as {
      stroke?: { thickness?: number; fill?: Array<{ color?: string }> };
      strokeWidth?: number;
    };
    expect(rec.stroke?.thickness).toBe(2.5);
    expect(rec.stroke?.fill?.[0]?.color).toBe('#C4F82A');
    // Stray strokeWidth 字段在迁移后被清理
    expect(rec.strokeWidth).toBeUndefined();
  });

  it('uses a default thickness when neither strokeWidth nor thickness is present', () => {
    const node = path({
      stroke: [{ type: 'solid', color: '#111' }],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { stroke?: { thickness?: number; fill?: unknown } };
    expect(rec.stroke?.thickness).toBeGreaterThan(0);
    expect(rec.stroke?.fill).toBeDefined();
  });

  it('handles an object-shaped stroke with a fill-shaped inner value', () => {
    // Some 子代理发出 { stroke: { type: "solid", color: "#fff" } }
    // （对象，而不是数组）— 仍然是填充形状。 Same 恢复规则适用。
    const node = path({
      stroke: { type: 'solid', color: '#FFFFFF' },
      strokeWidth: 3,
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as {
      stroke?: { thickness?: number; fill?: Array<{ color?: string }> };
    };
    expect(rec.stroke?.thickness).toBe(3);
    expect(rec.stroke?.fill?.[0]?.color).toBe('#FFFFFF');
  });

  it('migrates SVG-style stroke attrs into a canonical PenStroke', () => {
    const node = path({
      name: 'HR Chart Line',
      'stroke-width': '2',
      'stroke-dasharray': '4 2',
      'stroke-linecap': 'round',
      'stroke-linejoin': 'round',
    });

    normalizeStrokeFillSchema(node);

    const rec = node as unknown as {
      stroke?: {
        thickness?: number;
        dashPattern?: number[];
        dashOffset?: number;
        cap?: string;
        join?: string;
        fill?: Array<{ color?: string }>;
      };
      'stroke-width'?: unknown;
      'stroke-dasharray'?: unknown;
      'stroke-dashoffset'?: unknown;
    };

    expect(rec.stroke?.thickness).toBe(2);
    expect(rec.stroke?.dashPattern).toEqual([4, 2]);
    expect(rec.stroke?.dashOffset).toBeUndefined();
    expect(rec.stroke?.cap).toBe('round');
    expect(rec.stroke?.join).toBe('round');
    expect(rec.stroke?.fill?.[0]?.color).toBe('#22C55E');
    expect(rec['stroke-width']).toBeUndefined();
    expect(rec['stroke-dasharray']).toBeUndefined();
  });

  it('migrates SVG dash offset into PenStroke.dashOffset', () => {
    const node = path({
      name: 'Steps Progress',
      'stroke-width': '8',
      'stroke-dasharray': '158.4',
      'stroke-dashoffset': '44.4',
      'stroke-linecap': 'round',
    });

    normalizeStrokeFillSchema(node);

    const rec = node as unknown as {
      stroke?: {
        thickness?: number;
        dashPattern?: number[];
        dashOffset?: number;
        cap?: string;
        fill?: Array<{ color?: string }>;
      };
      'stroke-dashoffset'?: unknown;
    };

    expect(rec.stroke?.thickness).toBe(8);
    expect(rec.stroke?.dashPattern).toEqual([158.4, 158.4]);
    expect(rec.stroke?.dashOffset).toBe(44.4);
    expect(rec.stroke?.cap).toBe('round');
    expect(rec.stroke?.fill?.[0]?.color).toBe('#22C55E');
    expect(rec['stroke-dashoffset']).toBeUndefined();
  });

  it('infers a muted track color for SVG-style track paths with no explicit stroke fill', () => {
    const node = path({
      name: 'Steps Track',
      'stroke-width': '8',
      fill: [{ type: 'solid', color: '#00000000' }],
    });

    normalizeStrokeFillSchema(node);

    const rec = node as unknown as {
      stroke?: { fill?: Array<{ color?: string }> };
    };

    expect(rec.stroke?.fill?.[0]?.color).toBe('#2A2A2A');
  });
});

describe('normalizeStrokeFillSchema — illegal fill color replacement', () => {
  it('replaces a sole "none" fill with explicit transparent hex', () => {
    // “none”和“transparent”是 CSS 关键字，不是有效的 PenFill 颜色。 But
    // 它们表达了明确的“无填充”意图（例如，应该是空心的活动环）。 Deleting 填充字段完全会使画布渲染器回退到默认的灰色填充 - 与
    // AI 的含义相反。 Replace 具有 8 位透明十六进制，因此渲染器会看到明确的透明颜色并尊重它。
    const node = path({
      fill: [{ type: 'solid', color: 'none' }],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { fill?: Array<{ type?: string; color?: string }> };
    expect(rec.fill).toBeDefined();
    expect(rec.fill).toHaveLength(1);
    expect(rec.fill?.[0]?.type).toBe('solid');
    expect(rec.fill?.[0]?.color?.toLowerCase()).toBe('#00000000');
  });

  it('replaces a sole "transparent" fill with explicit transparent hex', () => {
    const node = path({
      fill: [{ type: 'solid', color: 'transparent' }],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { fill?: Array<{ type?: string; color?: string }> };
    expect(rec.fill).toBeDefined();
    expect(rec.fill).toHaveLength(1);
    expect(rec.fill?.[0]?.color?.toLowerCase()).toBe('#00000000');
  });

  it('drops a "none" entry but keeps other legitimate entries in the same array', () => {
    // Mixed 数组：1 个非法，1 个合法。 Just 丢弃非法颜色并保留真实颜色 - 无需透明后备。
    const node = path({
      fill: [
        { type: 'solid', color: 'none' },
        { type: 'solid', color: '#FF0000' },
      ],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { fill?: Array<{ color?: string }> };
    expect(rec.fill).toHaveLength(1);
    expect(rec.fill?.[0]?.color).toBe('#FF0000');
  });

  it('keeps legitimate hex fills alone', () => {
    const node = path({
      fill: [{ type: 'solid', color: '#C4F82A' }],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { fill?: Array<{ color?: string }> };
    expect(rec.fill?.[0]?.color).toBe('#C4F82A');
  });

  it('keeps 8-digit transparent hex (#00000000) alone', () => {
    // The 8 位十六进制 IS 有效颜色字符串（Alpha 通道）。 Only CSS
    // 关键字“none”和“transparent”是不受支持的形式。
    const node = ellipse({
      fill: [{ type: 'solid', color: '#00000000' }],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { fill?: Array<{ color?: string }> };
    expect(rec.fill?.[0]?.color).toBe('#00000000');
  });

  it('DELETES text node fill when all entries are CSS keywords', () => {
    // text.fill 是前景色，而不是形状背景。文本节点上的“无”/“透明”将完全隐藏文本 - 几乎肯定是一个错误。
    // Delete 该字段，以便下游层（角色默认值、按钮对比度启发式）可以填充可见颜色。 Replacing 与 #00000000
    // 会将文本冻结为不可见。
    const node = text({
      fill: [{ type: 'solid', color: 'none' }],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { fill?: unknown };
    expect(rec.fill).toBeUndefined();
  });

  it('DELETES text node fill for "transparent" too', () => {
    const node = text({
      fill: [{ type: 'solid', color: 'transparent' }],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { fill?: unknown };
    expect(rec.fill).toBeUndefined();
  });

  it('DELETES icon_font fill when all entries are CSS keywords', () => {
    // Same 推理：icon_font.fill 是前景色。
    const node = iconFont({
      fill: [{ type: 'solid', color: 'none' }],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { fill?: unknown };
    expect(rec.fill).toBeUndefined();
  });

  it('still keeps a legitimate text fill unchanged', () => {
    const node = text({
      fill: [{ type: 'solid', color: '#FFFFFF' }],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { fill?: Array<{ color?: string }> };
    expect(rec.fill?.[0]?.color).toBe('#FFFFFF');
  });

  it('still drops illegal entries from text but preserves legal siblings', () => {
    const node = text({
      fill: [
        { type: 'solid', color: 'none' },
        { type: 'solid', color: '#00FF00' },
      ],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { fill?: Array<{ color?: string }> };
    expect(rec.fill).toHaveLength(1);
    expect(rec.fill?.[0]?.color).toBe('#00FF00');
  });

  it('keeps the transparent-hex replacement for shape fills (frame / ellipse / path)', () => {
    // Sanity 防护：形状与前景的分割不得使形状分支回归。当 AI 写入“none”时，Frames、省略号和路径仍然得到显式的
    // #00000000。
    for (const node of [
      frame({ fill: [{ type: 'solid', color: 'none' }] }),
      ellipse({ fill: [{ type: 'solid', color: 'none' }] }),
      path({ fill: [{ type: 'solid', color: 'transparent' }] }),
    ]) {
      normalizeStrokeFillSchema(node);
      const rec = node as unknown as { fill?: Array<{ color?: string }> };
      expect(rec.fill).toHaveLength(1);
      expect(rec.fill?.[0]?.color?.toLowerCase()).toBe('#00000000');
    }
  });

  it('also strips illegal colors from stroke.fill arrays', () => {
    const node = path({
      stroke: { thickness: 2, fill: [{ type: 'solid', color: 'none' }] },
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { stroke?: { thickness?: number; fill?: unknown[] } };
    // Whole 描边要么未设置，要么描边.fill 为空——无论哪种方式，渲染器都不会尝试使用“none”。
    if (rec.stroke && Array.isArray(rec.stroke.fill)) {
      expect(rec.stroke.fill.length).toBe(0);
    }
  });
});

describe('normalizeStrokeFillSchema — recursion', () => {
  it('recurses into children', () => {
    const inner = ellipse({
      id: 'inner',
      stroke: [{ thickness: 8, fill: validFill }],
    });
    const root = frame({ id: 'root', children: [inner] });
    normalizeStrokeFillSchema(root);
    const rec = inner as unknown as { stroke?: { thickness?: number } };
    expect(Array.isArray((inner as unknown as { stroke?: unknown }).stroke)).toBe(false);
    expect(rec.stroke?.thickness).toBe(8);
  });

  it('reproduces the M2.7 activity rings case end-to-end', () => {
    const ring = ellipse({
      id: 'ring',
      name: 'Steps Circle',
      width: 100,
      height: 100,
      fill: [{ type: 'solid', color: '#00000000' }],
      stroke: [{ thickness: 12, fill: [{ type: 'solid', color: '#C4F82A' }] }],
    });
    normalizeStrokeFillSchema(ring);
    const rec = ring as unknown as {
      fill?: Array<{ color?: string }>;
      stroke?: { thickness?: number; fill?: Array<{ color?: string }> };
    };
    // Stroke 现在是一个具有原始厚度和颜色的正确对象
    expect(rec.stroke?.thickness).toBe(12);
    expect(rec.stroke?.fill?.[0]?.color).toBe('#C4F82A');
    // Transparent 8 位十六进制填充被保留（它不是 CSS 关键字）
    expect(rec.fill?.[0]?.color).toBe('#00000000');
  });

  it('reproduces the M2.7 heart-rate chart line case end-to-end', () => {
    const line = path({
      id: 'line',
      name: 'Chart Line',
      d: 'M0 50 L100 20',
      fill: [{ type: 'solid', color: 'none' }],
      stroke: [{ type: 'solid', color: '#C4F82A' }],
      strokeWidth: 2.5,
    });
    normalizeStrokeFillSchema(line);
    const rec = line as unknown as {
      fill?: Array<{ color?: string }>;
      stroke?: { thickness?: number; fill?: Array<{ color?: string }> };
      strokeWidth?: number;
    };
    expect(rec.stroke?.thickness).toBe(2.5);
    expect(rec.stroke?.fill?.[0]?.color).toBe('#C4F82A');
    // “无”填充被替换为显式透明 — NOT 被删除。 Deleting 将使画布工厂回退到默认的灰色填充，并且图表线的背景将渗透为纯灰
    // 色矩形。
    expect(rec.fill).toBeDefined();
    expect(rec.fill?.[0]?.color?.toLowerCase()).toBe('#00000000');
    expect(rec.strokeWidth).toBeUndefined();
  });
});
