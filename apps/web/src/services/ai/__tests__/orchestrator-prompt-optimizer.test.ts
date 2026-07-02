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

  it('detects component prompts and emits a 400-wide single-subtask plan (Type 0)', () => {
    // Regression for Codex stop-time review: fallback path must not classify
    // "X card" / "X badge" prompts as landing-page (1200x0) when AI parsing
    // fails — that produces a desktop-wide page with multi-section sub-agents
    // for what was meant to be a single 400px card.
    const plan = buildFallbackPlanFromPrompt('design a clean profile card with avatar');
    expect(plan.rootFrame.width).toBe(400);
    expect(plan.rootFrame.height).toBe(0);
    expect(plan.subtasks).toHaveLength(1);
    expect(plan.subtasks[0]?.label).toBe('Component');
    expect(plan.subtasks[0]?.region.height).toBe(200);
  });

  // Codex review #4: cover every documented Type 0 trigger from
  // pen-ai-skills/skills/phases/planning/design-type.md so the fallback
  // doesn't silently route "design a primary button" to a 1200px landing
  // page when AI parsing fails.
  it.each([
    ['design a profile card', 'card'],
    ['design a 卡片', '卡片'],
    ['design a primary button', 'button'],
    ['design a status badge', 'badge'],
    ['design a category chip', 'chip'],
    ['design a price tag', 'tag'],
    ['design a setting toggle', 'toggle'],
    ['design a confirm dialog', 'dialog'],
    ['design a tooltip with arrow', 'tooltip'],
    ['design a popover with menu', 'popover'],
    ['design a bottom sheet', 'sheet'],
    ['design a stat tile', 'tile'],
    ['design a notification row', 'row'],
    ['design an inbox item', 'item'],
    ['design a status label', 'label'],
    ['design a segmented selector', 'selector'],
    ['design a side panel', 'panel'],
    ['design a metric widget', 'widget'],
    ['design an avatar with initial', 'avatar'],
    ['design a step stepper', 'stepper'],
    ['design a revenue stat', 'stat'],
    ['design a metric for sales', 'metric'],
    ['design a pie chart', 'chart'],
  ])('classifies "%s" as Type 0 component (matches "%s")', (prompt) => {
    const plan = buildFallbackPlanFromPrompt(prompt);
    expect(plan.rootFrame.width).toBe(400);
    expect(plan.rootFrame.height).toBe(0);
  });

  it.each([
    ['design a card screen page'],
    ['design a profile page'],
    ['design a settings screen'],
    ['design a mobile login screen'],
    ['design a dashboard home page'],
    ['design an onboarding flow'],
    // Codex review #5: dashboard / admin / workspace prompts that ALSO
    // name a tile / panel / chart / metric must route to desktop-screen,
    // not Type 0. These are the anchors we lost when broadening the
    // trigger noun list to cover the full design-type.md catalog.
    ['design an admin dashboard with metric tiles'],
    ['design a dashboard with charts and stats'],
    ['design an admin panel for user management'],
    ['design a workspace with side panel and metrics'],
    ['设计一个后台管理页面 with 卡片'],
    ['design a mobile profile card screen'], // mobile keyword wins
    ['design a phone home screen with badge'], // phone keyword wins
  ])('does NOT misclassify "%s" as a component', (prompt) => {
    const plan = buildFallbackPlanFromPrompt(prompt);
    expect(plan.rootFrame.width).not.toBe(400);
  });

  // Codex review #6: keywords disqualifying a component fallback MUST
  // also route the prompt to the right non-component preset. Otherwise
  // "design a workspace with side panel" skipped component AND skipped
  // dashboard (the regex only matched dashboard|admin|管理|后台|控制台)
  // and fell through to landing-page (1200×0), which is the wrong shape
  // for a workspace UI.
  it.each([
    ['design a workspace with side panel and metrics'],
    ['design a console with charts and live metrics'],
    ['设计一个工作台 with charts'],
    ['设计一个工作区 with 卡片'],
  ])('classifies "%s" as desktop-screen (1200×800), not landing-page', (prompt) => {
    const plan = buildFallbackPlanFromPrompt(prompt);
    expect(plan.rootFrame.width).toBe(1200);
    expect(plan.rootFrame.height).toBe(800); // desktop-screen preset has rootHeight=800
    // 3 default sections (Header / Main Content / Actions) — not the
    // 4 sections of landing-page (Header / Main / Supporting / Footer).
    expect(plan.subtasks).toHaveLength(3);
    expect(plan.subtasks.map((st) => st.label)).toEqual(['Header', 'Main Content', 'Actions']);
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
