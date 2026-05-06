import { describe, it, expect } from 'vitest';
import type { PenNode } from '@zseven-w/pen-types';
import { normalizeTreeLayout } from '../layout/normalize-tree';

const frame = (props: Partial<PenNode> & { children?: PenNode[] }): PenNode =>
  ({
    id: 'f1',
    type: 'frame',
    ...props,
  }) as PenNode;

const rect = (id: string, props: Partial<PenNode> = {}): PenNode =>
  ({
    id,
    type: 'rectangle',
    width: 50,
    height: 30,
    ...props,
  }) as PenNode;

describe('normalizeTreeLayout', () => {
  it('leaves leaf rectangle nodes unchanged', () => {
    const node = rect('a', { x: 10, y: 20 });
    const before = JSON.stringify(node);
    normalizeTreeLayout(node);
    expect(JSON.stringify(node)).toBe(before);
  });

  it('preserves an explicit vertical layout', () => {
    const node = frame({
      layout: 'vertical',
      children: [rect('a'), rect('b')],
    });
    normalizeTreeLayout(node);
    expect((node as PenNode & { layout?: string }).layout).toBe('vertical');
  });

  it('does not overwrite horizontal layout set by a prior role resolver', () => {
    // Simulates 导航栏案例：角色解析器写入布局='水平'，子级没有布局提示（inferLayout
    // 将返回未定义），因此规范化将回退到“垂直”并破坏语义上正确的值。 It 一定不能。
    const node = frame({
      layout: 'horizontal',
      children: [rect('logo'), rect('links'), rect('cta')],
    });
    normalizeTreeLayout(node);
    expect((node as PenNode & { layout?: string }).layout).toBe('horizontal');
  });

  it('preserves absolute positioning when all children carry x/y', () => {
    // Frame 没有布局，并且子级都带有显式 x/y 是一个有意的绝对定位容器（手机模型内部结构、带有浮动覆盖层的英雄图像等）。 The
    // 后备不得写入 `vertical` 或剥离 x/y。
    const node = frame({
      width: 300,
      height: 600,
      children: [
        rect('statusBar', { x: 0, y: 0, width: 300, height: 44 }),
        rect('content', { x: 16, y: 60, width: 268, height: 500 }),
        rect('homeIndicator', { x: 100, y: 580, width: 100, height: 4 }),
      ],
    });
    normalizeTreeLayout(node);
    expect((node as PenNode & { layout?: string }).layout).toBeUndefined();
    const kids = (node as PenNode & { children: PenNode[] }).children;
    expect(kids[0].x).toBe(0);
    expect(kids[0].y).toBe(0);
    expect(kids[1].x).toBe(16);
    expect(kids[1].y).toBe(60);
    expect(kids[2].x).toBe(100);
    expect(kids[2].y).toBe(580);
  });

  it('preserves absolute positioning when even one child carries x/y', () => {
    // If 该模型至少用显式坐标标记了一个子级，我们将容器视为绝对定位。 This 是保守的，但避免破坏手工放置的覆盖层。
    const node = frame({
      children: [rect('base'), rect('overlay', { x: 120, y: 80 })],
    });
    normalizeTreeLayout(node);
    expect((node as PenNode & { layout?: string }).layout).toBeUndefined();
    const kids = (node as PenNode & { children: PenNode[] }).children;
    expect(kids[1].x).toBe(120);
    expect(kids[1].y).toBe(80);
  });

  it('writes explicit horizontal layout when gap is the only hint', () => {
    const node = frame({
      gap: 12,
      children: [rect('a'), rect('b')],
    });
    normalizeTreeLayout(node);
    // inferLayout() 目前将间隙视为水平信号 - 保留该行为。
    expect((node as PenNode & { layout?: string }).layout).toBe('horizontal');
  });

  it('falls back to vertical for multi-child frames with no layout hints', () => {
    const node = frame({
      children: [rect('a'), rect('b'), rect('c')],
    });
    normalizeTreeLayout(node);
    expect((node as PenNode & { layout?: string }).layout).toBe('vertical');
  });

  it('leaves single-child frame without a layout untouched', () => {
    const node = frame({
      children: [rect('a')],
    });
    normalizeTreeLayout(node);
    expect((node as PenNode & { layout?: string }).layout).toBeUndefined();
  });

  it('strips x and y from children of an active layout frame', () => {
    const node = frame({
      layout: 'vertical',
      children: [rect('a', { x: 99, y: 33 }), rect('b', { x: 12, y: 77 })],
    });
    normalizeTreeLayout(node);
    const kids = (node as PenNode & { children: PenNode[] }).children;
    expect('x' in kids[0]).toBe(false);
    expect('y' in kids[0]).toBe(false);
    expect('x' in kids[1]).toBe(false);
    expect('y' in kids[1]).toBe(false);
  });

  it('keeps x and y on overlay children', () => {
    const overlay: PenNode = {
      id: 'overlay1',
      type: 'rectangle',
      role: 'overlay',
      width: 8,
      height: 8,
      x: 40,
      y: 0,
    } as PenNode;
    const node = frame({
      layout: 'horizontal',
      children: [rect('a', { x: 5, y: 5 }), overlay],
    });
    normalizeTreeLayout(node);
    const kids = (node as PenNode & { children: PenNode[] }).children;
    expect('x' in kids[0]).toBe(false);
    expect('y' in kids[0]).toBe(false);
    expect(kids[1].x).toBe(40);
    expect(kids[1].y).toBe(0);
  });

  it('recurses into nested frames', () => {
    // Neither 内部或外部子级都携带 x/y，因此两者都获得垂直后备；递归访问内部框架并写入其布局。
    const inner = frame({
      id: 'inner',
      children: [rect('c'), rect('d')],
    });
    const outer = frame({
      id: 'outer',
      children: [inner, rect('b')],
    });
    normalizeTreeLayout(outer);
    expect((outer as PenNode & { layout?: string }).layout).toBe('vertical');
    expect((inner as PenNode & { layout?: string }).layout).toBe('vertical');
  });

  it('does not strip x/y when layout is "none"', () => {
    const node = frame({
      layout: 'none',
      children: [rect('a', { x: 10, y: 20 }), rect('b', { x: 30, y: 40 })],
    });
    normalizeTreeLayout(node);
    const kids = (node as PenNode & { children: PenNode[] }).children;
    expect(kids[0].x).toBe(10);
    expect(kids[0].y).toBe(20);
    expect(kids[1].x).toBe(30);
    expect(kids[1].y).toBe(40);
  });

  it('overlay-only children do not block the vertical fallback', () => {
    // 唯一定位的子级是覆盖层的容器仍然被认为是“模型忘记布局”——覆盖层保留它们的坐标，而基础框架为其余部分获取垂直布局。
    const overlay: PenNode = {
      id: 'overlay1',
      type: 'rectangle',
      role: 'overlay',
      width: 8,
      height: 8,
      x: 40,
      y: 0,
    } as PenNode;
    const node = frame({
      children: [rect('a'), rect('b'), overlay],
    });
    normalizeTreeLayout(node);
    expect((node as PenNode & { layout?: string }).layout).toBe('vertical');
    const kids = (node as PenNode & { children: PenNode[] }).children;
    // 覆盖层仍然带有其绝对坐标
    expect(kids[2].x).toBe(40);
    expect(kids[2].y).toBe(0);
  });

  it('does NOT verticalize when ALL non-overlay children are frames (overlay composition)', () => {
    // This 是诊断出的错误子代理：AI 发出无布局的父代理
    // 包含结构化嵌套框架子项（例如环+值包装，
    // 英雄 + 浮动卡叠加、徽章堆叠）。 Children 没有 x/y
    // 因为 AI 假设 (0,0) 就可以工作。 The 之前的行为
    // 会默默地将父级重写为 layout='vertical', stacking
    // 将帧放入列表中并破坏预期的覆盖。
//
    // New 行为：当每个非覆盖子级都是一个框架时，将其视为
    // 覆盖容器并保留布局未定义，以便渲染器
    // 尊重儿童的立场。
    const inner1 = frame({ id: 'inner1', width: 80, height: 80 });
    const inner2 = frame({ id: 'inner2', width: 80, height: 80 });
    const outer = frame({
      id: 'outer',
      width: 80,
      height: 80,
      children: [inner1, inner2],
    });
    normalizeTreeLayout(outer);
    expect((outer as PenNode & { layout?: string }).layout).toBeUndefined();
  });

  it('still verticalizes mixed text+shape stacks (frame + text) where vertical is the right call', () => {
    // Counter-测试扩展的组合原语规则。 The 规则不得过度触发：包含嵌套框架加上文本节点的框架是内容堆栈，并且 SHOULD
    // 是垂直的。 Homogeneous 仅形状子项（框架+框架、框架+矩形、椭圆+椭圆）信号合成；混合文本或 icon_font
    // 会破坏启发式。
    const inner = frame({ id: 'inner', children: [rect('c'), rect('d')] });
    const outer = frame({
      id: 'outer',
      children: [inner, { id: 'label', type: 'text', content: 'Hello' } as unknown as PenNode],
    });
    normalizeTreeLayout(outer);
    expect((outer as PenNode & { layout?: string }).layout).toBe('vertical');
  });

  it('does NOT verticalize all-ellipse concentric stacks (progress-ring pattern)', () => {
    // The 活动环回归：LLM 发出一个无布局的父级，在 (0,0) 处有 n 个同心椭圆，期望它们呈现居中。 The 旧的
    // all-FRAME-children 启发式错过了这一点，因为子级是省略号（不是框架）并落入垂直回退，将环堆叠为列表。
    // The 扩大的组合基元规则现在将仅限椭圆的子项识别为组合，并使布局未定义。
    const ellipse = (id: string, w: number, h: number): PenNode =>
      ({ id, type: 'ellipse', width: w, height: h }) as PenNode;
    const parent = frame({
      id: 'rings',
      width: 120,
      height: 120,
      children: [ellipse('outer', 120, 120), ellipse('middle', 84, 84), ellipse('inner', 52, 52)],
    });
    normalizeTreeLayout(parent);
    expect((parent as PenNode & { layout?: string }).layout).toBeUndefined();
  });

  it('does NOT verticalize frame + ellipse (ring wrapped in a frame)', () => {
    // 常见的构图：外框背景，顶部有嵌套的椭圆环。 Both 是组合基元，因此启发式保持布局未定义，并让渲染器使用子级自己的定位。
    const parent = frame({
      id: 'avatar',
      width: 80,
      height: 80,
      children: [
        { id: 'bg', type: 'frame', width: 80, height: 80 } as unknown as PenNode,
        { id: 'ring', type: 'ellipse', width: 80, height: 80 } as unknown as PenNode,
      ],
    });
    normalizeTreeLayout(parent);
    expect((parent as PenNode & { layout?: string }).layout).toBeUndefined();
  });

  it('STILL verticalizes rectangle-only stacks (list of cards, not an overlay)', () => {
    // Rectangles 是故意在组合基元集中的 NOT。没有布局提示的 3 个矩形几乎总是内容堆栈（卡片列表、导航按钮、分隔行），因此
    // 我们为其保留垂直回退。 Authors 如果确实想要一个矩形叠加组合，则必须在至少一个子元素上声明 x/y。
    const parent = frame({
      id: 'list',
      children: [rect('a'), rect('b'), rect('c')],
    });
    normalizeTreeLayout(parent);
    expect((parent as PenNode & { layout?: string }).layout).toBe('vertical');
  });
});
