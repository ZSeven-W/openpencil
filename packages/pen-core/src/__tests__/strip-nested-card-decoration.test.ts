import { describe, it, expect } from 'vitest';
import type { PenNode } from '@zseven-w/pen-types';
import { stripNestedCardDecoration } from '../layout/strip-nested-card-decoration';

const frame = (
  props: Partial<PenNode> & { children?: PenNode[]; role?: string; cornerRadius?: unknown },
): PenNode =>
  ({
    id: 'f',
    type: 'frame',
    ...props,
  }) as PenNode;

const shadow = () => [
  { type: 'shadow' as const, offsetX: 0, offsetY: 1, blur: 3, spread: 0, color: '#0000001A' },
];
const stroke = () => ({ thickness: 1, fill: [{ type: 'solid' as const, color: '#E2E8F0' }] });

describe('stripNestedCardDecoration', () => {
  // 2026-05-11 user-reported "Popular Restaurants" — outer `role: card`
  // had stroke + cornerRadius + 2-shadow stack, then nested an inner
  // `Card Info` frame ALSO with role:card and the same decoration.
  // Visual result: card-within-a-card "extra border".

  it('strips stroke + cornerRadius + shadow on inner card-roled frame inside an outer card', () => {
    const inner = frame({
      id: 'inner',
      role: 'card',
      cornerRadius: 12,
      stroke: stroke(),
      effects: shadow(),
    } as never);
    const outer = frame({
      id: 'outer',
      role: 'card',
      cornerRadius: 16,
      stroke: stroke(),
      effects: shadow(),
      children: [inner],
    } as never);

    const changed = stripNestedCardDecoration(outer);

    expect(changed).toBe(true);
    expect((inner as PenNode & { stroke?: unknown }).stroke).toBeUndefined();
    expect((inner as PenNode & { cornerRadius?: unknown }).cornerRadius).toBeUndefined();
    expect((inner as PenNode & { effects?: unknown }).effects).toBeUndefined();
    // Outer keeps its decoration.
    expect((outer as PenNode & { stroke?: unknown }).stroke).toBeDefined();
    expect((outer as PenNode & { cornerRadius?: unknown }).cornerRadius).toBe(16);
    expect((outer as PenNode & { effects?: unknown }).effects).toBeDefined();
  });

  it('only strips the decoration types the ancestor actually has', () => {
    // Outer has shadow only; inner has shadow + cornerRadius. Inner's
    // shadow gets stripped (ancestor also has it) but cornerRadius
    // stays (ancestor has none → not redundant).
    const inner = frame({
      id: 'inner',
      role: 'card',
      cornerRadius: 12,
      effects: shadow(),
    } as never);
    const outer = frame({
      id: 'outer',
      role: 'card',
      effects: shadow(),
      children: [inner],
    } as never);

    stripNestedCardDecoration(outer);

    expect((inner as PenNode & { effects?: unknown }).effects).toBeUndefined();
    expect((inner as PenNode & { cornerRadius?: unknown }).cornerRadius).toBe(12);
  });

  it('does NOT strip top-level decoration (no decorated ancestor)', () => {
    const card = frame({
      id: 'card',
      role: 'card',
      cornerRadius: 16,
      stroke: stroke(),
      effects: shadow(),
      children: [],
    } as never);

    const changed = stripNestedCardDecoration(card);

    expect(changed).toBe(false);
    expect((card as PenNode & { stroke?: unknown }).stroke).toBeDefined();
    expect((card as PenNode & { cornerRadius?: unknown }).cornerRadius).toBe(16);
    expect((card as PenNode & { effects?: unknown }).effects).toBeDefined();
  });

  it('preserves decoration on protected roles (button / chip / search-bar / input / badge / avatar)', () => {
    // A button or input inside a card legitimately keeps its own
    // affordance — the user expects the click target to be visually
    // distinct from the card surface.
    const protectedCases = ['button', 'chip', 'search-bar', 'input', 'badge', 'avatar', 'tag'];
    for (const role of protectedCases) {
      const inner = frame({
        id: `inner-${role}`,
        role,
        cornerRadius: 8,
        stroke: stroke(),
      } as never);
      const outer = frame({
        id: 'outer',
        role: 'card',
        stroke: stroke(),
        cornerRadius: 16,
        children: [inner],
      } as never);

      stripNestedCardDecoration(outer);

      expect(
        (inner as PenNode & { stroke?: unknown }).stroke,
        `${role} should retain its stroke`,
      ).toBeDefined();
      expect(
        (inner as PenNode & { cornerRadius?: unknown }).cornerRadius,
        `${role} should retain its cornerRadius`,
      ).toBe(8);
    }
  });

  it('does NOT touch fills (stripRedundantSectionFills handles that)', () => {
    const inner = frame({
      id: 'inner',
      role: 'card',
      stroke: stroke(),
      fill: [{ type: 'solid' as const, color: '#FFFFFF' }],
    } as never);
    const outer = frame({
      id: 'outer',
      role: 'card',
      stroke: stroke(),
      fill: [{ type: 'solid' as const, color: '#F1F5F9' }],
      children: [inner],
    } as never);

    stripNestedCardDecoration(outer);

    expect((inner as PenNode & { fill?: unknown }).fill).toBeDefined();
  });

  it('walks deep — strips on grandchild whose grandparent is decorated', () => {
    const grandchild = frame({
      id: 'gc',
      role: 'card',
      cornerRadius: 8,
      effects: shadow(),
    } as never);
    const middle = frame({
      id: 'mid',
      children: [grandchild],
    } as never);
    const outer = frame({
      id: 'outer',
      role: 'card',
      cornerRadius: 16,
      effects: shadow(),
      children: [middle],
    } as never);

    stripNestedCardDecoration(outer);

    expect((grandchild as PenNode & { cornerRadius?: unknown }).cornerRadius).toBeUndefined();
    expect((grandchild as PenNode & { effects?: unknown }).effects).toBeUndefined();
  });

  it('handles cornerRadius arrays (asymmetric per-corner)', () => {
    const inner = frame({
      id: 'inner',
      role: 'card',
      cornerRadius: [12, 12, 0, 0],
      stroke: stroke(),
    } as never);
    const outer = frame({
      id: 'outer',
      role: 'card',
      cornerRadius: 16,
      children: [inner],
    } as never);

    stripNestedCardDecoration(outer);

    expect((inner as PenNode & { cornerRadius?: unknown }).cornerRadius).toBeUndefined();
  });

  // 2026-05-11 Codex stop-hook caught: cornerRadius on a media-clipping
  // frame (clipContent: true wrapping an image / video) is doing the
  // rounding work, not stacking card decoration. Stripping it would
  // un-round the photo against the user's intent.

  it('preserves cornerRadius on a clipContent frame wrapping an image', () => {
    const image = frame({ id: 'img', type: 'image' as const } as never);
    const clipper = frame({
      id: 'clipper',
      cornerRadius: 12,
      clipContent: true,
      stroke: stroke(),
      effects: shadow(),
      children: [image],
    } as never);
    const card = frame({
      id: 'card',
      role: 'card',
      cornerRadius: 16,
      stroke: stroke(),
      effects: shadow(),
      children: [clipper],
    } as never);

    stripNestedCardDecoration(card);

    // cornerRadius preserved (media clip role).
    expect((clipper as PenNode & { cornerRadius?: unknown }).cornerRadius).toBe(12);
    // stroke + shadow still get stripped (those are pure decoration).
    expect((clipper as PenNode & { stroke?: unknown }).stroke).toBeUndefined();
    expect((clipper as PenNode & { effects?: unknown }).effects).toBeUndefined();
  });

  it('preserves cornerRadius on frames with media role (image-placeholder, thumbnail, cover)', () => {
    for (const role of ['image-placeholder', 'thumbnail', 'cover-image', 'gallery-item']) {
      const inner = frame({
        id: `inner-${role}`,
        role,
        cornerRadius: 8,
      } as never);
      const card = frame({
        id: 'card',
        role: 'card',
        cornerRadius: 16,
        children: [inner],
      } as never);

      stripNestedCardDecoration(card);

      expect(
        (inner as PenNode & { cornerRadius?: unknown }).cornerRadius,
        `${role} should retain cornerRadius`,
      ).toBe(8);
    }
  });

  it('returns false when nothing was modified', () => {
    const card = frame({
      id: 'plain-card',
      role: 'card',
      stroke: stroke(),
      cornerRadius: 8,
      children: [
        frame({ id: 'text-only', role: 'body', children: [] } as never), // no decoration
      ],
    } as never);

    const changed = stripNestedCardDecoration(card);

    expect(changed).toBe(false);
  });
});
