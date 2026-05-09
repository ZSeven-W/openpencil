import { describe, it, expect, beforeEach } from 'vitest';
import {
  inferAspectRatio,
  isImagePlaceholderFrame,
  isUnfilledImagePlaceholderFrame,
  isImageAreaFrameByHeuristic,
  collectImageSearchTargets,
} from '../image-search-pipeline';
import { useDocumentStore } from '@/stores/document-store';
import type { PenNode } from '@/types/pen';

function makeImageNode(w: number, h: number): PenNode {
  return { id: 'test', type: 'image', src: '', width: w, height: h } as PenNode;
}

describe('inferAspectRatio', () => {
  it('returns wide for landscape images', () => {
    expect(inferAspectRatio(makeImageNode(1200, 600))).toBe('wide');
  });

  it('returns tall for portrait images', () => {
    expect(inferAspectRatio(makeImageNode(400, 800))).toBe('tall');
  });

  it('returns square for roughly equal dimensions', () => {
    expect(inferAspectRatio(makeImageNode(500, 500))).toBe('square');
    expect(inferAspectRatio(makeImageNode(600, 500))).toBe('square');
  });

  it('returns undefined when dimensions missing', () => {
    expect(inferAspectRatio({ id: 'x', type: 'image', src: '' } as PenNode)).toBeUndefined();
  });
});

describe('isImagePlaceholderFrame', () => {
  it('returns true for frames with role: image-placeholder', () => {
    const frame = {
      id: 'p',
      type: 'frame',
      role: 'image-placeholder',
      width: 200,
      height: 140,
      children: [],
    } as PenNode;
    expect(isImagePlaceholderFrame(frame)).toBe(true);
  });

  it('returns false for image-type nodes (handled separately)', () => {
    const img = { id: 'i', type: 'image', src: '' } as PenNode;
    expect(isImagePlaceholderFrame(img)).toBe(false);
  });

  it('returns false for frames carrying a different role', () => {
    const frame = {
      id: 'c',
      type: 'frame',
      role: 'card',
      children: [],
    } as PenNode;
    expect(isImagePlaceholderFrame(frame)).toBe(false);
  });

  it('returns false for null / undefined', () => {
    expect(isImagePlaceholderFrame(undefined)).toBe(false);
    expect(isImagePlaceholderFrame(null)).toBe(false);
  });
});

describe('isUnfilledImagePlaceholderFrame', () => {
  it('returns true for placeholder frame with the default solid gray fill', () => {
    const frame = {
      id: 'p',
      type: 'frame',
      role: 'image-placeholder',
      width: 200,
      height: 140,
      fill: [{ type: 'solid', color: '#F1F5F9' }],
      children: [],
    } as PenNode;
    expect(isUnfilledImagePlaceholderFrame(frame)).toBe(true);
  });

  it('returns true when fill is missing or empty', () => {
    const noFill = {
      id: 'p',
      type: 'frame',
      role: 'image-placeholder',
      width: 200,
      height: 140,
    } as PenNode;
    const emptyFill = { ...noFill, fill: [] } as PenNode;
    expect(isUnfilledImagePlaceholderFrame(noFill)).toBe(true);
    expect(isUnfilledImagePlaceholderFrame(emptyFill)).toBe(true);
  });

  it('returns false once the placeholder has been filled with an image', () => {
    const filled = {
      id: 'p',
      type: 'frame',
      role: 'image-placeholder',
      width: 200,
      height: 140,
      fill: [{ type: 'image', url: 'https://cdn/burger.jpg', mode: 'crop' }],
      children: [],
    } as PenNode;
    expect(isUnfilledImagePlaceholderFrame(filled)).toBe(false);
  });

  it('returns false for non-placeholder frames regardless of fill', () => {
    const card = {
      id: 'c',
      type: 'frame',
      role: 'card',
      fill: [{ type: 'solid', color: '#FFFFFF' }],
      children: [],
    } as PenNode;
    expect(isUnfilledImagePlaceholderFrame(card)).toBe(false);
  });
});

