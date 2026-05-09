import { describe, it, expect } from 'vitest';
import type { PenNode, PenFill } from '@zseven-w/pen-types';
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

  it('strips a misrolled section wrapper (search-bar role with a nested input child)', () => {
    // Real repro: MiniMax-M2.7 emits Search Bar(role=search-bar)
    // > Search Input Container(role=input,fill=$color-surface). The OUTER
    // wrapper labeled 'search-bar' is actually a section, not the atom —
    // its safe-light hedge fill should still be stripped because the inner
    // child carries the real component fill.
    const innerInput = frame({
      id: 'search-input',
      name: 'Search Input Container',
      role: 'input',
      fill: solidFill('#FFFFFF'),
    });
    const wrapper = frame({
      id: 'search-bar-wrapper',
      name: 'Search Bar',
      role: 'search-bar',
      fill: solidFill('#F8FAFC'), // a safe-light hedge
      children: [innerInput],
    });
    const root = frame({
      id: 'root-frame',
      fill: solidFill('#FFF8F0'),
      children: [wrapper],
    });
    const changed = stripRedundantSectionFills(root);
    expect(changed).toBe(true);
    expect((wrapper as PenNode & { fill?: unknown }).fill).toBeUndefined();
    // Inner input keeps its fill — it's the actual atom.
    expect((innerInput as PenNode & { fill?: unknown }).fill).toEqual(solidFill('#FFFFFF'));
  });

  it('preserves a real search-bar atom with no inner-component children (not a wrapper)', () => {
    // Counter-case: a search bar that is the actual atom (no nested input/
    // search-bar/etc child) — its fill is intentional and must be kept.
    const realSearchBar = frame({
      id: 'real-search',
      name: 'Search Bar',
      role: 'search-bar',
      fill: solidFill('#F1F5F9'),
      children: [
        frame({ id: 'icon', type: 'frame' }), // no role / no fill
      ],
    });
    const root = frame({
      id: 'root',
      fill: solidFill('#FFFFFF'),
      children: [realSearchBar],
    });
    stripRedundantSectionFills(root);
    expect((realSearchBar as PenNode & { fill?: unknown }).fill).toEqual(solidFill('#F1F5F9'));
  });

  it('keeps a card fill when the card legitimately contains a filled button child', () => {
    // Critical counter-case: a `card` is a CONTAINER role, not atomic. It
    // legitimately holds buttons/badges/chips with their own fills. The
    // wrapper-detection must NOT fire for cards — even if the card's fill
    // is in SAFE_LIGHT, that fill is the card's intended surface color.
    const ctaButton = frame({
      id: 'cta',
      name: 'Order Now',
      role: 'button',
      fill: solidFill('#F97316'), // accent-colored CTA inside the card
    });
    const card = frame({
      id: 'restaurant-card',
      name: 'Restaurant Card',
      role: 'card',
      fill: solidFill('#FFFFFF'), // SAFE_LIGHT — would be stripped if card were treated as wrapper
      children: [ctaButton],
    });
    const root = frame({
      id: 'root',
      fill: solidFill('#FFF8F0'),
      children: [card],
    });
    stripRedundantSectionFills(root);
    expect((card as PenNode & { fill?: unknown }).fill).toEqual(solidFill('#FFFFFF'));
    expect((ctaButton as PenNode & { fill?: unknown }).fill).toEqual(solidFill('#F97316'));
  });

  it('keeps a pricing-card fill even when it contains badges and buttons with fills', () => {
    const ribbon = frame({
      id: 'ribbon',
      role: 'badge',
      fill: solidFill('#F97316'),
    });
    const cta = frame({
      id: 'cta',
      role: 'button',
      fill: solidFill('#0F172A'),
    });
    const pricingCard = frame({
      id: 'plan',
      name: 'Pro Plan',
      role: 'pricing-card',
      fill: solidFill('#FFFFFF'),
      children: [ribbon, cta],
    });
    const root = frame({
      id: 'root',
      fill: solidFill('#F8FAFC'),
      children: [pricingCard],
    });
    stripRedundantSectionFills(root);
    expect((pricingCard as PenNode & { fill?: unknown }).fill).toEqual(solidFill('#FFFFFF'));
  });

  it('keeps an input fill when it contains a trailing icon-button (clear/reveal)', () => {
    // Critical counter-case: an `input` is a primary atomic with a fill of
    // #FFFFFF (a SAFE_LIGHT hedge). It legitimately holds a trailing
    // icon-button (clear, reveal-password, voice). Wrapper detection must
    // NOT fire — icon-button is a SECONDARY atomic that doesn't signal
    // wrapper-ness.
    const clearButton = frame({
      id: 'clear-btn',
      role: 'icon-button',
      fill: solidFill('#F1F5F9'),
    });
    const input = frame({
      id: 'email-input',
      name: 'Email Input',
      role: 'input',
      fill: solidFill('#FFFFFF'),
      children: [clearButton],
    });
    const root = frame({
      id: 'root',
      fill: solidFill('#FFF8F0'),
      children: [input],
    });
    stripRedundantSectionFills(root);
    expect((input as PenNode & { fill?: unknown }).fill).toEqual(solidFill('#FFFFFF'));
    expect((clearButton as PenNode & { fill?: unknown }).fill).toEqual(solidFill('#F1F5F9'));
  });

  it('keeps a search-bar atom fill when it contains a voice/clear button', () => {
    // The actual atom case: search-bar that owns a voice-search icon-
    // button (filled accent). icon-button is SECONDARY atomic so the
    // search-bar is treated as the real atom, not a wrapper.
    const voiceBtn = frame({
      id: 'voice-btn',
      role: 'icon-button',
      fill: solidFill('#F97316'),
    });
    const realSearchBar = frame({
      id: 'real-search',
      role: 'search-bar',
      fill: solidFill('#FFFFFF'),
      children: [voiceBtn],
    });
    const root = frame({
      id: 'root',
      fill: solidFill('#FFF8F0'),
      children: [realSearchBar],
    });
    stripRedundantSectionFills(root);
    expect((realSearchBar as PenNode & { fill?: unknown }).fill).toEqual(solidFill('#FFFFFF'));
    expect((voiceBtn as PenNode & { fill?: unknown }).fill).toEqual(solidFill('#F97316'));
  });

  it('keeps a bottom-tab-bar / navbar fill on a cream root (white surface separation)', () => {
    // Real repro from food-app brief: bottom navigation row carries
    // fill=#FFFFFF to visually separate it from the cream #FFF8F0 root
    // background. Earlier the navbar role wasn't in PROTECTED_ROLES, so
    // the SAFE_LIGHT_HEXES branch stripped the white fill and the nav
    // disappeared into the page background.
    const navbar = frame({
      id: 'bottom-nav',
      role: 'navbar',
      fill: solidFill('#FFFFFF'),
      children: [
        frame({ id: 'home', role: 'icon-button' }),
        frame({ id: 'search', role: 'icon-button' }),
        frame({ id: 'orders', role: 'icon-button' }),
      ],
    });
    const root = frame({
      id: 'root',
      fill: solidFill('#FFF8F0'),
      children: [navbar],
    });
    stripRedundantSectionFills(root);
    expect((navbar as PenNode & { fill?: unknown }).fill).toEqual(solidFill('#FFFFFF'));
  });

  it('keeps a banner fill when it contains a nested card with its own fill', () => {
    // banner > card composition — banner is CONTAINER role, fill is its
    // intentional gradient/accent surface.
    const innerCard = frame({
      id: 'inner-card',
      role: 'card',
      fill: solidFill('#FFFFFF'),
    });
    const banner = frame({
      id: 'banner',
      role: 'banner',
      fill: solidFill('#F97316'),
      children: [innerCard],
    });
    const root = frame({
      id: 'root',
      fill: solidFill('#F8FAFC'),
      children: [banner],
    });
    stripRedundantSectionFills(root);
    expect((banner as PenNode & { fill?: unknown }).fill).toEqual(solidFill('#F97316'));
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

  // Container-role wrapper detection (2026-05-10 user report).
  // The food-app "Featured" section landed with a black bg on a cream
  // page because the model marked the wrapper role='card' AND the wrapper
  // held 3 child frames each with role='card'. The original PROTECTED_ROLES
  // gate kept the outer black bg untouched (cards legitimately have fills).
  // The new hasMultipleSameRoleChildren branch identifies the misroll and
  // strips the hedge fill — the 3 inner restaurant cards keep their own
  // fills.
  it("strips a 'card' wrapper that holds 2+ same-role children (Featured-block misroll)", () => {
    const innerCard1 = frame({
      id: 'r1',
      name: 'Bella Napoli',
      role: 'card',
      fill: solidFill('#FFFFFF'),
    });
    const innerCard2 = frame({
      id: 'r2',
      name: 'Burger House',
      role: 'card',
      fill: solidFill('#FFFFFF'),
    });
    const innerCard3 = frame({
      id: 'r3',
      name: 'Sakura Sushi',
      role: 'card',
      fill: solidFill('#FFFFFF'),
    });
    const featured = frame({
      id: 'featured',
      name: 'Featured',
      role: 'card', // misroll — actually a section wrapper
      fill: solidFill('#000000'),
      children: [innerCard1, innerCard2, innerCard3],
    });
    const root = frame({
      id: 'root',
      fill: solidFill('#FFF8F0'), // cream page bg
      children: [featured],
    });
    const changed = stripRedundantSectionFills(root);
    expect(changed).toBe(true);
    // Wrapper's fill stripped — section now inherits page bg
    expect((featured as PenNode & { fill?: unknown }).fill).toBeUndefined();
    // Inner cards preserved — their own surface fills are intentional
    expect((innerCard1 as PenNode & { fill?: PenFill[] }).fill).toEqual(solidFill('#FFFFFF'));
    expect((innerCard2 as PenNode & { fill?: PenFill[] }).fill).toEqual(solidFill('#FFFFFF'));
    expect((innerCard3 as PenNode & { fill?: PenFill[] }).fill).toEqual(solidFill('#FFFFFF'));
  });

  it('does NOT strip a real card holding ONE same-role child (e.g. card with badge inside)', () => {
    // 1 child of the same role doesn't trigger — needs ≥ 2.
    const innerBadge = frame({
      id: 'b',
      role: 'badge',
      fill: solidFill('#DBEAFE'),
    });
    const card = frame({
      id: 'c',
      role: 'card',
      fill: solidFill('#000000'),
      children: [innerBadge],
    });
    const root = frame({
      id: 'root',
      fill: solidFill('#FFF8F0'),
      children: [card],
    });
    const changed = stripRedundantSectionFills(root);
    // Card has no nested SAME-role child, so the wrapper detection is silent;
    // the strip-by-safe-dark-hex still does NOT fire because role='card' is
    // PROTECTED. The card surface stays.
    expect(changed).toBe(false);
    expect((card as PenNode & { fill?: PenFill[] }).fill).toEqual(solidFill('#000000'));
  });

  it('strips a banner wrapper holding 2+ same-role banners', () => {
    const banner1 = frame({
      id: 'b1',
      name: 'Promo 1',
      role: 'banner',
      fill: solidFill('#FF6B35'),
    });
    const banner2 = frame({
      id: 'b2',
      name: 'Promo 2',
      role: 'banner',
      fill: solidFill('#10B981'),
    });
    const wrapper = frame({
      id: 'w',
      role: 'banner',
      fill: solidFill('#0A0A0A'),
      children: [banner1, banner2],
    });
    const root = frame({
      id: 'root',
      fill: solidFill('#FFF8F0'),
      children: [wrapper],
    });
    const changed = stripRedundantSectionFills(root);
    expect(changed).toBe(true);
    expect((wrapper as PenNode & { fill?: unknown }).fill).toBeUndefined();
    // Inner banners keep their own brand fills
    expect((banner1 as PenNode & { fill?: PenFill[] }).fill).toEqual(solidFill('#FF6B35'));
    expect((banner2 as PenNode & { fill?: PenFill[] }).fill).toEqual(solidFill('#10B981'));
  });
});
