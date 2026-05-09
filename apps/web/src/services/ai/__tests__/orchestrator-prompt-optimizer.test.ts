import { describe, expect, it } from 'vitest';
import {
  buildFallbackPlanFromPrompt,
  buildCompactPlanningPrompt,
  buildPlanningStyleGuideContext,
  DESIGN_MD_STYLE_GUIDE_NAME,
  getBuiltinPlanningTimeouts,
} from '../orchestrator-prompt-optimizer';
import type { DesignMdSpec } from '@/types/design-md';

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

  it('steers the planning prompt away from brand colors when design.md has no explicit background role', () => {
    const designMd: DesignMdSpec = {
      raw: '# Test',
      visualTheme: 'sleek dark cyberpunk dashboard',
      colorPalette: [
        { name: 'Brand Coral', hex: '#FF5733', role: 'Primary CTA color' },
        { name: 'Ink', hex: '#1A1A1A', role: 'Body text' },
      ],
    };

    const ctx = buildPlanningStyleGuideContext(
      'design a dashboard',
      'claude-sonnet-4',
      'rich',
      designMd,
    );

    // No explicit background role → don't hint a palette color; provide a
    // neutral dark default and warn the model.
    const bgLine = ctx.availableStyleGuides
      .split('\n')
      .find((line) => line.startsWith('- Set rootFrame.fill'));
    expect(bgLine).toBeDefined();
    expect(bgLine).not.toContain('#FF5733');
    expect(bgLine).toContain('#111111');
    expect(ctx.availableStyleGuides).toContain('DO NOT pick a brand/CTA/accent/text color');
  });

  it('calls out surface/sidebar colors so dashboard layouts keep their layered styling', () => {
    const designMd: DesignMdSpec = {
      raw: '# Test',
      visualTheme: 'moody fintech dashboard',
      colorPalette: [
        { name: 'Main Canvas', hex: '#0A0F1C', role: 'Primary app background' },
        { name: 'Sidebar Surface', hex: '#12182A', role: 'Sidebar surface' },
        { name: 'Card Surface', hex: '#1A1F2E', role: 'Card surface' },
        { name: 'Brand', hex: '#22C55E', role: 'CTA accent' },
      ],
    };

    const ctx = buildPlanningStyleGuideContext(
      'design a finance dashboard',
      'claude-sonnet-4',
      'rich',
      designMd,
    );

    expect(ctx.availableStyleGuides).toContain('SURFACE COLORS');
    expect(ctx.availableStyleGuides).toContain('#12182A');
    expect(ctx.availableStyleGuides).toContain('#1A1F2E');
    // Page bg still resolves to the explicit Primary app background.
    const bgLine = ctx.availableStyleGuides
      .split('\n')
      .find((line) => line.startsWith('- Set rootFrame.fill'));
    expect(bgLine).toContain('#0A0F1C');
  });

  it('skips the pre-built catalog when the user has a design.md and routes design.md content instead', () => {
    const designMd: DesignMdSpec = {
      raw: '# Test',
      projectName: 'Test',
      visualTheme: 'moody athletic night-mode dashboard',
      colorPalette: [
        { name: 'Midnight Canvas', hex: '#111111', role: 'Primary app background' },
        { name: 'Vital Green', hex: '#22C55E', role: 'Active tab highlight' },
      ],
      typography: { fontFamily: 'Inter' },
    };

    const ctx = buildPlanningStyleGuideContext(
      'add a workouts screen',
      'claude-sonnet-4',
      'rich',
      designMd,
    );

    expect(ctx.metadataCount).toBe(0);
    expect(ctx.snippetCount).toBe(0);
    expect(ctx.topGuideNames).toEqual([DESIGN_MD_STYLE_GUIDE_NAME]);
    expect(ctx.availableStyleGuides).toContain('custom design system (design.md)');
    expect(ctx.availableStyleGuides).toContain(DESIGN_MD_STYLE_GUIDE_NAME);
    expect(ctx.availableStyleGuides).toContain('#111111');
    expect(ctx.availableStyleGuides).not.toContain('Available style guides (compact catalog');
  });
});

describe('buildFallbackPlanFromPrompt', () => {
  it('keeps mobile fallback checklist readable with two safe sections', () => {
    const plan = buildFallbackPlanFromPrompt('design a mobile wellness app home screen');

    expect(plan.subtasks.map((subtask) => subtask.label)).toEqual(['Top Summary', 'Main Content']);
    expect(plan.subtasks[0]?.elements).toContain('Top-of-screen summary');
    expect(plan.subtasks[1]?.elements).toContain('All remaining main UI content');
  });

  it('uses design.md background and style-guide name when designMd is present', () => {
    const designMd: DesignMdSpec = {
      raw: '# Test',
      visualTheme: 'dark athletic',
      colorPalette: [
        { name: 'Midnight Canvas', hex: '#111111', role: 'Primary app background' },
        { name: 'Accent', hex: '#22C55E', role: 'CTA accent' },
      ],
    };

    const plan = buildFallbackPlanFromPrompt('design a mobile wellness app home screen', designMd);

    expect(plan.styleGuideName).toBe(DESIGN_MD_STYLE_GUIDE_NAME);
    expect(plan.selectedStyleGuideContent).toBeUndefined();
    expect(plan.rootFrame.fill?.[0]).toMatchObject({ type: 'solid', color: '#111111' });
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

  it('injects design.md policy and background into the compact prompt', () => {
    const designMd: DesignMdSpec = {
      raw: '# Test',
      visualTheme: 'moody athletic night-mode dashboard',
      colorPalette: [
        { name: 'Midnight Canvas', hex: '#111111', role: 'Primary app background' },
        { name: 'Vital Green', hex: '#22C55E', role: 'Active tab highlight' },
      ],
      layoutPrinciples: 'Use 24px horizontal padding and 16-20px vertical gaps between cards.',
    };

    const compact = buildCompactPlanningPrompt(
      'design a mobile wellness home screen',
      'minimax-m2.7',
      designMd,
    );

    expect(compact.selectedStyleGuideName).toBe(DESIGN_MD_STYLE_GUIDE_NAME);
    expect(compact.systemPrompt).toContain(DESIGN_MD_STYLE_GUIDE_NAME);
    expect(compact.systemPrompt).toContain('#111111');
    expect(compact.systemPrompt).toContain('USER DESIGN SYSTEM');
    expect(compact.systemPrompt).toContain('LAYOUT PRINCIPLES');
  });
});