describe('collectImageSearchTargets', () => {
  beforeEach(() => {
    // Fresh empty doc per test
    useDocumentStore.setState({
      document: {
        children: [],
        variables: [],
        themes: [],
        pages: [],
      },
      isDirty: false,
    } as never);
  });

  it('finds both placeholder frames and image-type nodes with empty src', () => {
    const placeholder = {
      id: 'ph1',
      type: 'frame',
      role: 'image-placeholder',
      width: 200,
      height: 140,
      children: [
        {
          id: 'icon-child',
          type: 'icon_font',
          iconFontName: 'image',
          width: 40,
          height: 40,
        },
      ],
    } as PenNode;
    const realImage = {
      id: 'img1',
      type: 'image',
      src: '',
      width: 320,
      height: 200,
      imageSearchQuery: 'burger fries',
    } as PenNode;
    const filledImage = {
      id: 'img2',
      type: 'image',
      src: 'https://cdn.example/photo.jpg',
      width: 320,
      height: 200,
    } as PenNode;
    const root = {
      id: 'root',
      type: 'frame',
      width: 360,
      height: 800,
      children: [placeholder, realImage, filledImage],
    } as PenNode;
    useDocumentStore.setState({
      document: { children: [root], variables: [], themes: [], pages: [] },
    } as never);

    const targets = collectImageSearchTargets('root');
    const ids = targets.map((t) => t.node.id).sort();
    expect(ids).toEqual(['img1', 'ph1']);
    const phEntry = targets.find((t) => t.node.id === 'ph1');
    expect(phEntry?.kind).toBe('placeholder-frame');
    const imgEntry = targets.find((t) => t.node.id === 'img1');
    expect(imgEntry?.kind).toBe('image');
  });

  it('does not descend into placeholder frame children (icon_font is not a target)', () => {
    const placeholder = {
      id: 'ph1',
      type: 'frame',
      role: 'image-placeholder',
      width: 200,
      height: 140,
      children: [
        {
          id: 'nested-image',
          type: 'image',
          src: '',
          width: 40,
          height: 40,
        },
      ],
    } as PenNode;
    const root = {
      id: 'root',
      type: 'frame',
      width: 360,
      height: 800,
      children: [placeholder],
    } as PenNode;
    useDocumentStore.setState({
      document: { children: [root], variables: [], themes: [], pages: [] },
    } as never);

    const targets = collectImageSearchTargets('root');
    expect(targets).toHaveLength(1);
    expect(targets[0].node.id).toBe('ph1');
    expect(targets[0].kind).toBe('placeholder-frame');
  });

  it('returns empty when root id missing', () => {
    expect(collectImageSearchTargets('nonexistent')).toEqual([]);
  });

  it('does NOT collect placeholder frames that already have an image fill', () => {
    // A previous scan painted this placeholder; the role stays
    // (semantic survival) but the fill is now `type:'image'`. A
    // follow-up scan must NOT re-enqueue it.
    const filledPlaceholder = {
      id: 'ph-old',
      type: 'frame',
      role: 'image-placeholder',
      width: 320,
      height: 200,
      fill: [{ type: 'image', url: 'https://cdn/saved.jpg', mode: 'crop' }],
      children: [],
    } as PenNode;
    const newPlaceholder = {
      id: 'ph-new',
      type: 'frame',
      role: 'image-placeholder',
      width: 200,
      height: 140,
      fill: [{ type: 'solid', color: '#F1F5F9' }],
      children: [],
    } as PenNode;
    const root = {
      id: 'root',
      type: 'frame',
      width: 360,
      height: 800,
      children: [filledPlaceholder, newPlaceholder],
    } as PenNode;
    useDocumentStore.setState({
      document: { children: [root], variables: [], themes: [], pages: [] },
    } as never);

    const targets = collectImageSearchTargets('root');
    expect(targets).toHaveLength(1);
    expect(targets[0].node.id).toBe('ph-new');
  });
});

