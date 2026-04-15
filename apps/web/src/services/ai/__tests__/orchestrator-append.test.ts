import { describe, it, expect } from 'vitest';
import { applyAppendContextToPlan } from '../orchestrator-append';
import type { OrchestratorPlan } from '../ai-types';

function basePlan(): OrchestratorPlan {
  return {
    rootFrame: { id: 'new-root', name: 'Page', width: 375, height: 812 },
    subtasks: [
      {
        id: 'status-bar',
        label: 'Status Bar',
        region: { width: 375, height: 44 },
        idPrefix: '',
        parentFrameId: null,
      },
      {
        id: 'header',
        label: 'Screen Header',
        region: { width: 375, height: 120 },
        idPrefix: '',
        parentFrameId: null,
      },
      {
        id: 'cards',
        label: 'Workout Type Cards',
        region: { width: 375, height: 200 },
        idPrefix: '',
        parentFrameId: null,
      },
    ],
  };
}

describe('applyAppendContextToPlan', () => {
  it('returns the plan unchanged when no appendContext is given', () => {
    const plan = basePlan();
    const out = applyAppendContextToPlan(plan, undefined);
    expect(out.skipRootInsertion).toBe(false);
    expect(out.skipStatusBar).toBe(false);
    expect(out.plan.rootFrame.id).toBe('new-root');
    expect(out.plan.subtasks).toHaveLength(3);
  });

  it('repoints rootFrame.id to targetParentId and drops status-bar subtasks', () => {
    const out = applyAppendContextToPlan(basePlan(), {
      targetParentId: 'content-root',
      targetWidth: 375,
      existingSectionLabels: ['Hero'],
      isMobile: true,
    });
    expect(out.skipRootInsertion).toBe(true);
    expect(out.skipStatusBar).toBe(true);
    expect(out.plan.rootFrame.id).toBe('content-root');
    expect(out.plan.subtasks.map((s) => s.id)).toEqual(['header', 'cards']);
  });

  it('propagates existingSectionLabels onto every remaining subtask', () => {
    const out = applyAppendContextToPlan(basePlan(), {
      targetParentId: 'content-root',
      targetWidth: 1200,
      existingSectionLabels: ['Navbar', 'Hero', 'Features'],
      isMobile: false,
    });
    for (const st of out.plan.subtasks) {
      expect(st.existingSectionLabels).toEqual(['Navbar', 'Hero', 'Features']);
    }
  });
});
