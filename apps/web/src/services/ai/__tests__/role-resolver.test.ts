import { describe, it, expect, vi } from 'vitest';

// Mock canvas-text-measure 以避免测试中 CanvasKit WASM 依赖
vi.mock('@/canvas/canvas-text-measure', () => ({
  estimateLineWidth: () => 0,
  estimateTextHeight: () => 0,
  defaultLineHeight: () => 1.2,
  hasCjkText: () => false,
}));

import {
  hexLuminance,
  hasFill,
  hasVisibleFill,
  resolveNodeRole,
  resolveTreePostPass,
  resolveTreeRoles,
} from '../role-resolver';
import type { RoleContext } from '../role-resolver';
import type { PenNode } from '@zseven-w/pen-types';

// Ensure role definitions are registered
import '../role-definitions/index';

describe('hexLuminance', () => {
  it('returns 0 for black', () => {
    expect(hexLuminance('#000000')).toBeCloseTo(0, 2);
  });

  it('returns 1 for white', () => {
    expect(hexLuminance('#FFFFFF')).toBeCloseTo(1, 2);
  });

  it('returns ~0.5 for mid-gray', () => {
    const lum = hexLuminance('#808080');
    expect(lum).toBeGreaterThan(0.2);
    expect(lum).toBeLessThan(0.6);
  });

  it('handles lowercase hex', () => {
    expect(hexLuminance('#ffffff')).toBeCloseTo(1, 2);
  });

  it('handles 8-digit hex (with alpha)', () => {
    expect(hexLuminance('#000000FF')).toBeCloseTo(0, 2);
  });

  it('returns < 0.5 for dark blue (#2563EB)', () => {
    expect(hexLuminance('#2563EB')).toBeLessThan(0.5);
  });

  it('returns > 0.5 for light gray (#F8FAFC)', () => {
    expect(hexLuminance('#F8FAFC')).toBeGreaterThan(0.5);
  });
});

describe('hasFill', () => {
  // hasFill 回答“AI 是否声明了任何填充条目？” — 它由后通道启发法使用，必须 NOT 覆盖显式选择的填充，即使该填充是透明的。
  // Use `hasVisibleFill` 当您需要“这会绘制可见颜色吗？”时。

  it('returns false for node without fill', () => {
    const node = { id: 'n1', type: 'frame', x: 0, y: 0, width: 100, height: 100 } as PenNode;
    expect(hasFill(node)).toBe(false);
  });

  it('returns false for empty fill array', () => {
    const node = {
      id: 'n1',
      type: 'frame',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      fill: [],
    } as PenNode;
    expect(hasFill(node)).toBe(false);
  });

  it('returns true for node with solid fill', () => {
    const node = {
      id: 'n1',
      type: 'frame',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      fill: [{ type: 'solid', color: '#FFFFFF' }],
    } as PenNode;
    expect(hasFill(node)).toBe(true);
  });

  // hasFill 必须将透明填充报告为“已填充”，以便覆盖保护调用者（fixOrphanContainerContrast、fixSecti
  // onAlternation）不理会它们。 AI 作者明确设置 fill=#00000000 的框架正在做出故意的无背景选择。
  it('returns true for explicit-transparent hex (#00000000) — overwrite protection', () => {
    const node = {
      id: 'n',
      type: 'frame',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      fill: [{ type: 'solid', color: '#00000000' }],
    } as PenNode;
    expect(hasFill(node)).toBe(true);
  });

  it('returns true for CSS keyword "transparent"', () => {
    const node = {
      id: 'n',
      type: 'frame',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      fill: [{ type: 'solid', color: 'transparent' }],
    } as PenNode;
    expect(hasFill(node)).toBe(true);
  });

  it('returns true for opacity-0 fill — still a deliberate author choice', () => {
    // opacity: 0 是一个合法的“我想要透明背景”声明。 hasFill
    // 的存在是为了阻止后通过启发法覆盖此类选择，因此它必须保持报告真实。
    const node = {
      id: 'n',
      type: 'frame',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      fill: [{ type: 'solid', color: '#FFFFFF', opacity: 0 }],
    } as PenNode;
    expect(hasFill(node)).toBe(true);
  });
});

describe('hasVisibleFill', () => {
  // hasVisibleFill 回答“这会在屏幕上绘制可见的颜色吗？”。 Used 通过按钮前景对比度传递来决定子节点是否需要提供颜色。
  // Transparent 填充必须报告为 false，以便对比度可以绘制可见的前景色。

  it('returns false for node without fill', () => {
    const node = { id: 'n', type: 'frame', x: 0, y: 0, width: 100, height: 100 } as PenNode;
    expect(hasVisibleFill(node)).toBe(false);
  });

  it('returns false for empty fill array', () => {
    const node = {
      id: 'n',
      type: 'frame',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      fill: [],
    } as PenNode;
    expect(hasVisibleFill(node)).toBe(false);
  });

  it('returns true for node with opaque solid fill', () => {
    const node = {
      id: 'n',
      type: 'frame',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      fill: [{ type: 'solid', color: '#FFFFFF' }],
    } as PenNode;
    expect(hasVisibleFill(node)).toBe(true);
  });

  it('returns false for 8-digit transparent hex (#00000000)', () => {
    const node = {
      id: 'p',
      type: 'path',
      x: 0,
      y: 0,
      width: 24,
      height: 24,
      fill: [{ type: 'solid', color: '#00000000' }],
    } as PenNode;
    expect(hasVisibleFill(node)).toBe(false);
  });

  it('returns false for any 8-digit hex with 00 alpha', () => {
    const node = {
      id: 'p',
      type: 'path',
      x: 0,
      y: 0,
      width: 24,
      height: 24,
      fill: [{ type: 'solid', color: '#FF00FF00' }],
    } as PenNode;
    expect(hasVisibleFill(node)).toBe(false);
  });

  it('returns false for CSS keyword "transparent"', () => {
    const node = {
      id: 'p',
      type: 'path',
      x: 0,
      y: 0,
      width: 24,
      height: 24,
      fill: [{ type: 'solid', color: 'transparent' }],
    } as PenNode;
    expect(hasVisibleFill(node)).toBe(false);
  });

  it('returns false for CSS keyword "none"', () => {
    const node = {
      id: 'p',
      type: 'path',
      x: 0,
      y: 0,
      width: 24,
      height: 24,
      fill: [{ type: 'solid', color: 'none' }],
    } as PenNode;
    expect(hasVisibleFill(node)).toBe(false);
  });

  it('returns true for a partially-transparent 8-digit hex (non-zero alpha)', () => {
    const node = {
      id: 'n',
      type: 'frame',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      fill: [{ type: 'solid', color: '#FF000080' }],
    } as PenNode;
    expect(hasVisibleFill(node)).toBe(true);
  });

  // --- 不透明字段处理 --------------------------------------- PenFill
  // 变体都带有可选的 `opacity` 字段。不透明度=0 的实心填充呈现为完全透明，无论其颜色十六进制如何，下游对比度逻辑必须将其视为“
  // 不可见填充”，以便仍然可以提供前景色。
  it('returns false for a solid fill with opacity: 0', () => {
    const node = {
      id: 'n',
      type: 'frame',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      fill: [{ type: 'solid', color: '#FFFFFF', opacity: 0 }],
    } as PenNode;
    expect(hasVisibleFill(node)).toBe(false);
  });

  it('returns false for a linear gradient fill with opacity: 0', () => {
    const node = {
      id: 'n',
      type: 'frame',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      fill: [
        {
          type: 'linear_gradient',
          angle: 90,
          stops: [
            { offset: 0, color: '#FF0000' },
            { offset: 1, color: '#0000FF' },
          ],
          opacity: 0,
        },
      ],
    } as unknown as PenNode;
    expect(hasVisibleFill(node)).toBe(false);
  });

  it('returns false for negative opacity (treated as zero)', () => {
    const node = {
      id: 'n',
      type: 'frame',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      fill: [{ type: 'solid', color: '#FFFFFF', opacity: -0.5 }],
    } as PenNode;
    expect(hasVisibleFill(node)).toBe(false);
  });

  it('returns true for a solid fill with opacity: 0.5', () => {
    const node = {
      id: 'n',
      type: 'frame',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      fill: [{ type: 'solid', color: '#FFFFFF', opacity: 0.5 }],
    } as PenNode;
    expect(hasVisibleFill(node)).toBe(true);
  });

  it('returns true for a solid fill without an opacity field (default opaque)', () => {
    const node = {
      id: 'n',
      type: 'frame',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      fill: [{ type: 'solid', color: '#FFFFFF' }],
    } as PenNode;
    expect(hasVisibleFill(node)).toBe(true);
  });
});

