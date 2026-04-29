import { describe, expect, it } from 'vitest';
import {
  buildSubAgentStyleGuideInstruction,
  compactSubAgentSkills,
} from '../orchestrator-sub-agent-compact';

describe('compactSubAgentSkills', () => {
  const skills = [
    { meta: { name: 'schema' }, content: 'schema' },
    { meta: { name: 'jsonl-format' }, content: 'jsonl' },
    { meta: { name: 'jsonl-format-simplified' }, content: 'simple' },
    { meta: { name: 'layout' }, content: 'layout' },
    { meta: { name: 'overflow' }, content: 'overflow' },
    { meta: { name: 'text-rules' }, content: 'text' },
    { meta: { name: 'mobile-app' }, content: 'mobile' },
    { meta: { name: 'landing-page' }, content: 'landing' },
    { meta: { name: 'copywriting' }, content: 'copy' },
    { meta: { name: 'anti-slop' }, content: 'anti-slop' },
    { meta: { name: 'design-system' }, content: 'tokens' },
  ];

  it('keeps a compact mobile-safe stack for basic models', () => {
    const out = compactSubAgentSkills(skills, 'basic', true, true);
    const names = out.map((skill) => skill.meta.name);

    expect(names).toContain('jsonl-format-simplified');
    expect(names).not.toContain('jsonl-format');
    expect(names).not.toContain('landing-page');
    expect(names).not.toContain('copywriting');
    expect(names).not.toContain('anti-slop');
    expect(names).not.toContain('design-system');
  });

  it('uses an even smaller retry stack for reduced-complexity basic retries', () => {
    const out = compactSubAgentSkills(skills, 'basic', true, true, true);
    const names = out.map((skill) => skill.meta.name);

    expect(names).toContain('schema');
    expect(names).toContain('jsonl-format-simplified');
    expect(names).toContain('layout');
    expect(names).toContain('mobile-app');
    expect(names).not.toContain('overflow');
    expect(names).not.toContain('icon-catalog');
  });
});

describe('buildSubAgentStyleGuideInstruction', () => {
  const styleGuide = `---
name: dark-bold-mobile
tags: [dark-mode, fitness, rounded, vibrant]
platform: mobile
---

## Color System
| Token | Value |
| --- | --- |
| Page Background | #111111 |
| Card Surface | #252542 |
| Primary Accent | #22C55E |
| Primary Text | #F5F5F5 |
| Secondary Text | #A3A3A3 |

## Typography
### Font Families
| Role | Family | Usage |
| --- | --- | --- |
| Display | Space Grotesk | Headings |
| Body | Inter | UI copy |

## Corner Radius
| Token | Value |
| --- | --- |
| Card | 16px |
| Button | 12px |
`;

  it('summarizes style guide tokens for non-full tiers', () => {
    const summary = buildSubAgentStyleGuideInstruction(styleGuide, 'dark-bold-mobile', 'basic');

    expect(summary).toContain('VISUAL STYLE GUIDE SUMMARY');
    expect(summary).toContain('Background: #111111');
    expect(summary).toContain('Accent: #22C55E');
    expect(summary).not.toContain('## Color System');
  });
});

import { buildSubAgentUserPromptForTest } from '../orchestrator-sub-agent';

describe('buildSubAgentUserPrompt in append mode', () => {
  const plan = {
    rootFrame: { id: 'content-root', name: 'Page Content Root', width: 375, height: 0 },
    subtasks: [
      {
        id: 'workout-cards',
        label: 'Workout Type Cards',
        region: { width: 375, height: 200 },
        idPrefix: 'workoutCards',
        parentFrameId: 'content-root',
        existingSectionLabels: ['Greeting Section', 'Activity Rings Section'],
      },
    ],
  };

  it('injects APPEND MODE instructions and existing-section list', () => {
    const prompt = buildSubAgentUserPromptForTest({
      subtask: plan.subtasks[0] as any,
      plan: plan as any,
      compactPrompt: 'Continue the fitness app',
      fullPrompt: 'Continue the fitness app',
    });
    expect(prompt).toMatch(/APPEND MODE/);
    expect(prompt).toMatch(/Greeting Section/);
    expect(prompt).toMatch(/Activity Rings Section/);
    expect(prompt).toMatch(/do NOT.*status bar|no.{0,8}status bar/i);
    expect(prompt).toMatch(/do NOT re-emit|off-limits/i);
  });

  it('does not inject APPEND MODE when existingSectionLabels is absent', () => {
    const prompt = buildSubAgentUserPromptForTest({
      subtask: { ...(plan.subtasks[0] as any), existingSectionLabels: undefined },
      plan: plan as any,
      compactPrompt: 'new landing page',
      fullPrompt: 'new landing page',
    });
    expect(prompt).not.toMatch(/APPEND MODE/);
  });
});
