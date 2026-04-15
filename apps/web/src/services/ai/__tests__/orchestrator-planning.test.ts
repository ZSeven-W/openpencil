import { describe, expect, it } from 'vitest';
import { filterPlanningSkillsForPrompt, parseOrchestratorResponse } from '../orchestrator-planning';
import { DESIGN_MD_STYLE_GUIDE_NAME } from '../orchestrator-prompt-optimizer';
import { extractSidebarSurfaceColor } from '../orchestrator-sidebar-color';
import type { OrchestratorPlan } from '../ai-types';
import type { DesignMdSpec } from '@/types/design-md';

describe('filterPlanningSkillsForPrompt', () => {
  const skills = [
    { meta: { name: 'decomposition' }, content: 'decomposition' },
    { meta: { name: 'landing-page-predesign' }, content: 'landing' },
    { meta: { name: 'style-guide-selector' }, content: 'style' },
  ];

  it('drops landing-page predesign for mobile app home screens', () => {
    const filtered = filterPlanningSkillsForPrompt(
      skills,
      'Design a health and fitness tracking mobile app homepage',
    );

    expect(filtered.map((skill) => skill.meta.name)).toEqual([
      'decomposition',
      'style-guide-selector',
    ]);
  });

  it('keeps landing-page predesign for marketing homepages', () => {
    const filtered = filterPlanningSkillsForPrompt(
      skills,
      'Design a marketing homepage for an AI startup',
    );

    expect(filtered.map((skill) => skill.meta.name)).toContain('landing-page-predesign');
  });
});

describe('parseOrchestratorResponse', () => {
  it('repairs near-miss planner JSON into a valid mobile plan', () => {
    const raw = JSON.stringify({
      styleGuideName: 'health-minimal-mobile-dark',
      sections: [
        {
          title: 'Greeting Header',
          elements: ['good morning text', 'avatar'],
          height: 120,
        },
        {
          name: 'Activity Overview',
          elements: 'activity ring, heart rate card, workout chart, upcoming workouts',
        },
      ],
    });

    const parsed = parseOrchestratorResponse(
      raw,
      'Design a health and fitness tracking mobile app homepage with dark background and green accent.',
    );

    expect(parsed).not.toBeNull();
    expect(parsed?.repaired).toBe(true);
    expect(parsed?.plan.rootFrame.width).toBe(375);
    expect(parsed?.plan.rootFrame.height).toBe(812);
    expect(parsed?.plan.subtasks).toHaveLength(2);
    expect(parsed?.plan.subtasks[0]).toMatchObject({
      id: 'greeting-header',
      label: 'Greeting Header',
      region: { width: 375, height: 120 },
    });
    expect(parsed?.plan.subtasks[1]?.region.width).toBe(375);
    expect(parsed?.plan.styleGuideName).toBe('health-minimal-mobile-dark');
  });

  it('accepts valid planner JSON without marking it repaired', () => {
    const raw = JSON.stringify({
      rootFrame: {
        id: 'page',
        name: 'Page',
        width: 375,
        height: 812,
        layout: 'vertical',
      },
      styleGuideName: 'health-minimal-mobile-dark',
      subtasks: [
        {
          id: 'header',
          label: 'Header',
          elements: 'greeting, avatar',
          region: { width: 375, height: 140 },
        },
      ],
    });

    const parsed = parseOrchestratorResponse(
      raw,
      'Design a health and fitness tracking mobile app homepage.',
    );

    expect(parsed).not.toBeNull();
    expect(parsed?.repaired).toBe(false);
    expect(parsed?.plan.subtasks[0]?.id).toBe('header');
  });

  it('strips stale catalog styling from the repair path when design.md is present', () => {
    const designMd: DesignMdSpec = {
      raw: '# Test',
      colorPalette: [{ name: 'Midnight Canvas', hex: '#111111', role: 'Primary app background' }],
    };

    // Invalid JSON (missing rootFrame) with stale catalog styleGuideName + fill
    const raw = JSON.stringify({
      styleGuideName: 'health-minimal-mobile-dark',
      rootFrame: { fill: [{ type: 'solid', color: '#0A0F1C' }] },
      sections: [
        { title: 'Header', elements: 'greeting, avatar', height: 120 },
        { title: 'Main', elements: 'cards' },
      ],
    });

    const parsed = parseOrchestratorResponse(raw, 'design a mobile wellness home screen', designMd);

    expect(parsed?.repaired).toBe(true);
    expect(parsed?.plan.styleGuideName).toBe(DESIGN_MD_STYLE_GUIDE_NAME);
    expect(parsed?.plan.styleGuide).toBeUndefined();
    expect((parsed?.plan.rootFrame.fill as Array<{ color?: string }> | undefined)?.[0]?.color).toBe(
      '#111111',
    );
  });

  it('strips stale catalog styling from a fully-parsed plan when design.md is present', () => {
    const designMd: DesignMdSpec = {
      raw: '# Test',
      colorPalette: [{ name: 'Midnight Canvas', hex: '#111111', role: 'Primary app background' }],
    };

    const raw = JSON.stringify({
      rootFrame: {
        id: 'page',
        name: 'Page',
        width: 375,
        height: 812,
        layout: 'vertical',
        fill: [{ type: 'solid', color: '#0A0F1C' }],
      },
      styleGuideName: 'health-minimal-mobile-dark',
      styleGuide: {
        palette: { background: '#0A0F1C', accent: '#ff0000' },
        fonts: { heading: 'Space Grotesk', body: 'Inter' },
      },
      subtasks: [
        {
          id: 'header',
          label: 'Header',
          elements: 'greeting, avatar',
          region: { width: 375, height: 140 },
        },
      ],
    });

    const parsed = parseOrchestratorResponse(raw, 'design a mobile wellness home screen', designMd);

    expect(parsed?.repaired).toBe(false);
    expect(parsed?.plan.styleGuideName).toBe(DESIGN_MD_STYLE_GUIDE_NAME);
    expect(parsed?.plan.styleGuide).toBeUndefined();
    expect((parsed?.plan.rootFrame.fill as Array<{ color?: string }> | undefined)?.[0]?.color).toBe(
      '#111111',
    );
  });

  it('does not use a surface/card color as the page background', () => {
    // design.md has a "Card surface" role but no explicit page background.
    // Painting the whole page with #1A1F2E would flatten the card distinction.
    const designMd: DesignMdSpec = {
      raw: '# Test',
      visualTheme: 'moody dark fitness dashboard',
      colorPalette: [
        { name: 'Brand Green', hex: '#22C55E', role: 'CTA accent' },
        { name: 'Card Surface', hex: '#1A1F2E', role: 'Card surface / panel surface' },
      ],
    };

    const raw = JSON.stringify({
      rootFrame: {
        id: 'page',
        name: 'Page',
        width: 375,
        height: 812,
        layout: 'vertical',
        fill: [{ type: 'solid', color: '#1A1F2E' }],
      },
      styleGuideName: 'catalog-dark',
      subtasks: [
        { id: 'header', label: 'Header', elements: 'title', region: { width: 375, height: 120 } },
      ],
    });

    const parsed = parseOrchestratorResponse(raw, 'design a mobile screen', designMd);

    // Should fall back to neutral dark (#111111), NOT the card surface color.
    expect((parsed?.plan.rootFrame.fill as Array<{ color?: string }> | undefined)?.[0]?.color).toBe(
      '#111111',
    );
  });

  it('enforces the neutral fallback on parsed planner output when design.md lacks a background role', () => {
    const designMd: DesignMdSpec = {
      raw: '# Test',
      visualTheme: 'sleek dark cyberpunk dashboard',
      colorPalette: [
        { name: 'Brand Coral', hex: '#FF5733', role: 'Primary CTA color' },
        { name: 'Ink', hex: '#1A1A1A', role: 'Body text' },
      ],
    };

    // Model returned a brand color as page background — this is exactly the
    // regression we guard against. Finalize must overwrite it with the neutral
    // default derived from visualTheme ("dark" → #111111).
    const raw = JSON.stringify({
      rootFrame: {
        id: 'page',
        name: 'Page',
        width: 375,
        height: 812,
        layout: 'vertical',
        fill: [{ type: 'solid', color: '#FF5733' }],
      },
      styleGuideName: 'light-minimal',
      subtasks: [
        { id: 'header', label: 'Header', elements: 'title', region: { width: 375, height: 120 } },
      ],
    });

    const parsed = parseOrchestratorResponse(raw, 'design a mobile screen', designMd);

    expect((parsed?.plan.rootFrame.fill as Array<{ color?: string }> | undefined)?.[0]?.color).toBe(
      '#111111',
    );
  });
});

