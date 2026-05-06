import { describe, it, expect } from 'vitest';
import type { PenNode } from '@zseven-w/pen-types';
import { stripRedundantSectionFills } from '../layout/strip-redundant-section-fills';

const frame = (props: Partial<PenNode> & { children?: PenNode[] }): PenNode =>
  ({
    id: 'f1',
    type: 'frame',
    ...props,
  }) as PenNode;

const solidFill = (color: string) => [{ type: 'solid' as const, color }];

describe('stripRedundantSectionFills', () => {
  it('strips a section fill that exactly matches the root fill', () => {
    const section = frame({
      id: 'sec1',
      name: 'Section',
      fill: solidFill('#1a1a2e'),
      children: [frame({ id: 'child' })],
    });
    const root = frame({
      id: 'root',
      fill: solidFill('#1a1a2e'),
      children: [section],
    });
    const changed = stripRedundantSectionFills(root);
    expect(changed).toBe(true);
    expect((section as PenNode & { fill?: unknown }).fill).toBeUndefined();
  });

  it('strips a section fill that matches a common safe-dark tint', () => {
    // Root 有 #1a1a2e（深海军蓝），部分有 #0a0a0a（近黑色安全深色）——经典的 M2.7
    // 失败，模型为每个部分根选择“安全”深色，隐藏了预期的根背景。
    const section = frame({
      id: 'sec1',
      name: 'Activity Rings Section',
      fill: solidFill('#0A0A0A'),
      children: [frame({ id: 'child' })],
    });
    const root = frame({
      id: 'root',
      fill: solidFill('#1a1a2e'),
      children: [section],
    });
    stripRedundantSectionFills(root);
    expect((section as PenNode & { fill?: unknown }).fill).toBeUndefined();
  });

  it('does not strip fill from a card (cards own their visual fill)', () => {
    const card = frame({
      id: 'card1',
      name: 'Stat Card',
      role: 'card',
      fill: solidFill('#0A0A0A'),
      cornerRadius: 12,
      children: [frame({ id: 'child' })],
    });
    const root = frame({
      id: 'root',
      fill: solidFill('#1a1a2e'),
      children: [card],
    });
    const changed = stripRedundantSectionFills(root);
    expect(changed).toBe(false);
    expect((card as PenNode & { fill?: unknown }).fill).toEqual(solidFill('#0A0A0A'));
  });

  it('does not strip fill from a button', () => {
    const button = frame({
      id: 'btn',
      name: 'CTA Button',
      role: 'button',
      fill: solidFill('#0A0A0A'),
      children: [frame({ id: 'label' })],
    });
    const root = frame({
      id: 'root',
      fill: solidFill('#1a1a2e'),
      children: [button],
    });
    stripRedundantSectionFills(root);
    expect((button as PenNode & { fill?: unknown }).fill).toEqual(solidFill('#0A0A0A'));
  });

  it('does not strip fill from a badge or chip', () => {
    const badge = frame({
      id: 'bd',
      name: 'Badge',
      role: 'badge',
      fill: solidFill('#0A0A0A'),
    });
    const chip = frame({
      id: 'ch',
      name: 'Chip',
      role: 'chip',
      fill: solidFill('#0A0A0A'),
    });
    const root = frame({
      id: 'root',
      fill: solidFill('#1a1a2e'),
      children: [badge, chip],
    });
    stripRedundantSectionFills(root);
    expect((badge as PenNode & { fill?: unknown }).fill).toEqual(solidFill('#0A0A0A'));
    expect((chip as PenNode & { fill?: unknown }).fill).toEqual(solidFill('#0A0A0A'));
  });

  it('does not strip a fill that is clearly distinct from root (intentional)', () => {
    // #FF5733 与 root 的 #1a1a2e 完全不同，也不是安全黑暗 — 它可能是故意的口音/英雄部分。 Leave 吧。
    const hero = frame({
      id: 'hero',
      name: 'Hero Section',
      fill: solidFill('#FF5733'),
      children: [frame({ id: 'headline' })],
    });
    const root = frame({
      id: 'root',
      fill: solidFill('#1a1a2e'),
      children: [hero],
    });
    const changed = stripRedundantSectionFills(root);
    expect(changed).toBe(false);
    expect((hero as PenNode & { fill?: unknown }).fill).toEqual(solidFill('#FF5733'));
  });

  it('strips fills from multiple sections in one pass', () => {
    const section1 = frame({ id: 's1', fill: solidFill('#0A0A0A') });
    const section2 = frame({ id: 's2', fill: solidFill('#0A0A0A') });
    const section3 = frame({ id: 's3', fill: solidFill('#0A0A0A') });
    const root = frame({
      id: 'root',
      fill: solidFill('#1a1a2e'),
      children: [section1, section2, section3],
    });
    stripRedundantSectionFills(root);
    expect((section1 as PenNode & { fill?: unknown }).fill).toBeUndefined();
    expect((section2 as PenNode & { fill?: unknown }).fill).toBeUndefined();
    expect((section3 as PenNode & { fill?: unknown }).fill).toBeUndefined();
  });

  it('does not touch deeply nested frames inside a section', () => {
    // Only 根的直接子级被视为“节级别”。具有相同颜色的嵌套三层深度的卡片应该单独保留 - 它不是顶级部分。
    const deepCard = frame({
      id: 'deep-card',
      role: 'card',
      fill: solidFill('#0A0A0A'),
    });
    const middle = frame({ id: 'middle', children: [deepCard] });
    const section = frame({
      id: 'section',
      fill: solidFill('#0A0A0A'),
      children: [middle],
    });
    const root = frame({
      id: 'root',
      fill: solidFill('#1a1a2e'),
      children: [section],
    });
    stripRedundantSectionFills(root);
    // Section（直接子级）被剥离
    expect((section as PenNode & { fill?: unknown }).fill).toBeUndefined();
    // Deep 卡被单独留下
    expect((deepCard as PenNode & { fill?: unknown }).fill).toEqual(solidFill('#0A0A0A'));
  });

  it('returns false when there is nothing to strip', () => {
    const root = frame({
      id: 'root',
      fill: solidFill('#1a1a2e'),
      children: [
        frame({ id: 's1' }), // 无填充
        frame({
          id: 'card1',
          role: 'card',
          fill: solidFill('#0A0A0A'), // 卡保护
        }),
      ],
    });
    const changed = stripRedundantSectionFills(root);
    expect(changed).toBe(false);
  });

  it('handles a root frame without a fill (treats only safe-dark sections)', () => {
    // Root 没有填充；我们仍然剥离带有安全深色“默认”填充的部分，因为这些部分几乎肯定是对冲缺失背景规格的子代理。
    const section = frame({
      id: 'sec',
      fill: solidFill('#0A0A0A'),
    });
    const root = frame({
      id: 'root',
      children: [section],
    });
    stripRedundantSectionFills(root);
    expect((section as PenNode & { fill?: unknown }).fill).toBeUndefined();
  });

  it('is strictly non-recursive: never touches grandchildren even when caller mis-targets a card', () => {
    // Defensive：如果调用者不小心给了我们一个卡片框架而不是页面根，我们必须 NOT 递归到它。 Only
    // 所传递节点的直接子节点会被考虑——并且作为卡片的 DIRECT 子节点的卡头（无角色，安全深色填充）仍然是公平的游戏，但任何更深层次的内
    // 容都不会受到影响。
    const deepInner = frame({
      id: 'deep',
      // 没有角色，安全黑暗——通常会被剥夺，但要低两级，所以必须生存
      fill: solidFill('#0A0A0A'),
    });
    const cardBody = frame({ id: 'body', children: [deepInner] });
    const cardHeader = frame({
      id: 'header',
      // 没有角色，安全黑暗 - 错误定位的父级的直接子级，因此仍然会被剥夺（调用者有错）
      fill: solidFill('#0A0A0A'),
    });
    const card = frame({
      id: 'card',
      role: 'card',
      fill: solidFill('#141414'),
      children: [cardHeader, cardBody],
    });
    // Deliberately 错误定位卡（不是页面根目录）。 This 必须使 NOT 崩溃，并且 NOT 必须递归到 cardBody
    // 的孙子中。
    stripRedundantSectionFills(card);
    // Card 本身未受影响（我们从不触及传递的节点）
    expect((card as PenNode & { fill?: unknown }).fill).toEqual(solidFill('#141414'));
    // deepInner 幸存下来是因为 strip 是严格非递归的
    expect((deepInner as PenNode & { fill?: unknown }).fill).toEqual(solidFill('#0A0A0A'));
  });

  it('strips stale #FFFFFF section fills on a dark root (legacy alternation residue)', () => {
    // Regression 守卫 2026 年 4 月 15 日：遗留的 fixSectionAlternation 在未填充部分上绘制
    // #FFFFFF / #F8FAFC 运行，无论页面主题如何。 After 黑人父母登陆的交替跳跃，陈旧的文档（和弱模型树篱）仍然
    // 携带那些白人。 stripRedundantSectionFills 现在必须清理它们。
    const section1 = frame({
      id: 's1',
      name: 'Hero',
      role: 'hero',
      fill: solidFill('#FFFFFF'),
    });
    const section2 = frame({
      id: 's2',
      name: 'Stats',
      role: 'stats-section',
      fill: solidFill('#F8FAFC'),
    });
    const section3 = frame({
      id: 's3',
      name: 'CTA',
      role: 'cta-section',
      fill: solidFill('#FFFFFF'),
    });
    const root = frame({
      id: 'root',
      fill: solidFill('#111111'),
      children: [section1, section2, section3],
    });
    const changed = stripRedundantSectionFills(root);
    expect(changed).toBe(true);
    expect((section1 as PenNode & { fill?: unknown }).fill).toBeUndefined();
    expect((section2 as PenNode & { fill?: unknown }).fill).toBeUndefined();
    expect((section3 as PenNode & { fill?: unknown }).fill).toBeUndefined();
  });

  it('strips a safe-light hedge even when the root has no fill', () => {
    // 现有的“没有填充的根框架”深色案例的 Mirror：节根上的裸#FFFFFF
    // 几乎肯定是子代理对冲缺失的背景规范，而不是故意的选择。
    const section = frame({
      id: 'sec',
      fill: solidFill('#FAFAFA'),
    });
    const root = frame({
      id: 'root',
      children: [section],
    });
    stripRedundantSectionFills(root);
    expect((section as PenNode & { fill?: unknown }).fill).toBeUndefined();
  });

  it('reproduces the M2.7 health-tracker case', () => {
    // Direct 实际故障重现：根#1a1a2e，六个部分根全部硬编码#0a0a0a，包括一张真实卡。 The
    // 六部分填充被剥离，卡片保留其填充。
    const root = frame({
      id: 'root-frame',
      name: 'Health Dashboard',
      fill: solidFill('#1a1a2e'),
      children: [
        frame({ id: 'header-root', name: 'Greeting Header', fill: solidFill('#0A0A0A') }),
        frame({
          id: 'activityRings-root',
          name: 'Activity Rings Section',
          fill: solidFill('#0A0A0A'),
        }),
        frame({
          id: 'heartRate-root',
          name: 'Heart Rate Card Section',
          fill: solidFill('#0A0A0A'),
        }),
        frame({
          id: 'workoutChart-root',
          name: 'Weekly Workout Chart',
          fill: solidFill('#0A0A0A'),
        }),
        frame({
          id: 'upcomingWorkouts-root',
          name: 'Upcoming Workouts',
          fill: solidFill('#0A0A0A'),
        }),
        frame({ id: 'bottomNav-root', name: 'Bottom Tab Bar', fill: solidFill('#0A0A0A') }),
      ],
    });
    const changed = stripRedundantSectionFills(root);
    expect(changed).toBe(true);
    const kids = (root as PenNode & { children: PenNode[] }).children;
    for (const section of kids) {
      expect((section as PenNode & { fill?: unknown }).fill).toBeUndefined();
    }
  });
});
