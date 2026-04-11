// apps/web/src/components/panels/git-panel/__tests__/format-commit-message.test.ts
import { describe, it, expect } from 'vitest';
import { parseAutosaveMessage } from '@/components/panels/git-panel/format-commit-message';

describe('parseAutosaveMessage', () => {
  it('parses the simple auto: HH:MM format', () => {
    expect(parseAutosaveMessage('auto: 15:42')).toEqual({
      time: '15:42',
      summary: null,
    });
  });

  it('parses the extended format with diff suffix', () => {
    expect(parseAutosaveMessage('auto: 15:42 — 3 frames, 7 nodes modified')).toEqual({
      time: '15:42',
      summary: '3 frames, 7 nodes modified',
    });
  });

  it('returns null for non-autosave messages', () => {
    expect(parseAutosaveMessage('initial commit')).toBeNull();
    expect(parseAutosaveMessage('tweak login button color')).toBeNull();
    expect(parseAutosaveMessage('')).toBeNull();
  });

  it('returns null for malformed auto: messages', () => {
    expect(parseAutosaveMessage('auto:')).toBeNull();
    expect(parseAutosaveMessage('auto: 99')).toBeNull();
    expect(parseAutosaveMessage('auto: abc')).toBeNull();
  });
});
