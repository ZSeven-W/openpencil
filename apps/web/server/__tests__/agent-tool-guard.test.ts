import { describe, expect, it } from 'vitest';
import {
  shouldShortCircuitPlanLayout,
  updateLayoutSessionState,
  type LayoutSessionState,
} from '../utils/agent-tool-guard';

function createSessionState(): LayoutSessionState {
  return {
    layoutPhase: 'idle',
    layoutRootId: null,
  };
}

describe('agent tool guard', () => {
  it('does not short-circuit the first plan_layout call', () => {
    const session = createSessionState();

    const result = shouldShortCircuitPlanLayout(session, 'plan_layout', {
      prompt: 'mobile food app homepage',
    });

    expect(result).toBeNull();
  });

  it('short-circuits repeated plan_layout calls once layout exists', () => {
    const session: LayoutSessionState = {
      layoutPhase: 'layout_done',
      layoutRootId: 'root-1',
    };

    const result = shouldShortCircuitPlanLayout(session, 'plan_layout', {
      prompt: 'mobile food app homepage',
    });

    expect(result).toMatchObject({
      success: false,
      data: { rootFrameId: 'root-1' },
    });
    expect(result?.error).toContain('Use batch_insert or insert_node');
    expect(result?.error).toContain('"newRoot": true');
  });

  it('allows repeated plan_layout when newRoot is explicitly requested', () => {
    const session: LayoutSessionState = {
      layoutPhase: 'layout_done',
      layoutRootId: 'root-1',
    };

    const result = shouldShortCircuitPlanLayout(session, 'plan_layout', {
      prompt: 'another screen',
      newRoot: true,
    });

    expect(result).toBeNull();
  });

  it('tracks plan_layout success as layout_done with rootFrameId', () => {
    const session = createSessionState();

    updateLayoutSessionState(session, 'plan_layout', {
      success: true,
      data: { rootFrameId: 'root-1' },
    });

    expect(session).toEqual({
      layoutPhase: 'layout_done',
      layoutRootId: 'root-1',
    });
  });

  it('tracks content insertion success as content_started', () => {
    const session: LayoutSessionState = {
      layoutPhase: 'layout_done',
      layoutRootId: 'root-1',
    };

    updateLayoutSessionState(session, 'batch_insert', { success: true });
    expect(session.layoutPhase).toBe('content_started');

    session.layoutPhase = 'layout_done';
    updateLayoutSessionState(session, 'insert_node', { success: true });
    expect(session.layoutPhase).toBe('content_started');
  });
});