describe('isImageAreaFrameByHeuristic', () => {
  // Catches the "Bella Italia card" pattern from the 2026-05-09 user
  // report — a wide colored block at the top of a restaurant card,
  // emitted as a plain frame named "Image" / "Photo" / "Cover" without
  // role: 'image-placeholder'. The heuristic supplements the strict
  // role-based check so the auto-search pipeline still fires.

  it('flags a frame named "Image" with solid fill and image-area dimensions', () => {
    const node = {
      id: 'card-image',
      type: 'frame',
      name: 'Image',
      width: 200,
      height: 140,
      fill: [{ type: 'solid', color: '#FCD34D' }],
      children: [],
    } as unknown as PenNode;
    expect(isImageAreaFrameByHeuristic(node)).toBe(true);
  });

  it('flags "Photo" / "Cover" / "Hero" / "Thumbnail" / "Banner" / "Poster" by name', () => {
    for (const name of ['Photo', 'Cover', 'Hero Image', 'Thumbnail', 'Banner', 'Poster']) {
      const node = {
        id: 'x',
        type: 'frame',
        name,
        width: 200,
        height: 140,
        fill: [{ type: 'solid', color: '#FCD34D' }],
      } as unknown as PenNode;
      expect(isImageAreaFrameByHeuristic(node)).toBe(true);
    }
  });

  it('rejects when the canonical role is already set (handled by strict path)', () => {
    const node = {
      id: 'p',
      type: 'frame',
      name: 'Image',
      role: 'image-placeholder',
      width: 200,
      height: 140,
      fill: [{ type: 'solid', color: '#F1F5F9' }],
    } as unknown as PenNode;
    expect(isImageAreaFrameByHeuristic(node)).toBe(false);
  });

  it('rejects an unrelated frame name (e.g. "Card", "Wrapper")', () => {
    for (const name of ['Card', 'Wrapper', 'Container', 'Section']) {
      const node = {
        id: 'x',
        type: 'frame',
        name,
        width: 200,
        height: 140,
        fill: [{ type: 'solid', color: '#FCD34D' }],
      } as unknown as PenNode;
      expect(isImageAreaFrameByHeuristic(node)).toBe(false);
    }
  });

  it('rejects when fill is already an image (already filled)', () => {
    const node = {
      id: 'x',
      type: 'frame',
      name: 'Image',
      width: 200,
      height: 140,
      fill: [{ type: 'image', url: 'http://x.png' }],
    } as unknown as PenNode;
    expect(isImageAreaFrameByHeuristic(node)).toBe(false);
  });

  it('rejects gradient fills (decorative, not photo placeholder)', () => {
    const node = {
      id: 'x',
      type: 'frame',
      name: 'Hero',
      width: 200,
      height: 140,
      fill: [{ type: 'linear_gradient', stops: [] }],
    } as unknown as PenNode;
    expect(isImageAreaFrameByHeuristic(node)).toBe(false);
  });

  it('rejects content-rich frame (>1 child = real layout, not placeholder)', () => {
    const node = {
      id: 'x',
      type: 'frame',
      name: 'Photo',
      width: 200,
      height: 140,
      fill: [{ type: 'solid', color: '#FCD34D' }],
      children: [
        { id: 'c1', type: 'text', content: 'a' },
        { id: 'c2', type: 'text', content: 'b' },
      ],
    } as unknown as PenNode;
    expect(isImageAreaFrameByHeuristic(node)).toBe(false);
  });

  it('accepts single-icon-child frame (broken-image hint)', () => {
    const node = {
      id: 'x',
      type: 'frame',
      name: 'Cover',
      width: 200,
      height: 140,
      fill: [{ type: 'solid', color: '#FCD34D' }],
      children: [{ id: 'icon', type: 'icon_font', iconFontName: 'image' }],
    } as unknown as PenNode;
    expect(isImageAreaFrameByHeuristic(node)).toBe(true);
  });

  it('rejects undersize frames (60 < height threshold OR 80 < width)', () => {
    const tiny = {
      id: 'x',
      type: 'frame',
      name: 'Photo',
      width: 50,
      height: 30,
      fill: [{ type: 'solid', color: '#FCD34D' }],
    } as unknown as PenNode;
    expect(isImageAreaFrameByHeuristic(tiny)).toBe(false);
  });

  it('rejects when width or height is non-numeric (fill_container etc.)', () => {
    const fill = {
      id: 'x',
      type: 'frame',
      name: 'Photo',
      width: 'fill_container',
      height: 'fit_content',
      fill: [{ type: 'solid', color: '#FCD34D' }],
    } as unknown as PenNode;
    expect(isImageAreaFrameByHeuristic(fill)).toBe(false);
  });
});
