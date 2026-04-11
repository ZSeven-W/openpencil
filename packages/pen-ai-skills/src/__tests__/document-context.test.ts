import { describe, it, expect } from 'vitest';
import {
  createDesignContext,
  extractDesignContext,
  mergePreference,
} from '../memory/document-context';

describe('createDesignContext', () => {
  it('creates context with null documentPath for unsaved docs', () => {
    const ctx = createDesignContext(null);
    expect(ctx.documentPath).toBeNull();
    expect(ctx.createdAt).toBeTruthy();
    expect(ctx.designSystem).toEqual({});
    expect(ctx.structure).toEqual({});
    expect(ctx.preferences).toEqual({});
  });

  it('creates context with file path', () => {
    const ctx = createDesignContext('/path/to/design.op');
    expect(ctx.documentPath).toBe('/path/to/design.op');
  });
});

describe('extractDesignContext', () => {
  it('extracts palette and aesthetic from orchestrator plan', () => {
    const plan = {
      styleGuide: { palette: ['#000', '#FFF'], aesthetic: 'minimal' },
      subtasks: [{ label: 'Hero Section' }, { label: 'Features' }],
    };
    const ctx = createDesignContext('/test.op');
    const updated = extractDesignContext(ctx, plan as any);
    expect(updated.designSystem.palette).toEqual(['#000', '#FFF']);
    expect(updated.designSystem.aesthetic).toBe('minimal');
    expect(updated.structure.sections).toEqual(['Hero Section', 'Features']);
  });
});

describe('mergePreference', () => {
  it('adds a new override', () => {
    const ctx = createDesignContext('/test.op');
    const updated = mergePreference(ctx, {
      what: 'corner',
      from: 'rounded-lg',
      to: 'rounded-full',
    });
    expect(updated.preferences.overrides).toHaveLength(1);
    expect(updated.preferences.overrides![0].what).toBe('corner');
  });

  it('replaces existing override for same "what"', () => {
    let ctx = createDesignContext('/test.op');
    ctx = mergePreference(ctx, { what: 'corner', from: 'rounded-lg', to: 'rounded-full' });
    ctx = mergePreference(ctx, { what: 'corner', from: 'rounded-full', to: 'rounded-none' });
    expect(ctx.preferences.overrides).toHaveLength(1);
    expect(ctx.preferences.overrides![0].to).toBe('rounded-none');
  });
});
