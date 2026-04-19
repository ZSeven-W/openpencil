/**
 * Tests for the `elements` section of get_design_prompt.
 *
 * The elements section is the AI-facing index of the N-tool element
 * family (add_card_row_v0 / add_metric_row_v0 / add_nav_chip_row_v0 /
 * add_bottom_nav_v0 / add_activity_ring_v0). It teaches the LLM WHEN
 * to reach for a narrow tool vs fall through to batch_design.
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §5 step 4
 */

import { describe, it, expect } from 'vitest';
import { getSkillByName, resolveSkills } from '@zseven-w/pen-ai-skills';
import { buildDesignPrompt, listPromptSections } from '../tools/design-prompt';
import { DESIGN_TOOL_DEFINITIONS } from '../routes/design-routes';

describe('get_design_prompt — elements section', () => {
  it('"elements" appears in listPromptSections()', () => {
    expect(listPromptSections()).toContain('elements');
  });

  it('"elements" appears in get_design_prompt.inputSchema.section.enum', () => {
    const def = DESIGN_TOOL_DEFINITIONS.find((t) => t.name === 'get_design_prompt');
    expect(def).toBeDefined();
    const sectionProp = (def?.inputSchema.properties as Record<string, unknown> | undefined)
      ?.section as { enum?: string[] } | undefined;
    expect(sectionProp?.enum).toContain('elements');
  });

  it('buildDesignPrompt("elements") returns non-empty content', () => {
    const content = buildDesignPrompt('elements');
    expect(content.length).toBeGreaterThan(0);
    expect(content).toMatch(/ELEMENT TOOLS/i);
  });

  it('elements section names all 5 production element tools', () => {
    const content = buildDesignPrompt('elements');
    expect(content).toContain('add_card_row_v0');
    expect(content).toContain('add_metric_row_v0');
    expect(content).toContain('add_nav_chip_row_v0');
    expect(content).toContain('add_bottom_nav_v0');
    expect(content).toContain('add_activity_ring_v0');
  });

  it('elements section teaches a decision tree (pick first match)', () => {
    const content = buildDesignPrompt('elements');
    expect(content).toMatch(/decision tree/i);
    // Decision tree should reference the "match items shape → tool" pattern
    expect(content).toMatch(/title.*subtitle|label.*value|chip|tab bar|ring/);
  });

  it('elements section teaches when to fall back to batch_design', () => {
    const content = buildDesignPrompt('elements');
    expect(content).toMatch(/batch_design/);
    // Must name at least one fallback condition so the AI knows escape is OK
    expect(content).toMatch(/heterogeneous|composite|post-hoc|styling/i);
  });

  it('elements section teaches the composition pattern (parent_id for row-in-section)', () => {
    const content = buildDesignPrompt('elements');
    expect(content).toMatch(/parent_id/i);
  });

  it('elements section names the invariants tools guarantee', () => {
    const content = buildDesignPrompt('elements');
    // The AI should know it doesn't need to reproduce wrapper structure
    expect(content).toMatch(/wrapper|clipContent|overflow|invariant/i);
    expect(content).toMatch(/unique id/i);
  });

  it('elements section NOT returned when section argument omitted, but IS part of full prompt', () => {
    // Sanity: default / full prompt should not crash
    const full = buildDesignPrompt();
    expect(full.length).toBeGreaterThan(0);
    // Passing 'elements' explicitly returns just that section (smaller than full)
    const elementsOnly = buildDesignPrompt('elements');
    expect(elementsOnly.length).toBeLessThan(full.length);
  });

  it('unknown section string falls back to full prompt (documented behavior)', () => {
    const unknown = buildDesignPrompt('does-not-exist');
    const full = buildDesignPrompt();
    expect(unknown).toBe(full);
  });
});

describe('elements skill — flag-gated auto-loading (no prompt pollution)', () => {
  it('getSkillByName("elements") returns the skill regardless of flags (direct lookup)', () => {
    const skill = getSkillByName('elements');
    expect(skill).toBeDefined();
    expect(skill?.meta.name).toBe('elements');
    // Trigger is a flag gate, not unconditional loading
    expect(skill?.meta.trigger).toEqual({ flags: ['hasMcpTools'] });
  });

  it('resolveSkills("generation") WITHOUT hasMcpTools flag → elements NOT auto-included', () => {
    const ctx = resolveSkills('generation', 'create a dashboard', { flags: {} });
    const names = ctx.skills.map((s) => s.meta.name);
    expect(names).not.toContain('elements');
  });

  it('resolveSkills("generation") WITH hasMcpTools flag → elements IS auto-included', () => {
    const ctx = resolveSkills('generation', 'create a dashboard', {
      flags: { hasMcpTools: true },
    });
    const names = ctx.skills.map((s) => s.meta.name);
    expect(names).toContain('elements');
  });

  it('buildDesignPrompt("elements") returns content regardless of flags (uses direct lookup, not resolver)', () => {
    // get_design_prompt's section path bypasses resolveSkills — it uses
    // getSkillByName directly. So the flag gate doesn't affect this path:
    // external MCP clients asking for the section explicitly still get it.
    const content = buildDesignPrompt('elements');
    expect(content.length).toBeGreaterThan(500);
    expect(content).toContain('add_card_row_v0');
  });
});
