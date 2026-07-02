import { describe, it, expect } from 'vitest';
import type { PenNode } from '@zseven-w/pen-types';
import { unwrapFakePhoneMockups } from '../layout/unwrap-fake-phone-mockup';

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

describe('unwrapFakePhoneMockups', () => {
  it('leaves a real phone mockup with 0 children alone', () => {
    const phone = frame({
      id: 'phone',
      name: 'Phone Mockup',
      cornerRadius: 32,
      width: 280,
      height: 580,
      children: [],
    });
    const root = frame({ id: 'root', children: [phone] });
    unwrapFakePhoneMockups(root);
    const kids = (root as PenNode & { children: PenNode[] }).children;
    expect(kids).toHaveLength(1);
    expect(kids[0].id).toBe('phone');
  });

  it('leaves a real phone mockup with a single placeholder child alone', () => {
    const placeholder: PenNode = {
      id: 'placeholder',
      type: 'text',
      content: 'App Preview',
    } as PenNode;
    const phone = frame({
      id: 'phone',
      name: 'Phone Mockup',
      cornerRadius: 32,
      width: 280,
      children: [placeholder],
    });
    const root = frame({ id: 'root', children: [phone] });
    unwrapFakePhoneMockups(root);
    const kids = (root as PenNode & { children: PenNode[] }).children;
    expect(kids).toHaveLength(1);
    expect(kids[0].id).toBe('phone');
  });

  it('unwraps a fake "Phone Mockup" frame with multiple children', () => {
    const fakePhone = frame({
      id: 'fake-phone',
      name: 'Phone Mockup',
      cornerRadius: 32,
      width: 260,
      height: 456,
      layout: 'horizontal',
      children: [rect('section1'), rect('section2'), rect('section3')],
    });
    const root = frame({
      id: 'root',
      children: [rect('header'), fakePhone],
    });
    unwrapFakePhoneMockups(root);
    const kids = (root as PenNode & { children: PenNode[] }).children;
    expect(kids).toHaveLength(4);
    expect(kids[0].id).toBe('header');
    expect(kids[1].id).toBe('section1');
    expect(kids[2].id).toBe('section2');
    expect(kids[3].id).toBe('section3');
    expect(kids.find((c) => c.id === 'fake-phone')).toBeUndefined();
  });

  it('detects fake mockup by visual signature even when name is generic', () => {
    const fakePhone = frame({
      id: 'wrapper',
      name: 'Container',
      cornerRadius: 32,
      width: 280,
      layout: 'vertical',
      children: [rect('a'), rect('b'), rect('c'), rect('d')],
    });
    const root = frame({ id: 'root', children: [fakePhone] });
    unwrapFakePhoneMockups(root);
    const kids = (root as PenNode & { children: PenNode[] }).children;
    expect(kids).toHaveLength(4);
    expect(kids.map((c) => c.id)).toEqual(['a', 'b', 'c', 'd']);
  });

  it('does not unwrap a card-like frame with cornerRadius < phone bezel range', () => {
    // Cards typically use cornerRadius 12-20, not 28-40.
    const card = frame({
      id: 'card',
      name: 'Stat Card',
      cornerRadius: 16,
      width: 200,
      layout: 'vertical',
      children: [rect('icon'), rect('label'), rect('value')],
    });
    const root = frame({ id: 'root', children: [card] });
    unwrapFakePhoneMockups(root);
    const kids = (root as PenNode & { children: PenNode[] }).children;
    expect(kids).toHaveLength(1);
    expect(kids[0].id).toBe('card');
  });

  it('does not unwrap a wide frame even with cornerRadius 32', () => {
    // A 600px wide hero card with cornerRadius 32 is not a phone mockup.
    const heroCard = frame({
      id: 'hero',
      name: 'Hero Card',
      cornerRadius: 32,
      width: 600,
      layout: 'vertical',
      children: [rect('a'), rect('b'), rect('c')],
    });
    const root = frame({ id: 'root', children: [heroCard] });
    unwrapFakePhoneMockups(root);
    const kids = (root as PenNode & { children: PenNode[] }).children;
    expect(kids).toHaveLength(1);
  });

  it('recurses into nested frames', () => {
    const fakePhone = frame({
      id: 'fake',
      name: 'Phone Mockup',
      cornerRadius: 32,
      width: 260,
      layout: 'horizontal',
      children: [rect('a'), rect('b')],
    });
    const inner = frame({ id: 'inner', children: [fakePhone] });
    const outer = frame({ id: 'outer', children: [inner] });
    unwrapFakePhoneMockups(outer);
    const innerKids = (inner as PenNode & { children: PenNode[] }).children;
    expect(innerKids).toHaveLength(2);
    expect(innerKids[0].id).toBe('a');
    expect(innerKids[1].id).toBe('b');
  });

  it('does nothing for non-frame leaf nodes', () => {
    const text: PenNode = { id: 't', type: 'text', content: 'hi' } as PenNode;
    expect(() => unwrapFakePhoneMockups(text)).not.toThrow();
  });

  it('returns false when no fake phone mockup is found anywhere', () => {
    const root = frame({
      id: 'root',
      layout: 'vertical',
      children: [rect('a'), rect('b'), rect('c')],
    });
    const changed = unwrapFakePhoneMockups(root);
    expect(changed).toBe(false);
  });

  it('returns true when a child fake phone mockup is unwrapped', () => {
    const fakePhone = frame({
      id: 'fake',
      name: 'Phone Mockup',
      cornerRadius: 32,
      width: 260,
      layout: 'horizontal',
      children: [rect('a'), rect('b')],
    });
    const root = frame({ id: 'root', children: [fakePhone] });
    const changed = unwrapFakePhoneMockups(root);
    expect(changed).toBe(true);
  });

  it('returns true when the root itself is sanitized', () => {
    const fakePhoneRoot = frame({
      id: 'phone',
      name: 'Phone Mockup',
      cornerRadius: 32,
      width: 260,
      layout: 'horizontal',
      children: [rect('a'), rect('b')],
    });
    const changed = unwrapFakePhoneMockups(fakePhoneRoot);
    expect(changed).toBe(true);
  });

  it('returns true when a deeply nested fake phone mockup is unwrapped', () => {
    const fake = frame({
      id: 'fake',
      name: 'Phone Mockup',
      cornerRadius: 32,
      width: 260,
      layout: 'horizontal',
      children: [rect('a')],
    });
    const inner = frame({ id: 'inner', children: [fake] });
    const outer = frame({ id: 'outer', children: [inner] });
    const changed = unwrapFakePhoneMockups(outer);
    expect(changed).toBe(true);
  });

  it('returns false for a real phone mockup with at most one child', () => {
    const realPhone = frame({
      id: 'phone',
      name: 'Phone Mockup',
      cornerRadius: 32,
      width: 280,
      height: 580,
      children: [],
    });
    const root = frame({ id: 'root', children: [realPhone] });
    const changed = unwrapFakePhoneMockups(root);
    expect(changed).toBe(false);
  });

  it('sanitizes the root node when the root itself is a fake phone mockup', () => {
    // The actual sub-agent path: bottomNav-root IS the fake wrapper. The
    // function is called on the wrapper itself, with no parent in scope.
    // We can't drop the root, so we sanitize: strip phone bezel visuals,
    // restore vertical layout + fill_container width, drop the misleading
    // name. The children stay (might be duplicates, but at least they're
    // not crushed into a 260px column).
    const fakePhoneRoot = frame({
      id: 'bottomNav-root',
      name: 'Phone Mockup',
      width: 260,
      height: 456,
      cornerRadius: 32,
      fill: [{ type: 'solid', color: '#0A0A0A' }],
      layout: 'horizontal',
      children: [rect('child1'), rect('child2'), rect('child3')],
    });
    unwrapFakePhoneMockups(fakePhoneRoot);
    const sanitized = fakePhoneRoot as PenNode & {
      width?: unknown;
      cornerRadius?: unknown;
      layout?: unknown;
      name?: unknown;
    };
    // Phone mockup visual signature must be gone
    expect(sanitized.cornerRadius).toBeUndefined();
    expect(sanitized.width).toBe('fill_container');
    expect(sanitized.layout).toBe('vertical');
    // Misleading name is cleared
    expect(sanitized.name).not.toBe('Phone Mockup');
    // Children survive — at least they will render in a normal vertical stack
    const kids = (fakePhoneRoot as PenNode & { children: PenNode[] }).children;
    expect(kids).toHaveLength(3);
    expect(kids[0].id).toBe('child1');
  });

  it('does not sanitize a real phone mockup root with 0 children', () => {
    // Important: when called on a legitimate phone mockup root (no children),
    // it must be left alone — the sanitize branch should only fire when the
    // mockup is structurally invalid (children > 1 OR layout horizontal/vertical).
    const realPhoneRoot = frame({
      id: 'phone',
      name: 'Phone Mockup',
      cornerRadius: 32,
      width: 280,
      height: 580,
      children: [],
    });
    unwrapFakePhoneMockups(realPhoneRoot);
    const node = realPhoneRoot as PenNode & {
      cornerRadius?: unknown;
      width?: unknown;
      name?: unknown;
    };
    expect(node.cornerRadius).toBe(32);
    expect(node.width).toBe(280);
    expect(node.name).toBe('Phone Mockup');
  });

  it('reproduces the M2.7 health-tracker bottomNav case', () => {
    // Direct repro of the actual failing case: bottomNav-root frame named
    // "Phone Mockup", w=260, h=456, cornerRadius=32, layout=horizontal,
    // wrapping a complete copy of the entire mobile app (6 sections).
    const fakeBottomNav = frame({
      id: 'bottomNav-root',
      name: 'Phone Mockup',
      width: 260,
      height: 456,
      cornerRadius: 32,
      fill: [{ type: 'solid', color: '#0A0A0A' }],
      layout: 'horizontal',
      children: [
        frame({ id: 'bottomNav-header', name: 'Header' }),
        frame({ id: 'bottomNav-activity-section', name: 'Activity Section' }),
        frame({ id: 'bottomNav-heart-card', name: 'Heart Rate Card' }),
        frame({ id: 'bottomNav-weekly-section', name: 'Weekly Summary' }),
        frame({ id: 'bottomNav-upcoming-section', name: 'Upcoming Section' }),
        frame({ id: 'bottomNav-tab-bar', name: 'Tab Bar' }),
      ],
    });
    const rootFrame = frame({
      id: 'root-frame',
      name: 'Health Tracker Home',
      layout: 'vertical',
      children: [
        frame({ id: 'header-root', name: 'Header Section' }),
        frame({ id: 'activityRings-root', name: 'Activity Rings Section' }),
        frame({ id: 'heartRate-root', name: 'HeartRateCard' }),
        frame({ id: 'weeklyWorkout-root', name: 'Weekly Workout Summary' }),
        frame({ id: 'upcomingWorkouts-root', name: 'Upcoming Workouts Section' }),
        fakeBottomNav,
      ],
    });
    unwrapFakePhoneMockups(rootFrame);
    const kids = (rootFrame as PenNode & { children: PenNode[] }).children;
    // Original 5 real sections + 6 unwrapped grandchildren = 11
    expect(kids).toHaveLength(11);
    // Fake wrapper is gone
    expect(kids.find((c) => c.id === 'bottomNav-root')).toBeUndefined();
    // Real tab-bar is now a direct child of root-frame
    expect(kids.find((c) => c.id === 'bottomNav-tab-bar')).toBeDefined();
  });
});
