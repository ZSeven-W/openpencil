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
    // Root has #1a1a2e (deep navy), section has #0A0A0A (near-black safe
    // dark) — the classic M2.7 failure where the model picks a "safe"
    // dark for every section root, hiding the intended root background.
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
    // #FF5733 is nothing like root's #1a1a2e and is not a safe-dark — it
    // is probably a deliberate accent / hero section. Leave it.
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
    // Only direct children of the root are considered "section level". A
    // card nested three levels deep with the same color should be left
    // alone — it is not a top-level section.
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
    // Section (direct child) is stripped
    expect((section as PenNode & { fill?: unknown }).fill).toBeUndefined();
    // Deep card is left alone
    expect((deepCard as PenNode & { fill?: unknown }).fill).toEqual(solidFill('#0A0A0A'));
  });

  it('returns false when there is nothing to strip', () => {
    const root = frame({
      id: 'root',
      fill: solidFill('#1a1a2e'),
      children: [
        frame({ id: 's1' }), // no fill
        frame({
          id: 'card1',
          role: 'card',
          fill: solidFill('#0A0A0A'), // card protected
        }),
      ],
    });
    const changed = stripRedundantSectionFills(root);
    expect(changed).toBe(false);
  });

  it('handles a root frame without a fill (treats only safe-dark sections)', () => {
    // Root has no fill; we still strip sections that carry a safe-dark
    // "default" fill, because those are almost certainly the sub-agent
    // hedging against a missing background spec.
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
    // Defensive: if a caller accidentally hands us a card frame instead of
    // the page root, we must NOT recurse into it. Only the direct children
    // of the passed node are ever considered — and a card header (no role,
    // safe-dark fill) that is a DIRECT child of a card is still fair game,
    // but anything deeper is untouched.
    const deepInner = frame({
      id: 'deep',
      // no role, safe-dark — would normally be stripped, but is two levels
      // down so must survive
      fill: solidFill('#0A0A0A'),
    });
    const cardBody = frame({ id: 'body', children: [deepInner] });
    const cardHeader = frame({
      id: 'header',
      // no role, safe-dark — direct child of the mis-targeted parent, so
      // will still be stripped (the caller is at fault)
      fill: solidFill('#0A0A0A'),
    });
    const card = frame({
      id: 'card',
      role: 'card',
      fill: solidFill('#141414'),
      children: [cardHeader, cardBody],
    });
    // Deliberately mis-target the card (not the page root). This must NOT
    // crash and must NOT recurse into cardBody's grandchildren.
    stripRedundantSectionFills(card);
    // Card itself is untouched (we never touch the passed node)
    expect((card as PenNode & { fill?: unknown }).fill).toEqual(solidFill('#141414'));
    // deepInner survives because strip is strictly non-recursive
    expect((deepInner as PenNode & { fill?: unknown }).fill).toEqual(solidFill('#0A0A0A'));
  });

  it('strips stale #FFFFFF section fills on a dark root (legacy alternation residue)', () => {
    // Regression guard for 2026-04-15: the legacy fixSectionAlternation
    // painted #FFFFFF / #F8FAFC on unfilled section runs regardless of
    // page theme. After the alternation skip for dark parents landed,
    // stale docs (and weak-model hedges) still carry those whites.
    // stripRedundantSectionFills must now clean them up.
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
    // Mirror of the existing "root frame without a fill" dark case: a
    // bare #FFFFFF on a section root is almost certainly the sub-agent
    // hedging against a missing background spec, not a deliberate choice.
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
    // Direct repro of the actual failure: root #1a1a2e, six section roots
    // all hardcoded #0A0A0A, including one real card. The six section
    // fills get stripped, the card keeps its fill.
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
