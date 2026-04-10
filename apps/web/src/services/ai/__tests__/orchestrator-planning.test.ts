import { describe, expect, it } from 'vitest';
import { filterPlanningSkillsForPrompt, parseOrchestratorResponse } from '../orchestrator-planning';

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
});
