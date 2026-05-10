import { describe, it, expect } from 'vitest';
import { isMobileFullScreen } from '../orchestrator-plan-classify';
import type { OrchestratorPlan } from '../ai-types';

const plan = (width: number, height: number, subtaskCount: number): OrchestratorPlan =>
  ({
    rootFrame: { id: 'page', name: 'Page', width, height, layout: 'vertical' },
    subtasks: Array.from({ length: subtaskCount }, (_, i) => ({
      id: `s-${i}`,
      label: `Section ${i}`,
      elements: [],
      region: { width, height: 100 },
      idPrefix: '',
      parentFrameId: null,
    })),
  }) as unknown as OrchestratorPlan;

describe('isMobileFullScreen', () => {
  it('true for narrow + tall (canonical mobile page)', () => {
    expect(isMobileFullScreen(plan(375, 812, 5))).toBe(true);
  });

  it('false for wide root (desktop / landing page)', () => {
    expect(isMobileFullScreen(plan(1200, 800, 4))).toBe(false);
  });

  it('false for narrow + tiny height + single subtask (Type 0 component)', () => {
    expect(isMobileFullScreen(plan(375, 200, 1))).toBe(false);
  });

  // 2026-05-10 user report — DeepSeek "Bistro" food app generated with
  // status bar missing. Plan came back with width=375, height=0 because
  // the LLM emitted a non-numeric height that asNonNegativeNumber
  // rejected → parser fell back to landing-page preset's rootHeight=0.
  // Six structural subtasks (Header / Hero / Search / Categories /
  // Specials / BottomNav) make it clearly a mobile page, not a Type 0
  // component. The subtask-count discriminator catches this.

  it('true for narrow + height=0 with 2+ subtasks (height lost in plan coercion)', () => {
    expect(isMobileFullScreen(plan(375, 0, 6))).toBe(true);
  });

  it('true for narrow + height=0 with exactly 2 subtasks', () => {
    expect(isMobileFullScreen(plan(375, 0, 2))).toBe(true);
  });

  it('false for narrow + height=0 with single subtask (canonical Type 0)', () => {
    expect(isMobileFullScreen(plan(375, 0, 1))).toBe(false);
  });

  it('honors width boundary at 480 (exclusive)', () => {
    expect(isMobileFullScreen(plan(480, 812, 5))).toBe(true);
    expect(isMobileFullScreen(plan(481, 812, 5))).toBe(false);
  });
});
