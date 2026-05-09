import { describe, it, expect, vi } from 'vitest';
import type { OrchestratorPlan } from '../ai-types';
import type { PenNode } from '@zseven-w/pen-types';

// Mock canvas-text-measure to avoid alias resolution pulling browser-only deps.
vi.mock('@/canvas/canvas-text-measure', () => ({
  estimateLineWidth: () => 0,
  estimateTextHeight: () => 0,
  defaultLineHeight: () => 1.2,
  hasCjkText: () => false,
}));

vi.mock('@/stores/document-store', () => ({
  useDocumentStore: {
    getState: () => ({ getNodeById: () => undefined, moveNode: () => {}, removeNode: () => {} }),
  },
  DEFAULT_FRAME_ID: 'page',
  createEmptyDocument: () => ({}),
}));

import { shouldUnwrapSingleComponentSectionRoot } from '../orchestrator';

function makePlan(rootW: number, rootH: number, subtaskCount = 1): OrchestratorPlan {
  return {
    rootFrame: {
      id: 'page',
      name: 'Notification Card',
      width: rootW,
      height: rootH,
      layout: 'vertical',
    },
    subtasks: Array.from({ length: subtaskCount }, (_, i) => ({
      id: `s-${i}`,
      label: 'Component',
      region: { width: rootW, height: 200 },
      idPrefix: `s-${i}`,
      parentFrameId: 'page',
    })),
  } as unknown as OrchestratorPlan;
}

function makeFrame(props: Partial<PenNode> & { id: string; name?: string }): PenNode {
  return {
    type: 'frame',
    x: 0,
    y: 0,
    width: 400,
    height: 200,
    layout: 'vertical',
    children: [],
    ...props,
  } as unknown as PenNode;
}

describe('shouldUnwrapSingleComponentSectionRoot', () => {
  // Trip-wires the unwrap pass added in 2026-05-09 (commit dd8eb0eb /
  // refactored to a pure helper here). The 5 negative cases below
  // ensure we never silently flatten a multi-section page or a real
  // mobile screen — they're the load-bearing "do nothing" guards.

  it('unwraps when wrapper id ends with "-root"', () => {
    const plan = makePlan(400, 0);
    const root = makeFrame({
      id: 'page',
      name: 'Notification Card',
      children: [
        makeFrame({
          id: 'notification-root',
          name: 'Notification Card',
          children: [makeFrame({ id: 'icon' }), makeFrame({ id: 'text' })],
        }),
      ],
    });
    expect(shouldUnwrapSingleComponentSectionRoot(plan, root)).toBe(true);
  });

  it('unwraps when wrapper id ends with "-section"', () => {
    const plan = makePlan(400, 0);
    const root = makeFrame({
      id: 'page',
      name: 'Profile Card',
      children: [
        makeFrame({
          id: 'profile-section',
          name: 'Section A',
          children: [makeFrame({ id: 'avatar' })],
        }),
      ],
    });
    expect(shouldUnwrapSingleComponentSectionRoot(plan, root)).toBe(true);
  });

  it('unwraps when wrapper name copies parent root name', () => {
    const plan = makePlan(400, 0);
    const root = makeFrame({
      id: 'page',
      name: 'Notification Card',
      children: [
        makeFrame({
          id: 'random-id',
          name: 'Notification Card',
          children: [makeFrame({ id: 'inner' })],
        }),
      ],
    });
    expect(shouldUnwrapSingleComponentSectionRoot(plan, root)).toBe(true);
  });

  it('does NOT unwrap a multi-section plan', () => {
    const plan = makePlan(1200, 0, 6); // landing-page shape
    const root = makeFrame({
      id: 'page',
      name: 'Page',
      children: [makeFrame({ id: 's-0-root', name: 'Hero', children: [makeFrame({ id: 'a' })] })],
    });
    expect(shouldUnwrapSingleComponentSectionRoot(plan, root)).toBe(false);
  });

  it('does NOT unwrap a mobile screen (height >= 480)', () => {
    const plan = makePlan(375, 812);
    const root = makeFrame({
      id: 'page',
      name: 'Login',
      children: [
        makeFrame({ id: 'login-root', name: 'Login', children: [makeFrame({ id: 'a' })] }),
      ],
    });
    expect(shouldUnwrapSingleComponentSectionRoot(plan, root)).toBe(false);
  });

  it('does NOT unwrap a desktop plan (width > 480)', () => {
    const plan = makePlan(1200, 0);
    const root = makeFrame({
      id: 'page',
      name: 'Page',
      children: [makeFrame({ id: 's-0-root', name: 'Hero', children: [makeFrame({ id: 'a' })] })],
    });
    expect(shouldUnwrapSingleComponentSectionRoot(plan, root)).toBe(false);
  });

  it('does NOT unwrap when root has 0 or >1 children', () => {
    const plan = makePlan(400, 0);
    const empty = makeFrame({ id: 'page', name: 'X', children: [] });
    expect(shouldUnwrapSingleComponentSectionRoot(plan, empty)).toBe(false);
    const multi = makeFrame({
      id: 'page',
      name: 'X',
      children: [makeFrame({ id: 'a' }), makeFrame({ id: 'b' })],
    });
    expect(shouldUnwrapSingleComponentSectionRoot(plan, multi)).toBe(false);
  });

  it('does NOT unwrap when wrapper has unrelated id and name', () => {
    const plan = makePlan(400, 0);
    const root = makeFrame({
      id: 'page',
      name: 'Notification Card',
      children: [
        makeFrame({
          id: 'random-id',
          name: 'Header',
          children: [makeFrame({ id: 'inner' })],
        }),
      ],
    });
    expect(shouldUnwrapSingleComponentSectionRoot(plan, root)).toBe(false);
  });

  it('does NOT unwrap when wrapper itself has no children', () => {
    const plan = makePlan(400, 0);
    const root = makeFrame({
      id: 'page',
      name: 'Notification Card',
      children: [makeFrame({ id: 'notification-root', name: 'Notification Card', children: [] })],
    });
    expect(shouldUnwrapSingleComponentSectionRoot(plan, root)).toBe(false);
  });

  it('does NOT unwrap when the only child is non-frame (text / icon)', () => {
    const plan = makePlan(400, 0);
    const root = makeFrame({
      id: 'page',
      name: 'Stat',
      children: [
        {
          type: 'text',
          id: 'label',
          name: 'Label',
          x: 0,
          y: 0,
          width: 100,
          height: 20,
          content: 'Hello',
        } as unknown as PenNode,
      ],
    });
    expect(shouldUnwrapSingleComponentSectionRoot(plan, root)).toBe(false);
  });
});
