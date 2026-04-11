import { describe, expect, it } from 'vitest';
import {
  buildFallbackPlanFromPrompt,
  buildCompactPlanningPrompt,
  buildPlanningStyleGuideContext,
  getBuiltinPlanningTimeouts,
} from '../orchestrator-prompt-optimizer';

describe('buildPlanningStyleGuideContext', () => {
  it('lists the full guide catalog while limiting detailed snippets for basic models', () => {
    const basic = buildPlanningStyleGuideContext(
      'design a dark health and fitness mobile app',
      'minimax-m2.7',
      'rich',
    );
    const full = buildPlanningStyleGuideContext(
      'design a dark health and fitness mobile app',
      'claude-sonnet-4',
      'rich',
    );

    expect(basic.metadataCount).toBeGreaterThanOrEqual(50);
    expect(basic.availableStyleGuides).toContain('Available style guides');
    expect(basic.availableStyleGuides).toContain('Detailed references');
    expect(basic.snippetCount).toBe(4);
    expect(basic.topGuideNames.length).toBe(12);
    expect(basic.snippetGuideNames.length).toBe(4);
    expect(full.snippetCount).toBeGreaterThan(basic.snippetCount);
  });

  it('builds an even lighter minimal context without detailed snippets', () => {
    const minimal = buildPlanningStyleGuideContext(
      'design a fintech dashboard',
      'glm-4.5',
      'minimal',
    );

    expect(minimal.metadataCount).toBeGreaterThanOrEqual(50);
    expect(minimal.snippetCount).toBe(0);
    expect(minimal.snippetGuideNames).toEqual([]);
    expect(minimal.availableStyleGuides).not.toContain('Detailed references');
  });
});

describe('buildFallbackPlanFromPrompt', () => {
  it('keeps mobile fallback checklist readable with two safe sections', () => {
    const plan = buildFallbackPlanFromPrompt('design a mobile wellness app home screen');

    expect(plan.subtasks.map((subtask) => subtask.label)).toEqual(['Top Summary', 'Main Content']);
    expect(plan.subtasks[0]?.elements).toContain('Top-of-screen summary');
    expect(plan.subtasks[1]?.elements).toContain('All remaining main UI content');
  });
});

describe('getBuiltinPlanningTimeouts', () => {
  it('gives basic builtin models more runway before planner fallback', () => {
    const timeouts = getBuiltinPlanningTimeouts('minimax-m2.7');

    expect(timeouts.thinkingMode).toBe('disabled');
    expect(timeouts.noTextTimeoutMs).toBeGreaterThan(30_000);
    expect(timeouts.firstTextTimeoutMs).toBeGreaterThan(30_000);
    expect(timeouts.hardTimeoutMs).toBeGreaterThan(60_000);
  });
});

describe('buildCompactPlanningPrompt', () => {
  it('builds a short model-driven retry prompt for compact planning', () => {
    const compact = buildCompactPlanningPrompt(
      'Design a dark health and fitness tracking mobile app homepage with green accent',
      'minimax-m2.7',
    );

    expect(compact.systemPrompt).toContain('Output ONLY one JSON object');
    expect(compact.systemPrompt).toContain('This is a direct mobile screen, not a phone mockup.');
    expect(compact.selectedStyleGuideName).toBeTruthy();
    expect(compact.systemPrompt).not.toContain('Available style guides');
  });
});
