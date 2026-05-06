// @vitest-环境 jsdom
import { describe, it, expect, vi } from 'vitest';

// paper.js 在导入时自初始化 Canvas 2d 上下文，这会在 jsdom 中崩溃。 Mock 在任何传递导入之前都可以触发真正的模块。
vi.mock('paper', () => ({
  default: {
    PaperScope: class {},
    setup: () => {},
    project: { activeLayer: { removeChildren: () => {} } },
  },
}));

import { parseSvgToNodes } from '@/utils/svg-parser';

// ---------------------------------------------------------------------------
// 1. SVG 解析器 SKIP_TAGS — 危险元素被剥离
// ---------------------------------------------------------------------------

describe('SVG parser SKIP_TAGS', () => {
  /** Recursively 收集树中所有节点名称 */
  function collectNames(nodes: { name?: string; children?: any[] }[]): string[] {
    const names: string[] = [];
    for (const n of nodes) {
      if (n.name) names.push(n.name);
      if (n.children) names.push(...collectNames(n.children));
    }
    return names;
  }

  /** Recursively 收集所有节点类型 */
  function collectTypes(nodes: { type?: string; children?: any[] }[]): string[] {
    const types: string[] = [];
    for (const n of nodes) {
      if (n.type) types.push(n.type);
      if (n.children) types.push(...collectTypes(n.children));
    }
    return types;
  }

  it('strips <script> tags from parsed SVG', () => {
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
        <rect x="0" y="0" width="50" height="50" fill="red"/>
        <script>alert("xss")</script>
      </svg>`;
    const nodes = parseSvgToNodes(svg);
    const names = collectNames(nodes).map((n) => n.toLowerCase());
    expect(names).not.toContain('script');
    // The 矩形应该仍然存在
    expect(nodes.length).toBeGreaterThan(0);
  });

  it('strips <foreignObject> tags from parsed SVG', () => {
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
        <rect x="0" y="0" width="50" height="50" fill="blue"/>
        <foreignObject width="100" height="100">
          <div xmlns="http://www.w3.org/1999/xhtml">HTML content</div>
        </foreignObject>
      </svg>`;
    const nodes = parseSvgToNodes(svg);
    const names = collectNames(nodes).map((n) => n.toLowerCase());
    expect(names).not.toContain('foreignobject');
    expect(names).not.toContain('foreignObject');
  });

  it('strips <animate>, <animateMotion>, and <set> tags', () => {
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
        <rect x="0" y="0" width="50" height="50" fill="green">
          <animate attributeName="x" from="0" to="100" dur="1s"/>
          <animateMotion path="M0,0 L100,100" dur="2s"/>
          <set attributeName="fill" to="red"/>
        </rect>
      </svg>`;
    const nodes = parseSvgToNodes(svg);
    const types = collectTypes(nodes);
    // Only 矩形类型节点应保留，无动画节点
    for (const t of types) {
      expect(['rectangle', 'frame', 'path', 'ellipse', 'line', 'text', 'group']).toContain(t);
    }
  });

  it('preserves valid shape elements while stripping dangerous ones', () => {
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" width="200" height="200">
        <rect x="0" y="0" width="50" height="50" fill="red"/>
        <circle cx="100" cy="100" r="30" fill="blue"/>
        <script>document.cookie</script>
        <foreignObject><body xmlns="http://www.w3.org/1999/xhtml"><iframe/></body></foreignObject>
      </svg>`;
    const nodes = parseSvgToNodes(svg);
    // Should 有一个包含 2 个子元素的环绕框架（矩形 + 圆形）
    expect(nodes).toHaveLength(1);
    expect(nodes[0].type).toBe('frame');
    expect('children' in nodes[0] && nodes[0].children).toHaveLength(2);
  });
});

// ---------------------------------------------------------------------------
// 2. SVG 解析器 getAttr — ReDoS 安全正则表达式转义
// ---------------------------------------------------------------------------

describe('SVG parser getAttr ReDoS safety', () => {
  it('parses SVG with regex-special chars in style attribute without hanging', () => {
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
        <rect x="0" y="0" width="50" height="50" style="(x+x+)+y: red; fill: blue"/>
      </svg>`;

    const start = performance.now();
    const nodes = parseSvgToNodes(svg);
    const elapsed = performance.now() - start;

    // Must 在 100 毫秒内完成（将挂起而不转义）
    expect(elapsed).toBeLessThan(100);
    expect(nodes.length).toBeGreaterThan(0);
  });

  it('handles style with brackets and special characters safely', () => {
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
        <rect x="0" y="0" width="50" height="50" style="fill: green; [weird]: val; opacity: 0.5"/>
      </svg>`;

    const start = performance.now();
    const nodes = parseSvgToNodes(svg);
    const elapsed = performance.now() - start;

    expect(elapsed).toBeLessThan(100);
    expect(nodes.length).toBeGreaterThan(0);
  });
});

// NOTE：Figma 解析器解压缩限制是 fig-parser.ts (MAX_COMPRESSED_SIZE /
// MAX_UNZIPPED_SIZE / MAX_IMAGE_SIZE) 中的内部常量。 They 在 .fig 文件解压过程中防范 zip 炸弹。
// Testing 它们需要制作具有超大有效负载的有效 .fig 二进制文件，并且基于 WASM 的解析器挂起 vitest 的 jsdom
// 环境，因此这些是通过代码审查进行验证的。