describe('resolveTreePostPass — transparent fill overwrite protection', () => {
  // Regression：DON 不想覆盖显式填充选择的后通道试探必须尊重透明填充，就像尊重任何其他颜色一样。 Only
  // 按钮对比度通道需要将透明视为“无可见填充”。

  it('fixOrphanContainerContrast does NOT overwrite a card with opacity: 0 fill', () => {
    const card: PenNode = {
      id: 'card',
      type: 'frame',
      name: 'Opacity Zero Card',
      x: 0,
      y: 0,
      width: 300,
      height: 200,
      cornerRadius: 12,
      fill: [{ type: 'solid', color: '#FFFFFF', opacity: 0 }],
      children: [
        {
          id: 'txt',
          type: 'text',
          name: 'Title',
          x: 0,
          y: 0,
          width: 200,
          height: 20,
          content: 'Hello',
        } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 1200,
      height: 800,
      children: [card],
    } as PenNode;
    resolveTreePostPass(root, 1200);
    expect((card as any).fill).toEqual([{ type: 'solid', color: '#FFFFFF', opacity: 0 }]);
    expect((card as any).effects).toBeUndefined();
  });

  it('fixOrphanContainerContrast does NOT overwrite a card with explicit transparent fill', () => {
    // An 作者用 cornerRadius + 孩子写了一张卡片，但故意选择了透明背景。 The
    // 孤儿对比通道不得突然将其涂成白色并添加阴影。
    const card: PenNode = {
      id: 'card',
      type: 'frame',
      name: 'Hollow Card',
      x: 0,
      y: 0,
      width: 300,
      height: 200,
      cornerRadius: 12,
      fill: [{ type: 'solid', color: '#00000000' }],
      children: [
        {
          id: 'txt',
          type: 'text',
          name: 'Title',
          x: 0,
          y: 0,
          width: 200,
          height: 20,
          content: 'Hello',
        } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 1200,
      height: 800,
      children: [card],
    } as PenNode;
    resolveTreePostPass(root, 1200);
    // Fill 正是作者设置的——而不是#FFFFFF。
    expect((card as any).fill).toEqual([{ type: 'solid', color: '#00000000' }]);
    // 添加了 No 阴影。
    expect((card as any).effects).toBeUndefined();
  });

  it('fixSectionAlternation does NOT repaint a section with explicit transparent fill', () => {
    const children = [
      {
        id: 's0',
        type: 'frame' as const,
        name: 'Hero',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'hero',
        layout: 'vertical' as const,
        children: [],
        fill: [{ type: 'solid', color: '#00000000' }],
      },
      {
        id: 's1',
        type: 'frame' as const,
        name: 'Features',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'section',
        layout: 'vertical' as const,
        children: [],
      },
      {
        id: 's2',
        type: 'frame' as const,
        name: 'CTA',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'cta-section',
        layout: 'vertical' as const,
        children: [],
      },
      {
        id: 's3',
        type: 'frame' as const,
        name: 'Footer',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'footer',
        layout: 'vertical' as const,
        children: [],
      },
    ] as PenNode[];
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 1200,
      height: 1600,
      layout: 'vertical',
      children,
    } as PenNode;
    resolveTreePostPass(root, 1200);
    // The 透明英雄保持透明 - 交替不会覆盖任何颜色的显式填充。
    expect((children[0] as any).fill).toEqual([{ type: 'solid', color: '#00000000' }]);
  });
});

describe('resolveTreePostPass — button foreground contrast with transparent-hex path icon', () => {
  // The 真正的失败，normalizeStrokeFillSchema 修复必须解决：按钮内 AI 生成的笔划样式线条图标，其中
  // AI 写入 `fill: [{color: "none"}]`，规范化器替换 `#00000000` 以保留空洞意图。 The
  // 按钮对比通道仍必须看到“无可见填充”并提供可见的描边颜色。
  it('paints stroke on a path icon whose fill is 8-digit transparent hex', () => {
    const button: PenNode = {
      id: 'btn',
      type: 'frame',
      name: 'Icon Button',
      x: 0,
      y: 0,
      width: 44,
      height: 44,
      role: 'icon-button',
      fill: [{ type: 'solid', color: '#1E293B' }],
      children: [
        {
          id: 'p',
          type: 'path',
          name: 'Arrow',
          x: 0,
          y: 0,
          width: 24,
          height: 24,
          fill: [{ type: 'solid', color: '#00000000' }],
          stroke: { thickness: 2 },
        } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 375,
      height: 812,
      children: [button],
    } as PenNode;
    resolveTreePostPass(root, 375);
    const p = (root as any).children[0].children[0];
    expect(p.stroke.fill).toEqual([{ type: 'solid', color: '#FFFFFF' }]);
  });

  it('paints fill on a path icon whose only fill is transparent and has no stroke', () => {
    const button: PenNode = {
      id: 'btn',
      type: 'frame',
      name: 'Icon Button',
      x: 0,
      y: 0,
      width: 44,
      height: 44,
      role: 'icon-button',
      fill: [{ type: 'solid', color: '#2563EB' }],
      children: [
        {
          id: 'p',
          type: 'path',
          name: 'Star',
          x: 0,
          y: 0,
          width: 24,
          height: 24,
          fill: [{ type: 'solid', color: '#00000000' }],
        } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 375,
      height: 812,
      children: [button],
    } as PenNode;
    resolveTreePostPass(root, 375);
    const p = (root as any).children[0].children[0];
    expect(p.fill).toEqual([{ type: 'solid', color: '#FFFFFF' }]);
  });
});

describe('resolveTreePostPass — button foreground contrast', () => {
  it('sets white text on dark button', () => {
    const button: PenNode = {
      id: 'btn',
      type: 'frame',
      name: 'Button',
      x: 0,
      y: 0,
      width: 120,
      height: 44,
      role: 'button',
      fill: [{ type: 'solid', color: '#2563EB' }],
      children: [
        {
          id: 'txt',
          type: 'text',
          name: 'Label',
          x: 0,
          y: 0,
          width: 80,
          height: 20,
          content: 'Sign In',
        } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 375,
      height: 812,
      children: [button],
    } as PenNode;
    resolveTreePostPass(root, 375);
    const txt = (root as any).children[0].children[0];
    expect(txt.fill).toEqual([{ type: 'solid', color: '#FFFFFF' }]);
  });

  it('sets dark text on light button', () => {
    const button: PenNode = {
      id: 'btn',
      type: 'frame',
      name: 'Button',
      x: 0,
      y: 0,
      width: 120,
      height: 44,
      role: 'button',
      fill: [{ type: 'solid', color: '#DBEAFE' }],
      children: [
        {
          id: 'txt',
          type: 'text',
          name: 'Label',
          x: 0,
          y: 0,
          width: 80,
          height: 20,
          content: 'Sign In',
        } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 375,
      height: 812,
      children: [button],
    } as PenNode;
    resolveTreePostPass(root, 375);
    const txt = (root as any).children[0].children[0];
    expect(txt.fill).toEqual([{ type: 'solid', color: '#0F172A' }]);
  });

  it('does not overwrite explicit text fill', () => {
    const button: PenNode = {
      id: 'btn',
      type: 'frame',
      name: 'Button',
      x: 0,
      y: 0,
      width: 120,
      height: 44,
      role: 'button',
      fill: [{ type: 'solid', color: '#2563EB' }],
      children: [
        {
          id: 'txt',
          type: 'text',
          name: 'Label',
          x: 0,
          y: 0,
          width: 80,
          height: 20,
          content: 'Sign In',
          fill: [{ type: 'solid', color: '#FDE047' }],
        } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 375,
      height: 812,
      children: [button],
    } as PenNode;
    resolveTreePostPass(root, 375);
    const txt = (root as any).children[0].children[0];
    expect(txt.fill).toEqual([{ type: 'solid', color: '#FDE047' }]);
  });

  it('sets fill on icon_font child in dark button', () => {
    const button: PenNode = {
      id: 'btn',
      type: 'frame',
      name: 'Button',
      x: 0,
      y: 0,
      width: 44,
      height: 44,
      role: 'icon-button',
      fill: [{ type: 'solid', color: '#1E293B' }],
      children: [
        {
          id: 'ico',
          type: 'icon_font',
          name: 'Icon',
          x: 0,
          y: 0,
          width: 24,
          height: 24,
        } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 375,
      height: 812,
      children: [button],
    } as PenNode;
    resolveTreePostPass(root, 375);
    const ico = (root as any).children[0].children[0];
    expect(ico.fill).toEqual([{ type: 'solid', color: '#FFFFFF' }]);
  });

  it('sets stroke.fill on stroke-style path in dark button', () => {
    const button: PenNode = {
      id: 'btn',
      type: 'frame',
      name: 'Button',
      x: 0,
      y: 0,
      width: 44,
      height: 44,
      role: 'button',
      fill: [{ type: 'solid', color: '#2563EB' }],
      children: [
        {
          id: 'p',
          type: 'path',
          name: 'Arrow',
          x: 0,
          y: 0,
          width: 24,
          height: 24,
          stroke: { thickness: 2 },
        } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 375,
      height: 812,
      children: [button],
    } as PenNode;
    resolveTreePostPass(root, 375);
    const p = (root as any).children[0].children[0];
    expect(p.stroke.fill).toEqual([{ type: 'solid', color: '#FFFFFF' }]);
  });

  it('sets fill on unstyled path (no stroke, no fill) in dark button', () => {
    const button: PenNode = {
      id: 'btn',
      type: 'frame',
      name: 'Button',
      x: 0,
      y: 0,
      width: 44,
      height: 44,
      role: 'button',
      fill: [{ type: 'solid', color: '#2563EB' }],
      children: [
        { id: 'p', type: 'path', name: 'Arrow', x: 0, y: 0, width: 24, height: 24 } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 375,
      height: 812,
      children: [button],
    } as PenNode;
    resolveTreePostPass(root, 375);
    const p = (root as any).children[0].children[0];
    expect(p.fill).toEqual([{ type: 'solid', color: '#FFFFFF' }]);
  });
});

describe('resolveTreePostPass — section background alternation', () => {
  it('alternates fills on 3+ consecutive unfilled sections', () => {
    const children = [
      {
        id: 's0',
        type: 'frame' as const,
        name: 'Hero',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'hero',
        layout: 'vertical' as const,
        children: [],
      },
      {
        id: 's1',
        type: 'frame' as const,
        name: 'Features',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'section',
        layout: 'vertical' as const,
        children: [],
      },
      {
        id: 's2',
        type: 'frame' as const,
        name: 'CTA',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'cta-section',
        layout: 'vertical' as const,
        children: [],
      },
    ] as PenNode[];
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 1200,
      height: 2400,
      layout: 'vertical',
      children,
    } as PenNode;
    resolveTreePostPass(root, 1200);
    expect((children[0] as any).fill).toEqual([{ type: 'solid', color: '#FFFFFF' }]);
    expect((children[1] as any).fill).toEqual([{ type: 'solid', color: '#F8FAFC' }]);
    expect((children[2] as any).fill).toEqual([{ type: 'solid', color: '#FFFFFF' }]);
  });

  it('only alternates within contiguous runs — non-section children break the run', () => {
    const children = [
      {
        id: 's0',
        type: 'frame' as const,
        name: 'Hero',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'hero',
        layout: 'vertical' as const,
        children: [],
      },
      {
        id: 's1',
        type: 'frame' as const,
        name: 'Features',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'section',
        layout: 'vertical' as const,
        children: [],
      },
      {
        id: 'card',
        type: 'frame' as const,
        name: 'Card',
        x: 0,
        y: 0,
        width: 300,
        height: 200,
        role: 'card',
        children: [],
      },
      {
        id: 's2',
        type: 'frame' as const,
        name: 'Footer',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'footer',
        layout: 'vertical' as const,
        children: [],
      },
      {
        id: 's3',
        type: 'frame' as const,
        name: 'Section2',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'section',
        layout: 'vertical' as const,
        children: [],
      },
    ] as PenNode[];
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 1200,
      height: 3000,
      layout: 'vertical',
      children,
    } as PenNode;
    resolveTreePostPass(root, 1200);
    expect((children[0] as any).fill).toBeUndefined();
    expect((children[1] as any).fill).toBeUndefined();
    expect((children[3] as any).fill).toBeUndefined();
    expect((children[4] as any).fill).toBeUndefined();
  });

  it('skips sections with existing fills', () => {
    const children = [
      {
        id: 's0',
        type: 'frame' as const,
        name: 'Hero',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'hero',
        layout: 'vertical' as const,
        fill: [{ type: 'solid', color: '#1E293B' }],
        children: [],
      },
      {
        id: 's1',
        type: 'frame' as const,
        name: 'Features',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'section',
        layout: 'vertical' as const,
        children: [],
      },
      {
        id: 's2',
        type: 'frame' as const,
        name: 'Footer',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'footer',
        layout: 'vertical' as const,
        children: [],
      },
    ] as PenNode[];
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 1200,
      height: 2400,
      layout: 'vertical',
      children,
    } as PenNode;
    resolveTreePostPass(root, 1200);
    expect((children[0] as any).fill).toEqual([{ type: 'solid', color: '#1E293B' }]);
    expect((children[1] as any).fill).toBeUndefined();
  });

  it('does nothing with fewer than 3 consecutive sections', () => {
    const children = [
      {
        id: 's0',
        type: 'frame' as const,
        name: 'Hero',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'hero',
        layout: 'vertical' as const,
        children: [],
      },
      {
        id: 's1',
        type: 'frame' as const,
        name: 'Footer',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'footer',
        layout: 'vertical' as const,
        children: [],
      },
    ] as PenNode[];
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 1200,
      height: 1200,
      layout: 'vertical',
      children,
    } as PenNode;
    resolveTreePostPass(root, 1200);
    expect((children[0] as any).fill).toBeUndefined();
    expect((children[1] as any).fill).toBeUndefined();
  });

  // 2026 年 4 月 15 日的 Regression 防护：当 design.md 强制使用深色 rootFrame 时，硬编码的
  // #FFFFFF/#F8FAFC 交替在深色页面背景上绘制可见的白色条带。 On 深色页面部分必须保持透明 -
  // 内部卡片对比度已经在视觉上将它们分组。
  it('does NOT alternate on a dark-themed parent (luminance < 0.5)', () => {
    const children = [
      {
        id: 's0',
        type: 'frame' as const,
        name: 'Hero',
        x: 0,
        y: 0,
        width: 375,
        height: 400,
        role: 'hero',
        layout: 'vertical' as const,
        children: [],
      },
      {
        id: 's1',
        type: 'frame' as const,
        name: 'Stats',
        x: 0,
        y: 0,
        width: 375,
        height: 400,
        role: 'stats-section',
        layout: 'vertical' as const,
        children: [],
      },
      {
        id: 's2',
        type: 'frame' as const,
        name: 'CTA',
        x: 0,
        y: 0,
        width: 375,
        height: 400,
        role: 'cta-section',
        layout: 'vertical' as const,
        children: [],
      },
    ] as PenNode[];
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 375,
      height: 1200,
      layout: 'vertical',
      fill: [{ type: 'solid', color: '#111111' }],
      children,
    } as PenNode;
    resolveTreePostPass(root, 375);
    expect((children[0] as any).fill).toBeUndefined();
    expect((children[1] as any).fill).toBeUndefined();
    expect((children[2] as any).fill).toBeUndefined();
  });
});

describe('resolveTreePostPass — orphan container contrast', () => {
  it('adds fill + shadow to untagged rounded frame when parent has no fill', () => {
    const card: PenNode = {
      id: 'card',
      type: 'frame',
      name: 'Card',
      x: 0,
      y: 0,
      width: 300,
      height: 200,
      cornerRadius: 12,
      children: [
        {
          id: 'txt',
          type: 'text',
          name: 'Title',
          x: 0,
          y: 0,
          width: 200,
          height: 20,
          content: 'Hello',
        } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 1200,
      height: 800,
      children: [card],
    } as PenNode;
    resolveTreePostPass(root, 1200);
    expect((card as any).fill).toEqual([{ type: 'solid', color: '#FFFFFF' }]);
    expect((card as any).effects).toHaveLength(2);
    expect((card as any).effects[0].type).toBe('shadow');
  });

  it('does not apply to structural roles like section', () => {
    const section: PenNode = {
      id: 'sec',
      type: 'frame',
      name: 'Section',
      x: 0,
      y: 0,
      width: 1200,
      height: 400,
      role: 'section',
      cornerRadius: 12,
      children: [
        {
          id: 'txt',
          type: 'text',
          name: 'Title',
          x: 0,
          y: 0,
          width: 200,
          height: 20,
          content: 'Hello',
        } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 1200,
      height: 800,
      children: [section],
    } as PenNode;
    resolveTreePostPass(root, 1200);
    expect((section as any).fill).toBeUndefined();
  });

  it('does not apply when parent has fill', () => {
    const card: PenNode = {
      id: 'card',
      type: 'frame',
      name: 'Card',
      x: 0,
      y: 0,
      width: 300,
      height: 200,
      cornerRadius: 12,
      children: [
        {
          id: 'txt',
          type: 'text',
          name: 'Title',
          x: 0,
          y: 0,
          width: 200,
          height: 20,
          content: 'Hello',
        } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 1200,
      height: 800,
      fill: [{ type: 'solid', color: '#F8FAFC' }],
      children: [card],
    } as PenNode;
    resolveTreePostPass(root, 1200);
    expect((card as any).fill).toBeUndefined();
  });

  it('does not apply to empty frames', () => {
    const empty: PenNode = {
      id: 'e',
      type: 'frame',
      name: 'Empty',
      x: 0,
      y: 0,
      width: 300,
      height: 200,
      cornerRadius: 12,
      children: [],
    } as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 1200,
      height: 800,
      children: [empty],
    } as PenNode;
    resolveTreePostPass(root, 1200);
    expect((empty as any).fill).toBeUndefined();
  });
});

describe('resolveTreePostPass — input sibling consistency', () => {
  it('propagates first input fill/stroke to mismatched siblings', () => {
    const input1: PenNode = {
      id: 'i1',
      type: 'frame',
      name: 'Email',
      x: 0,
      y: 0,
      width: 300,
      height: 48,
      role: 'form-input',
      fill: [{ type: 'solid', color: '#E0F2FE' }],
      stroke: { thickness: 1, fill: [{ type: 'solid', color: '#0EA5E9' }] },
    } as unknown as PenNode;
    const input2: PenNode = {
      id: 'i2',
      type: 'frame',
      name: 'Password',
      x: 0,
      y: 0,
      width: 300,
      height: 48,
      role: 'form-input',
      fill: [{ type: 'solid', color: '#F8FAFC' }],
      stroke: { thickness: 1, fill: [{ type: 'solid', color: '#E2E8F0' }] },
    } as unknown as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Form',
      x: 0,
      y: 0,
      width: 400,
      height: 200,
      layout: 'vertical',
      children: [input1, input2],
    } as PenNode;
    resolveTreePostPass(root, 375);
    expect((input2 as any).fill).toEqual([{ type: 'solid', color: '#E0F2FE' }]);
    expect((input2 as any).stroke.fill).toEqual([{ type: 'solid', color: '#0EA5E9' }]);
  });

  it('skips when all inputs already match', () => {
    const fill = [{ type: 'solid' as const, color: '#F8FAFC' }];
    const stroke = { thickness: 1, fill: [{ type: 'solid' as const, color: '#E2E8F0' }] };
    const input1: PenNode = {
      id: 'i1',
      type: 'frame',
      name: 'Email',
      x: 0,
      y: 0,
      width: 300,
      height: 48,
      role: 'input',
      fill: [...fill],
      stroke: { ...stroke },
    } as unknown as PenNode;
    const input2: PenNode = {
      id: 'i2',
      type: 'frame',
      name: 'Password',
      x: 0,
      y: 0,
      width: 300,
      height: 48,
      role: 'input',
      fill: [...fill],
      stroke: { ...stroke },
    } as unknown as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Form',
      x: 0,
      y: 0,
      width: 400,
      height: 200,
      layout: 'vertical',
      children: [input1, input2],
    } as PenNode;
    resolveTreePostPass(root, 375);
    expect((input1 as any).fill[0].color).toBe('#F8FAFC');
    expect((input2 as any).fill[0].color).toBe('#F8FAFC');
  });
});

// ---------------------------------------------------------------------------
// 基于 Name 的角色推理
// ---------------------------------------------------------------------------

describe('resolveNodeRole — name-based role inference', () => {
  const ctx: RoleContext = { canvasWidth: 375 };

  it('infers button role from name "Sign In Button"', () => {
    const node = {
      id: 'b',
      type: 'frame',
      name: 'Sign In Button',
      x: 0,
      y: 0,
      width: 120,
      height: 44,
    } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBe('button');
    // Button 角色应该 NOT 注入硬编码的填充颜色 - 填充来自 AI/design 系统
    expect((node as any).fill).toBeUndefined();
  });

  it('does not infer button role for container names like "Button Group"', () => {
    const node = {
      id: 'bg',
      type: 'frame',
      name: 'Button Group',
      x: 0,
      y: 0,
      width: 300,
      height: 60,
    } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBeUndefined();
  });

  it('does not infer button role for container names like "Buttons Row"', () => {
    const node = {
      id: 'br',
      type: 'frame',
      name: 'Buttons Row',
      x: 0,
      y: 0,
      width: 300,
      height: 60,
    } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBeUndefined();
  });

  it('infers card role from name "Restaurant Card"', () => {
    const node = {
      id: 'c',
      type: 'frame',
      name: 'Restaurant Card',
      x: 0,
      y: 0,
      width: 300,
      height: 200,
    } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBe('card');
    expect((node as any).fill).toBeDefined();
    expect((node as any).effects).toHaveLength(2);
  });

  it('infers input role from name "Email Input"', () => {
    const node = {
      id: 'i',
      type: 'frame',
      name: 'Email Input',
      x: 0,
      y: 0,
      width: 300,
      height: 48,
    } as PenNode;
    resolveNodeRole(node, { ...ctx, parentLayout: 'vertical' });
    expect(node.role).toBe('input');
    expect((node as any).fill[0].color).toBe('#F8FAFC');
    expect((node as any).stroke).toBeDefined();
  });

  it('infers navbar from exact name "Navigation"', () => {
    const node = {
      id: 'n',
      type: 'frame',
      name: 'Navigation',
      x: 0,
      y: 0,
      width: 375,
      height: 56,
    } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBe('navbar');
    expect((node as any).fill[0].color).toBe('#FFFFFF');
  });

  it('infers search-bar from name "Search"', () => {
    const node = {
      id: 's',
      type: 'frame',
      name: 'Search',
      x: 0,
      y: 0,
      width: 300,
      height: 44,
    } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBe('search-bar');
  });

  it('infers hero from exact name "Hero"', () => {
    const node = {
      id: 'h',
      type: 'frame',
      name: 'Hero',
      x: 0,
      y: 0,
      width: 375,
      height: 400,
    } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBe('hero');
  });

  it('infers footer from exact name "Footer"', () => {
    const node = {
      id: 'f',
      type: 'frame',
      name: 'Footer',
      x: 0,
      y: 0,
      width: 375,
      height: 200,
    } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBe('footer');
  });

  it('does not infer role for non-frame nodes', () => {
    const node = {
      id: 't',
      type: 'text',
      name: 'Button Label',
      x: 0,
      y: 0,
      width: 80,
      height: 20,
      content: 'Click',
    } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBeUndefined();
  });

  it('does not override explicit role', () => {
    const node = {
      id: 'b',
      type: 'frame',
      name: 'Search',
      x: 0,
      y: 0,
      width: 300,
      height: 44,
      role: 'input',
    } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBe('input'); // 保持明确的角色，而不是推断的搜索栏
  });

  it('does not infer role for generic names', () => {
    const node = {
      id: 'g',
      type: 'frame',
      name: 'Container',
      x: 0,
      y: 0,
      width: 300,
      height: 200,
    } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBeUndefined();
  });

  // ---------------------------------------------------------------------
  // Regression 2026-04-06 部分词/修饰符修复的覆盖范围
  // （提交 5e2e6f9、d45f1a5、f842853）。 These 案件被锁定
  // ROLE_PART_WORDS /匹配后第一个单词扫描的最终行为。
// ---------------------------------------------------------------------
  // --- "Card x" 其中 x 是结构片段：必须 NOT 成为 role=card ---
  it.each([
    ['Card Header'],
    ['Card Body'],
    ['Card Footer'],
    ['Card Title'],
    ['Card Content'],
    ['Card Image'],
    ['Card Media'],
    ['Card Label'],
    ['Card Action'],
    ['Card Actions'],
    ['Card Meta'],
    ['Card Caption'],
    ['Card Description'],
    ['Card Wrapper'],
  ])('does not infer card role from structural part name "%s"', (name) => {
    const node = { id: 'n', type: 'frame', name, x: 0, y: 0, width: 300, height: 60 } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).not.toBe('card');
    // Must 不继承卡默认 fill/shadow
    expect((node as { fill?: unknown }).fill).toBeUndefined();
    expect((node as { effects?: unknown }).effects).toBeUndefined();
  });

  // --- 角色词和部分词之间的 Punctuation 仍算作部分 ---
  it.each([['Card - Header'], ['Card: Body'], ['Card / Footer']])(
    'does not infer card role when punctuation separates it from part word: "%s"',
    (name) => {
      const node = { id: 'n', type: 'frame', name, x: 0, y: 0, width: 300, height: 60 } as PenNode;
      resolveNodeRole(node, ctx);
      expect(node.role).not.toBe('card');
    },
  );

  // --- Numeric 角色词和部分词之间的索引必须 NOT 泄漏角色 --- Regression：子代理名称节点“Card 1
  // Content”、“Card 2 Header”、“Button 3 Label”。 The 天真的 \w+
  // 单词扫描抓取了数字“1”作为第一个标记，错过了尾随部分单词，并错误地推断出内容包装器上的 role=card -
  // 然后得到了白卡默认填充并隐藏了其中的所有文本（来自 2026-04-06 健康跟踪器转储的“Upcoming 卡标题不可见”错误）。
  it.each([
    ['Card 1 Content'],
    ['Card 2 Header'],
    ['Card 3 Footer'],
    ['Card 1 Body'],
    ['Card 10 Title'],
    ['Button 1 Label'],
    ['Button 2 Icon'],
  ])(
    'does not infer card/button role when a numeric index precedes the part word: "%s"',
    (name) => {
      const node = { id: 'n', type: 'frame', name, x: 0, y: 0, width: 300, height: 60 } as PenNode;
      resolveNodeRole(node, ctx);
      // The 节点不应继承卡片白色填充+阴影。 Role 未定义或不是 card/button。
      expect(node.role).not.toBe('card');
      expect(node.role).not.toBe('button');
      expect((node as { fill?: unknown }).fill).toBeUndefined();
      expect((node as { effects?: unknown }).effects).toBeUndefined();
    },
  );

  // --- Modifier BEFORE 角色词：必须由 STILL 推断角色 ---
  it.each([
    ['Icon Button', 'button'],
    ['Primary Button', 'button'],
    ['Submit Button', 'button'],
    ['Text Button', 'button'],
    ['Image Card', 'card'],
    ['Icon Card', 'card'],
    ['Media Card', 'card'],
    ['User Card', 'card'],
    ['Product Card', 'card'],
  ])('infers %s role from modifier-before name "%s"', (name, expected) => {
    const node = { id: 'n', type: 'frame', name, x: 0, y: 0, width: 300, height: 60 } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBe(expected);
  });

  // --- Prepositional 变体：“x with y”表示 x 的变体，保留 --- 角色
  it.each([
    ['Card with Icon', 'card'],
    ['Card with Image', 'card'],
    ['Card with Header', 'card'],
    ['Card with Label', 'card'],
    ['Button with Icon', 'button'],
    ['Button with Image', 'button'],
    ['Button with Label', 'button'],
  ])('keeps role on prepositional variant "%s" → %s', (name, expected) => {
    const node = { id: 'n', type: 'frame', name, x: 0, y: 0, width: 300, height: 60 } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBe(expected);
  });

  // --- "x Icon" / "x Label"：角色模式被部分单词跳过，图标模式（稍后在 NAME_PATTERN_MAP 中）然后接管
  // ---
  it('falls through to icon role for "Card Icon" (icon is a part word after card)', () => {
    const node = {
      id: 'n',
      type: 'frame',
      name: 'Card Icon',
      x: 0,
      y: 0,
      width: 40,
      height: 40,
    } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBe('icon');
  });

  it('falls through to icon role for "Button Icon"', () => {
    const node = {
      id: 'n',
      type: 'frame',
      name: 'Button Icon',
      x: 0,
      y: 0,
      width: 40,
      height: 40,
    } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBe('icon');
  });
});

// ---------------------------------------------------------------------------
// Size-卡片系列角色的理智守卫
// ---------------------------------------------------------------------------

describe('resolveNodeRole — absurd-size guard for card-family roles', () => {
  const ctx: RoleContext = { canvasWidth: 375 };

  it('refuses stat-card inference on a 6×6 status dot (name "Status Dot")', () => {
    // Regression：基于名称的推理在“Status Dot”中看到了 `/\bstat/` 并将其标记为统计卡。 That
    // 角色注入 padding:[24,24] + cornerRadius + 阴影，将 6
    // 像素点膨胀到超大卡片中。当节点太小而无法成为卡容器时，Guard 会完全剥夺该角色。
    const node = {
      id: 'dot',
      type: 'frame',
      name: 'Status Dot',
      x: 0,
      y: 0,
      width: 6,
      height: 6,
      cornerRadius: 3,
      fill: [{ type: 'solid', color: '#22C55E' }],
    } as unknown as PenNode;
    resolveNodeRole(node, ctx);
    expect((node as { role?: string }).role).toBeUndefined();
    expect((node as { padding?: unknown }).padding).toBeUndefined();
    expect((node as { effects?: unknown[] }).effects).toBeUndefined();
  });

  it('refuses card inference on a tiny swatch (width=20)', () => {
    // 名为“Card Swatch”的 20×20 色样也不应该成为卡片 — 20px 低于
    // CARD_LIKE_MIN_DIMENSION 阈值。
    const node = {
      id: 'swatch',
      type: 'frame',
      name: 'Card Swatch',
      x: 0,
      y: 0,
      width: 20,
      height: 20,
    } as unknown as PenNode;
    resolveNodeRole(node, ctx);
    expect((node as { role?: string }).role).toBeUndefined();
  });

  it('refuses LLM-emitted card role on a 16×16 node (not just name-inferred)', () => {
    // The LLM 还可以直接在小节点上发出 `role: "card"`。 Guard 无论哪种方式都适用。
    const node = {
      id: 'pill',
      type: 'frame',
      role: 'card',
      x: 0,
      y: 0,
      width: 16,
      height: 16,
    } as unknown as PenNode;
    resolveNodeRole(node, ctx);
    expect((node as { role?: string }).role).toBeUndefined();
  });

  it('applies card defaults normally on a normal-sized card (200×120)', () => {
    // Counter-test：真正的卡片大小的框架仍然必须应用卡片默认值。 The 防护应该只在微小元素上发挥作用。
    const node = {
      id: 'real-card',
      type: 'frame',
      name: 'Restaurant Card',
      x: 0,
      y: 0,
      width: 200,
      height: 120,
    } as unknown as PenNode;
    resolveNodeRole(node, ctx);
    expect((node as { role?: string }).role).toBe('card');
    expect((node as { cornerRadius?: number }).cornerRadius).toBeDefined();
    expect((node as { effects?: unknown[] }).effects).toBeDefined();
  });

  it('leaves the role alone when width/height are fill_container (unknown pixel size)', () => {
    // When 尺寸是调整大小的关键字，我们无法判断最终渲染是否会很小，因此守卫拒绝决定并且角色正常应用。 This
    // 可防止警卫意外剥离响应式布局上的卡片角色。
    const node = {
      id: 'responsive-card',
      type: 'frame',
      role: 'card',
      width: 'fill_container',
      height: 'fit_content',
    } as unknown as PenNode;
    resolveNodeRole(node, ctx);
    expect((node as { role?: string }).role).toBe('card');
    expect((node as { effects?: unknown[] }).effects).toBeDefined();
  });
});

// ---------------------------------------------------------------------------
// Page-镀铬卡防护装置
// ---------------------------------------------------------------------------

describe('resolveNodeRole — page-chrome roles must not infer inside a card-family parent', () => {
  it('does NOT infer navbar from "Header" when parent is a card', () => {
    // Regression：心率卡有一个名为“Header”的子项（其标题行带有心形图标和“Heart Rate”标签）。
    // The 名称 `header` 在词法上匹配 NAME_EXACT_MAP → 'navbar'，然后注入
    // navbarFill（#FFFFFF on light）+ 底部边框，将内部部分变成不属于黑暗心率卡内部的耀眼的白色
    // 条。
    const node = {
      id: 'card-header',
      type: 'frame',
      name: 'Header',
      width: 343,
      height: 48,
    } as unknown as PenNode;
    const ctxInCard: RoleContext = { canvasWidth: 375, parentRole: 'card' };
    resolveNodeRole(node, ctxInCard);
    expect((node as { role?: string }).role).toBeUndefined();
    expect((node as { fill?: unknown }).fill).toBeUndefined();
    expect((node as { stroke?: unknown }).stroke).toBeUndefined();
  });

  it('does NOT infer footer from "Footer" when parent is a stat-card', () => {
    const node = {
      id: 'card-footer',
      type: 'frame',
      name: 'Footer',
      width: 343,
      height: 36,
    } as unknown as PenNode;
    const ctxInCard: RoleContext = { canvasWidth: 375, parentRole: 'stat-card' };
    resolveNodeRole(node, ctxInCard);
    expect((node as { role?: string }).role).toBeUndefined();
    expect((node as { padding?: unknown }).padding).toBeUndefined();
  });

  it('STILL infers navbar from "Header" at the page top level (no card parent)', () => {
    // Counter-test：在页面顶层（没有父角色，或者父级是类似部分的布局容器），“Header”仍然合法地表
    // 示页面导航栏。 Don 不要过度剥离。
    const node = {
      id: 'page-header',
      type: 'frame',
      name: 'Header',
      width: 375,
      height: 56,
    } as unknown as PenNode;
    const ctxAtTop: RoleContext = { canvasWidth: 375 };
    resolveNodeRole(node, ctxAtTop);
    expect((node as { role?: string }).role).toBe('navbar');
    expect((node as { fill?: Array<{ color?: string }> }).fill).toBeDefined();
  });

  it('STILL infers footer from "Footer" at the page top level (no card parent)', () => {
    const node = {
      id: 'page-footer',
      type: 'frame',
      name: 'Footer',
      width: 375,
      height: 200,
    } as unknown as PenNode;
    const ctxAtTop: RoleContext = { canvasWidth: 375 };
    resolveNodeRole(node, ctxAtTop);
    expect((node as { role?: string }).role).toBe('footer');
  });

  it('does NOT strip an EXPLICIT navbar role even when parent is a card (LLM was deliberate)', () => {
    // Guard 仅适用于 NAME-INFERRED page-chrome 角色。 If 的
    // LLM 在卡内的节点上显式发出 `role: navbar`，
    // 我们相信作者的意图并应用导航栏默认值。 (Edge
// case: a card containing a mini search/nav header. Probably
    // 错误，但不是我们要覆盖的地方。）
    const node = {
      id: 'explicit-nav',
      type: 'frame',
      name: 'Some Inner Nav',
      role: 'navbar',
      width: 343,
      height: 48,
    } as unknown as PenNode;
    const ctxInCard: RoleContext = { canvasWidth: 375, parentRole: 'card' };
    resolveNodeRole(node, ctxInCard);
    expect((node as { role?: string }).role).toBe('navbar');
  });
});

// ---------------------------------------------------------------------------
// Theme 检测 (resolveTreeRoles)
// ---------------------------------------------------------------------------

describe('resolveTreeRoles — theme-aware role defaults', () => {
  it('paints navbar default with WHITE on a light page (theme inferred light)', () => {
    // Baseline：页面背景为白色，导航栏角色默认应选择浅色主题填充（#FFFFFF）。 Locks
    // 在原始行为中，因此主题切换纯粹是附加的。
    const navbar = {
      id: 'nav',
      type: 'frame',
      role: 'navbar',
      width: 375,
      height: 56,
    } as unknown as PenNode;
    const page = {
      id: 'page',
      type: 'frame',
      width: 375,
      height: 800,
      fill: [{ type: 'solid', color: '#FFFFFF' }],
      children: [navbar],
    } as unknown as PenNode;

    resolveTreeRoles(page, 375);

    const fill = (navbar as unknown as { fill?: Array<{ color?: string }> }).fill;
    expect(fill?.[0]?.color).toBe('#FFFFFF');
  });

  it('paints navbar default with DARK fill on a dark page (theme inferred dark)', () => {
    // The bug：在使用的 #111111 深色页面上没有 LLM-set 填充的导航栏
    // 继承硬编码的 #FFFFFF 默认值，留下耀眼的白色
    // 酒吧横贯顶部的深色设计。 The 主题检测器读取
    // 根填充，将页面分类为深色，以及导航栏角色
// function picks navbarFill('dark') = #111111.
    const navbar = {
      id: 'nav',
      type: 'frame',
      role: 'navbar',
      width: 375,
      height: 56,
    } as unknown as PenNode;
    const page = {
      id: 'page',
      type: 'frame',
      width: 375,
      height: 800,
      fill: [{ type: 'solid', color: '#111111' }],
      children: [navbar],
    } as unknown as PenNode;

    resolveTreeRoles(page, 375);

    const fill = (navbar as unknown as { fill?: Array<{ color?: string }> }).fill;
    expect(fill?.[0]?.color).toBe('#111111');
  });

  it('paints card default with DARK fill on a dark page', () => {
    const card = {
      id: 'card',
      type: 'frame',
      role: 'card',
      width: 300,
      height: 200,
    } as unknown as PenNode;
    const page = {
      id: 'page',
      type: 'frame',
      width: 375,
      height: 800,
      fill: [{ type: 'solid', color: '#0A0A0A' }],
      children: [card],
    } as unknown as PenNode;

    resolveTreeRoles(page, 375);

    const fill = (card as unknown as { fill?: Array<{ color?: string }> }).fill;
    expect(fill?.[0]?.color).toBe('#1A1A1A');
  });

  it('does not paint white orphan-container fill over stroked activity rings', () => {
    const ringCircle = {
      id: 'activity_rings-steps-circle',
      type: 'frame',
      name: 'Steps Circle',
      width: 100,
      height: 100,
      cornerRadius: 50,
      stroke: { thickness: 10, fill: [{ type: 'solid', color: '#00D09C' }] },
      layout: 'vertical',
      children: [
        {
          id: 'steps-value',
          type: 'text',
          content: '8,432',
        },
      ],
    } as unknown as PenNode;
    const ring = {
      id: 'steps-ring',
      type: 'frame',
      name: 'Steps Ring',
      width: 100,
      height: 100,
      children: [ringCircle],
    } as unknown as PenNode;
    const page = {
      id: 'page',
      type: 'frame',
      width: 375,
      height: 812,
      fill: [{ type: 'solid', color: '#1A1A2E' }],
      children: [ring],
    } as unknown as PenNode;

    resolveTreeRoles(page, 375);
    resolveTreePostPass(page, 375);

    const fill = (ringCircle as unknown as { fill?: Array<{ color?: string }> }).fill;
    const effects = (ringCircle as unknown as { effects?: unknown[] }).effects;
    expect(fill).toBeUndefined();
    expect(effects).toBeUndefined();
  });

  it('does NOT overwrite an LLM-supplied fill (only fills missing defaults)', () => {
    // The applyDefaults 规则必须仍然成立：如果 LLM 发出任何填充，解析器将不理会它 -
    // 即使主题感知默认值会选择其他内容。 This 防止解析器成为破坏性层。
    const navbar = {
      id: 'nav',
      type: 'frame',
      role: 'navbar',
      width: 375,
      height: 56,
      fill: [{ type: 'solid', color: '#FF00AA' }],
    } as unknown as PenNode;
    const page = {
      id: 'page',
      type: 'frame',
      width: 375,
      height: 800,
      fill: [{ type: 'solid', color: '#111111' }],
      children: [navbar],
    } as unknown as PenNode;

    resolveTreeRoles(page, 375);

    const fill = (navbar as unknown as { fill?: Array<{ color?: string }> }).fill;
    expect(fill?.[0]?.color).toBe('#FF00AA');
  });

  it('falls back to LIGHT theme when the root fill is missing or unparseable', () => {
    const navbar = {
      id: 'nav',
      type: 'frame',
      role: 'navbar',
      width: 375,
      height: 56,
    } as unknown as PenNode;
    const page = {
      id: 'page',
      type: 'frame',
      width: 375,
      height: 800,
      // No 填充根 → 默认为浅色主题
      children: [navbar],
    } as unknown as PenNode;

    resolveTreeRoles(page, 375);

    const fill = (navbar as unknown as { fill?: Array<{ color?: string }> }).fill;
    expect(fill?.[0]?.color).toBe('#FFFFFF');
  });

  it('honors EXPLICIT theme override even when called on a sub-tree without its own fill', () => {
    // Locks 在修复 Codex 中捕获：`sanitizeNodesForInsert` 和
    // `sanitizeNodesForUpsert` 任意调用 `resolveTreeRoles`
    // 子树（没有自己填充的卡片或导航栏 - LLM
    // 省略它期望黑暗页面背景显示出来）。 Without
    // 明确的主题参数，主题检测将读取
    // 子树根缺少填充，回落到“光”，然后盖章
    // #FFFFFF 在黑暗页面的顶部。
//
    // The 修复是从文档中查找实际的页面根目录
    // 存储在调用站点并通过此传递检测到的主题
    // 最后一个位置参数。 This 测试模拟该路径。
    const navbar = {
      id: 'nav-subtree',
      type: 'frame',
      role: 'navbar',
      width: 375,
      height: 56,
      // No 填充此导航栏 — LLM 期望显示页面 bg。
    } as unknown as PenNode;

    // Call resolveTreeRoles 在导航栏上 DIRECTLY （没有父级传递），但提供一个明确的深色主题 -
    // 这是清理路径在查找实时页面根目录后所做的事情。
    resolveTreeRoles(navbar, 375, undefined, undefined, undefined, false, 'dark');

    const fill = (navbar as unknown as { fill?: Array<{ color?: string }> }).fill;
    expect(fill?.[0]?.color).toBe('#111111');
  });

  it('explicit theme override beats auto-detection from a non-page sub-tree root', () => {
    // 上一个测试的 Stronger 版本：子树根 HAS 填充（例如带有 #1a1a1a
    // 的卡片），自动检测无论如何都会将其分类为“黑暗”。 This 测试确保显式“轻”覆盖仍然获胜，证明显式参数不会被默默
    // 忽略。
    const card = {
      id: 'card-subtree',
      type: 'frame',
      role: 'card',
      width: 300,
      height: 200,
      fill: [{ type: 'solid', color: '#FAFAFA' }],
      children: [
        {
          id: 'inner-navbar',
          type: 'frame',
          role: 'navbar',
          width: 300,
          height: 56,
        },
      ],
    } as unknown as PenNode;

    // Force theme = 'dark' 即使卡片本身看起来很亮
    resolveTreeRoles(card, 375, undefined, undefined, undefined, false, 'dark');

    const innerNavbar = (card as unknown as { children: PenNode[] }).children[0];
    const fill = (innerNavbar as unknown as { fill?: Array<{ color?: string }> }).fill;
    expect(fill?.[0]?.color).toBe('#111111');
  });
});
