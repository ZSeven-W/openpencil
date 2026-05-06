import { describe, it, expect } from 'vitest';
import type { PenNode } from '@zseven-w/pen-types';

// Mock canvas-text-measure 以避免别名解析拉动仅浏览器依赖。
import { vi } from 'vitest';
vi.mock('@/canvas/canvas-text-measure', () => ({
  estimateLineWidth: () => 0,
  estimateTextHeight: () => 0,
  defaultLineHeight: () => 1.2,
  hasCjkText: () => false,
}));

vi.mock('@/stores/document-store', () => ({
  useDocumentStore: {
    getState: () => ({
      getNodeById: () => undefined,
      updateNode: () => {},
    }),
  },
}));

import { applyIconPathResolution } from '../icon-resolver';

// 最小有效 SVG 路径数据，用作测试的占位符，在这些测试中我们希望断言解析器保持 `d` 不变。
const CUSTOM_GEOMETRY_D = 'M0 0 L100 0 L100 50 Z';

function makePath(props: Partial<PenNode> & { name: string }): PenNode {
  return {
    id: 'p',
    type: 'path',
    x: 0,
    y: 0,
    width: 100,
    height: 50,
    d: CUSTOM_GEOMETRY_D,
    ...props,
  } as unknown as PenNode;
}

describe('applyIconPathResolution — opt-in marker gate', () => {
  // Regression 提交 5e2e6f9 的覆盖范围。
//
  // Before 修复后，解析器运行 prefix/substring 查找
  // EVERY 路径节点上的图标字典。 Data-viz 发生的路径名
  // 与图标键共享子字符串被劫持：
  //   - “Heart Rate Chart”→ lucide:circle
  //   - “Chart Fill”→ lucide:bar-chart-2
  //   - “Steps Progress” → 清晰：圆
  //   - “Calories Progress” → 清晰：圆
  //   - “Distance Progress” → 清晰：圆
  // The 修复了解析器的门控，因此只有名称带有
  // 考虑明确的图标/徽标/符号/字形标记。
  // --- Descriptive 几何名称 MUST 单独保留 ---
  it.each([
    ['Heart Rate Chart'],
    ['Heart Rate Waveform'],
    ['Steps Progress'],
    ['Calories Progress'],
    ['Distance Progress'],
    ['Chart Fill'],
    ['Bar Chart'],
    ['Line Chart'],
    ['Sparkline'],
    ['Activity Curve'],
    ['Custom Illustration'],
    ['Decorative Shape'],
  ])('does not touch custom geometry path named "%s"', (name) => {
    const node = makePath({ name });
    applyIconPathResolution(node);
    // The 解析器在成功匹配时写入 node.d 和 node.iconId — 对于数据可视化路径，两者必须保持不变。
    expect((node as { d?: string }).d).toBe(CUSTOM_GEOMETRY_D);
    expect((node as { iconId?: string }).iconId).toBeUndefined();
  });

  // --- Explicit 图标标记 MUST 仍然解析 --- “Search Icon”
  // 在删除尾随“icon”后缀后在图标字典中作为完全匹配→“搜索”→ lucide:search。
  it('still resolves a legitimate "Search Icon" path', () => {
    const node = makePath({ name: 'Search Icon' });
    applyIconPathResolution(node);
    // d 替换为 lucide 搜索路径，设置 iconId。
    expect((node as { d?: string }).d).not.toBe(CUSTOM_GEOMETRY_D);
    expect((node as { iconId?: string }).iconId).toBeDefined();
  });

  it('still resolves camelCase "SearchIcon"', () => {
    const node = makePath({ name: 'SearchIcon' });
    applyIconPathResolution(node);
    expect((node as { d?: string }).d).not.toBe(CUSTOM_GEOMETRY_D);
    expect((node as { iconId?: string }).iconId).toBeDefined();
  });

  it('still resolves kebab-case "search-icon"', () => {
    const node = makePath({ name: 'search-icon' });
    applyIconPathResolution(node);
    expect((node as { d?: string }).d).not.toBe(CUSTOM_GEOMETRY_D);
    expect((node as { iconId?: string }).iconId).toBeDefined();
  });

  it('still resolves snake_case "search_icon"', () => {
    const node = makePath({ name: 'search_icon' });
    applyIconPathResolution(node);
    expect((node as { d?: string }).d).not.toBe(CUSTOM_GEOMETRY_D);
    expect((node as { iconId?: string }).iconId).toBeDefined();
  });

  it('still resolves "Brand Logo" via the logo marker', () => {
    const node = makePath({ name: 'Brand Logo' });
    applyIconPathResolution(node);
    // “品牌”不是已知的图标字典键；没有准确的命中
    // 解析器仍然可以识别徽标意图并对节点进行排队
// for async iconify lookup, leaving a placeholder d behind — so the
    // 自定义几何图形 IS 被覆盖（这是正确的：调用者询问
// for a logo, not custom geometry).
    expect((node as { d?: string }).d).not.toBe(CUSTOM_GEOMETRY_D);
  });

  it('does not replace generic scaffold names like "WC1 Icon" with a fallback circle', () => {
    const node = makePath({ name: 'WC1 Icon' });
    applyIconPathResolution(node);
    expect((node as { d?: string }).d).toBe(CUSTOM_GEOMETRY_D);
    expect((node as { iconId?: string }).iconId).toBeUndefined();
  });

  // --- 单独的“图表”（带有明确图标标记的单词）仍然可以解析 ---
  it('resolves "Chart Icon" via the dictionary', () => {
    const node = makePath({ name: 'Chart Icon' });
    applyIconPathResolution(node);
    // After 去掉“图标”后缀，rawName 是“图表”，映射到字典中的条形图条目。
    expect((node as { d?: string }).d).not.toBe(CUSTOM_GEOMETRY_D);
    expect((node as { iconId?: string }).iconId).toBeDefined();
  });

  // ---“Bar Chart”（NO 标记）必须单独解析 NOT
  it('does NOT resolve bare "Bar Chart" (no icon/logo marker)', () => {
    const node = makePath({ name: 'Bar Chart' });
    applyIconPathResolution(node);
    expect((node as { d?: string }).d).toBe(CUSTOM_GEOMETRY_D);
    expect((node as { iconId?: string }).iconId).toBeUndefined();
  });

  // --- Non-path 节点类型是无操作 ---
  it('ignores non-path nodes', () => {
    const node = {
      id: 'f',
      type: 'frame',
      name: 'Heart Icon',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
    } as unknown as PenNode;
    expect(() => applyIconPathResolution(node)).not.toThrow();
    expect((node as { iconId?: string }).iconId).toBeUndefined();
  });
});