describe('extractSidebarSurfaceColor', () => {
  const basePlan: OrchestratorPlan = {
    rootFrame: { id: 'page', name: 'Page', width: 1440, height: 900 },
    subtasks: [],
  };

  it('reads Sidebar Surface from catalog style guide content when present', () => {
    const plan: OrchestratorPlan = {
      ...basePlan,
      selectedStyleGuideContent: '| Sidebar Surface | #1E293B |\n',
    };
    expect(extractSidebarSurfaceColor(plan)).toBe('#1E293B');
  });

  it('falls back to design.md sidebar role when no style guide content is set', () => {
    const designMd: DesignMdSpec = {
      raw: '# Spec',
      colorPalette: [
        { name: 'Canvas', hex: '#0B1120', role: 'Page background' },
        { name: 'Rail', hex: '#1E293B', role: 'Sidebar navigation surface' },
        { name: 'Card', hex: '#111827', role: 'Card surface' },
      ],
    };
    expect(extractSidebarSurfaceColor(basePlan, designMd)).toBe('#1E293B');
  });

  it('falls back to a generic surface/card color when design.md has no sidebar role', () => {
    const designMd: DesignMdSpec = {
      raw: '# Spec',
      colorPalette: [
        { name: 'Canvas', hex: '#0B1120', role: 'Page background' },
        { name: 'Card', hex: '#111827', role: 'Card surface' },
      ],
    };
    expect(extractSidebarSurfaceColor(basePlan, designMd)).toBe('#111827');
  });

  it('returns undefined when design.md provides no usable palette and no style guide is present', () => {
    const designMd: DesignMdSpec = {
      raw: '# Spec',
      colorPalette: [{ name: 'Accent', hex: '#22C55E', role: 'CTA accent' }],
    };
    expect(extractSidebarSurfaceColor(basePlan, designMd)).toBeUndefined();
  });
});
