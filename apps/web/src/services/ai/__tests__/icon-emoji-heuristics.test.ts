import { describe, it, expect, vi } from 'vitest';

vi.mock('@/canvas/canvas-text-measure', () => ({
  estimateLineWidth: () => 0,
  estimateTextHeight: () => 0,
  defaultLineHeight: () => 1.2,
  hasCjkText: () => false,
}));

import type { PenNode } from '@zseven-w/pen-types';
import { applyNoEmojiIconHeuristic } from '../icon-emoji-heuristics';

/**
 * Lock-in regression for the 2026-04-22 finding (element-tool-round-
 * trip.test.ts surfaced this behavior indirectly):
 *
 * `applyNoEmojiIconHeuristic` is called by `applyGenerationHeuristics`
 * inside `insertStreamingNode`. For any `text` node whose content
 * contains an emoji (per EMOJI_REGEX), it:
 *   1. Strips the emoji characters
 *   2. Collapses any 2+ whitespace runs to a single space
 *   3. Trims leading/trailing whitespace
 *
 * This is INTENTIONAL — the AI is supposed to emit `icon_font` for
 * icons, not bake emoji into text content. This test pins the
 * behavior so future refactors don't silently regress it (either
 * direction: suddenly preserving emoji OR newly mangling CJK / other
 * unicode).
 */

function makeText(content: string): PenNode {
  return {
    id: 't1',
    type: 'text',
    name: 'Text',
    content,
    fontSize: 14,
  } as unknown as PenNode;
}

describe('applyNoEmojiIconHeuristic', () => {
  describe('emoji scrubbing', () => {
    it('strips simple emoji + collapses resulting double spaces', () => {
      const node = makeText('Launch 🚀 ready');
      applyNoEmojiIconHeuristic(node);
      expect((node as { content: string }).content).toBe('Launch ready');
    });

    it('strips multiple emojis in one string', () => {
      const node = makeText('Start 🚀 middle 🎉 end 🎊');
      applyNoEmojiIconHeuristic(node);
      expect((node as { content: string }).content).toBe('Start middle end');
    });

    it('leading emoji → trimmed leading space', () => {
      const node = makeText('🚀 Launch');
      applyNoEmojiIconHeuristic(node);
      expect((node as { content: string }).content).toBe('Launch');
    });

    it('trailing emoji → trimmed trailing space', () => {
      const node = makeText('Ship it 🎉');
      applyNoEmojiIconHeuristic(node);
      expect((node as { content: string }).content).toBe('Ship it');
    });
  });

  describe('non-emoji preservation (what we must NOT mangle)', () => {
    it('plain ASCII untouched', () => {
      const node = makeText('Hello, World!');
      applyNoEmojiIconHeuristic(node);
      expect((node as { content: string }).content).toBe('Hello, World!');
    });

    it('CJK (Chinese) untouched', () => {
      const node = makeText('欢迎使用 OpenPencil');
      applyNoEmojiIconHeuristic(node);
      expect((node as { content: string }).content).toBe('欢迎使用 OpenPencil');
    });

    it('CJK (Japanese with hiragana) untouched', () => {
      const node = makeText('こんにちは世界');
      applyNoEmojiIconHeuristic(node);
      expect((node as { content: string }).content).toBe('こんにちは世界');
    });

    it('CJK (Korean with hangul) untouched', () => {
      const node = makeText('안녕하세요 세계');
      applyNoEmojiIconHeuristic(node);
      expect((node as { content: string }).content).toBe('안녕하세요 세계');
    });

    it('Arabic (RTL) untouched', () => {
      const node = makeText('مرحبا بالعالم');
      applyNoEmojiIconHeuristic(node);
      expect((node as { content: string }).content).toBe('مرحبا بالعالم');
    });

    it('unicode punctuation (em-dash, ellipsis, curly quotes) untouched', () => {
      const node = makeText('Dash—em-dash, ellipsis… and "curly" quotes');
      applyNoEmojiIconHeuristic(node);
      expect((node as { content: string }).content).toBe(
        'Dash—em-dash, ellipsis… and "curly" quotes',
      );
    });

    it('arrow characters (→, ←, ↑, ↓) untouched', () => {
      const node = makeText('Next → or ← Back; ↑/↓ to scroll');
      applyNoEmojiIconHeuristic(node);
      expect((node as { content: string }).content).toBe('Next → or ← Back; ↑/↓ to scroll');
    });
  });

  describe('non-text nodes are skipped', () => {
    it('frame node is untouched', () => {
      const frame = {
        id: 'f1',
        type: 'frame',
        name: 'Frame',
        width: 100,
        height: 100,
        children: [],
      } as unknown as PenNode;
      const before = JSON.stringify(frame);
      applyNoEmojiIconHeuristic(frame);
      expect(JSON.stringify(frame)).toBe(before);
    });

    it('icon_font node is untouched even if name contains emoji-like chars', () => {
      const icon = {
        id: 'i1',
        type: 'icon_font',
        name: 'Icon',
        iconFontName: 'rocket',
        iconFontFamily: 'lucide',
      } as unknown as PenNode;
      const before = JSON.stringify(icon);
      applyNoEmojiIconHeuristic(icon);
      expect(JSON.stringify(icon)).toBe(before);
    });
  });

  describe('text node without content', () => {
    it('missing content is a no-op', () => {
      const node = { id: 't', type: 'text', name: 'Empty' } as unknown as PenNode;
      applyNoEmojiIconHeuristic(node);
      // content still missing → no crash, nothing added
      expect((node as { content?: string }).content).toBeUndefined();
    });

    it('empty string content is a no-op', () => {
      const node = makeText('');
      applyNoEmojiIconHeuristic(node);
      expect((node as { content: string }).content).toBe('');
    });
  });

  describe('emoji-only content falls through to the icon-replacement branch', () => {
    it('emoji-only text whose content would be empty after scrub → replaced with fallback path', () => {
      const node = makeText('🚀');
      applyNoEmojiIconHeuristic(node);
      // When scrub empties the content, the heuristic converts the
      // text node into a `path` node with a fallback geometry.
      expect(node.type).toBe('path');
    });
  });
});
