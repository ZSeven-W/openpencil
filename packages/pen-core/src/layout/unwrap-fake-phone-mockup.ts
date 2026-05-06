import type { PenNode } from '@zseven-w/pen-types';

/**
 * Detect 并修复“假
 * 手机样机”框架 - 看起来像手机样机的框架（cornerRadius 28-40 + 宽度 240-320，或字面名称为“Phone
 * Mockup”），但填充了多个子项或使用水平/垂直布局。真正的手机模型是 `ONE` 框架，最多有一个占位符子项和 `layout:
 * 'none'`。 Failure 模式解决了以下问题：较弱的 AI 子代理（M2 系列、GLM、Kimi）有时会读取通用的“Phone 模型 =
 * ONE 框架、cornerRadius 32”提示片段，并将这些视觉属性应用到自己的根框架，然后将整个部分内容放入其中。 The
 *
 * 生成的包装器具有 260 像素宽的水平布局，将每个部分压缩为 40 像素的列
 * - 视觉上一团糟（参见
 * 2026-04-06 健康跟踪器案例）。 Two 恢复模式： 1. **Unwrap（首选）** — 当假包装器是 `node` 的 CHILD
 * 时，我们将其丢弃并将其子级提升一级。 Visual 样式与包装器一起被丢弃。 2. **Sanitize（后备）** - 当 `node`
 * ITSELF 是假包装器时（子代理自己的根框架是假的；我们在范围内没有父框架），我们剥离手机边框视觉效果（cornerRadius、固定宽度、布
 * 局）并重命名。 Children 留下来；它们可能是重复的，但至少它们以正常的垂直堆栈呈现，而不是被压成 260px 的水平列。
 *
 * Returns `true` 如果任何节点发生突变。当返回 true 时，Callers 在商店拥有的节点上运行 MUST 发布
 *
 * Zustand 更新 - 就地突变本身不会通知订阅者，因此画布将继续显示旧的（假模型）状态，直到其他事件触发商店写入。
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 *
 */
export function unwrapFakePhoneMockups(node: PenNode): boolean {
  let changed = false;

  // Mode (2)：如果根本身是假包装器，则进行自我清理。
  if (isFakePhoneMockup(node)) {
    sanitizeFakePhoneMockupRoot(node);
    changed = true;
  }

  if (!('children' in node) || !Array.isArray(node.children)) return changed;

  // Mode (1)：向孩子们扔假包装纸，宣传他们的内容。
  const newChildren: PenNode[] = [];
  let unwrappedAnyChild = false;
  for (const child of node.children) {
    if (isFakePhoneMockup(child)) {
      const grandchildren =
        ('children' in child && Array.isArray(child.children) ? child.children : []) ?? [];
      for (const gc of grandchildren) newChildren.push(gc);
      unwrappedAnyChild = true;
    } else {
      newChildren.push(child);
    }
  }
  if (unwrappedAnyChild) {
    node.children = newChildren;
    changed = true;
  }

  // Recurse 进入（可能更新的）子级。 Promoted 孙子也会被访问 - 如果一个假模型包裹另一个假模型，则两者都会在一次传递中处理。
  for (const child of node.children) {
    if (unwrapFakePhoneMockups(child)) {
      changed = true;
    }
  }

  return changed;
}

/**
 * Strip 手机边框视觉
 * 签名来自框架到位。 Used 当框架本身是假包装器并且我们无法将其与父级分离时。
 */
function sanitizeFakePhoneMockupRoot(node: PenNode): void {
  const rec = node as PenNode & {
    name?: string;
    cornerRadius?: unknown;
    width?: unknown;
    height?: unknown;
    layout?: 'none' | 'vertical' | 'horizontal';
    fill?: unknown;
  };
  // Drop 误导性视觉效果
  delete rec.cornerRadius;
  // Restore 一个合理的容器宽度 - fill_container 让父级的布局决定实际宽度，而不是将我们锁定在 260px。
  rec.width = 'fill_container';
  // Drop 固定边框高度；让内容驱动内在高度。
  rec.height = 'fit_content';
  // Force 垂直，因此孩子们可以堆叠，而不是在假边框内水平压缩。
  rec.layout = 'vertical';
  // Clear 具有误导性的“Phone Mockup”名称，因此下游角色推断不会对其起作用。
  if (typeof rec.name === 'string' && /phone\s*mockup|app\s*mockup/i.test(rec.name)) {
    rec.name = 'Section';
  }
}

function isFakePhoneMockup(node: PenNode): boolean {
  if (node.type !== 'frame') return false;
  if (!('children' in node) || !Array.isArray(node.children)) return false;

  // Detection 信号#1：字面名称匹配。 Models 通常将提示的措辞逐字复制到节点名称中。
  const name = (node.name ?? '').toLowerCase();
  const hasPhoneName = /phone\s*mockup|app\s*mockup|手机\s*样机|手机\s*模型|device\s*frame/.test(
    name,
  );

  // Detection 信号#2：视觉签名。 The 大半径 (>= 28) 和窄宽度 (240-320) 的组合是经典的手机边框。
  const cornerR = readCornerRadius(node);
  const widthNum = typeof node.width === 'number' ? node.width : null;
  const hasPhoneShape =
    cornerR != null &&
    cornerR >= 28 &&
    cornerR <= 40 &&
    widthNum != null &&
    widthNum >= 240 &&
    widthNum <= 320;

  if (!hasPhoneName && !hasPhoneShape) return false;

  // 真正的手机模型是 ONE 框架，最多有一个占位符子项和 `layout: 'none'` （或未设置布局）。 Anything
  // 更详细地说是一个分代理错误。
  const childCount = node.children.length;
  const layout = (node as PenNode & { layout?: string }).layout;
  const tooManyChildren = childCount > 1;
  const wrongLayout = layout === 'horizontal' || layout === 'vertical';
  return tooManyChildren || wrongLayout;
}

function readCornerRadius(node: PenNode): number | null {
  const cr = (node as PenNode & { cornerRadius?: number | number[] }).cornerRadius;
  if (typeof cr === 'number') return cr;
  if (Array.isArray(cr) && cr.length > 0 && typeof cr[0] === 'number') return cr[0];
  return null;
}
