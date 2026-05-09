import { describe, it, expect, beforeEach } from 'vitest';
import {
  inferAspectRatio,
  isImagePlaceholderFrame,
  isUnfilledImagePlaceholderFrame,
  isImageAreaFrameByHeuristic,
  isFramePlaceholderStillUnfilled,
  collectImageSearchTargets,
  extractQueryForNode,
  findParentSemanticName,
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

  it('rejects single-NON-icon-child frame (CTA / text / nested layout)', () => {
    // Codex stop-time review 2026-05-10: a hero frame named "Hero" with
    // exactly one CTA button child used to match the heuristic. The
    // queue's children:[] erase step would then destroy the button when
    // the photo landed. Now: must be 0 children or 1 icon_font child;
    // any other shape is rejected so legitimate hero / banner content
    // is preserved.
    const heroWithButton = {
      id: 'hero',
      type: 'frame',
      name: 'Hero',
      width: 600,
      height: 320,
      fill: [{ type: 'solid', color: '#0EA5E9' }],
      children: [
        {
          id: 'cta',
          type: 'frame',
          role: 'button',
          children: [{ id: 'cta-text', type: 'text', content: 'Get Started' }],
        },
      ],
    } as unknown as PenNode;
    expect(isImageAreaFrameByHeuristic(heroWithButton)).toBe(false);

    const bannerWithText = {
      id: 'banner',
      type: 'frame',
      name: 'Banner',
      width: 800,
      height: 200,
      fill: [{ type: 'solid', color: '#FF6B35' }],
      children: [{ id: 't', type: 'text', content: 'Sale!' }],
    } as unknown as PenNode;
    expect(isImageAreaFrameByHeuristic(bannerWithText)).toBe(false);

    const coverWithFrame = {
      id: 'cover',
      type: 'frame',
      name: 'Cover',
      width: 400,
      height: 200,
      fill: [{ type: 'solid', color: '#10B981' }],
      children: [{ id: 'inner', type: 'frame', children: [] }],
    } as unknown as PenNode;
    expect(isImageAreaFrameByHeuristic(coverWithFrame)).toBe(false);
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

describe('extractQueryForNode + findParentSemanticName', () => {
  beforeEach(() => {
    useDocumentStore.setState({
      document: { children: [], variables: [], themes: [], pages: [] },
      isDirty: false,
    } as never);
  });

  function loadTree(rootChildren: PenNode[]) {
    useDocumentStore.setState({
      document: {
        children: [],
        variables: [],
        themes: [],
        pages: [
          {
            id: 'page-1',
            name: 'Page 1',
            children: [
              { id: 'root', type: 'frame', name: 'Page', children: rootChildren } as PenNode,
            ],
          },
        ],
      },
      isDirty: false,
    } as never);
  }

  it('extractQueryForNode prefers explicit imageSearchQuery over name', () => {
    const node = {
      id: 'i',
      type: 'frame',
      name: 'Image',
      imageSearchQuery: 'sushi platter',
    } as PenNode;
    expect(extractQueryForNode(node)).toBe('sushi platter');
  });

  it('extractQueryForNode skips generic placeholder names and walks to parent', () => {
    loadTree([
      {
        id: 'card',
        type: 'frame',
        name: 'Bella Italia',
        children: [
          {
            id: 'image-area',
            type: 'frame',
            name: 'Image',
            width: 200,
            height: 140,
            fill: [{ type: 'solid', color: '#FCD34D' }],
          } as PenNode,
        ],
      } as PenNode,
    ]);
    const node = useDocumentStore.getState().getNodeById('image-area')!;
    expect(extractQueryForNode(node)).toBe('Bella Italia');
  });

  it('findParentSemanticName skips layout words (Card / Wrapper / Container)', () => {
    loadTree([
      {
        id: 'wrap',
        type: 'frame',
        name: 'Card Wrapper',
        children: [
          {
            id: 'inner-card',
            type: 'frame',
            name: 'Margherita Pizza',
            children: [
              {
                id: 'image-area',
                type: 'frame',
                name: 'Photo',
                width: 200,
                height: 140,
              } as PenNode,
            ],
          } as PenNode,
        ],
      } as PenNode,
    ]);
    // Should skip "Card Wrapper" (matches the layout-word filter) and
    // accept "Margherita Pizza" (semantic).
    expect(findParentSemanticName('image-area')).toBe('Margherita Pizza');
  });

  it('findParentSemanticName returns null when no semantic parent within hops', () => {
    loadTree([
      {
        id: 'wrap1',
        type: 'frame',
        name: 'Wrapper',
        children: [
          {
            id: 'wrap2',
            type: 'frame',
            name: 'Container',
            children: [
              {
                id: 'wrap3',
                type: 'frame',
                name: 'Section',
                children: [
                  {
                    id: 'wrap4',
                    type: 'frame',
                    name: 'Frame',
                    children: [
                      {
                        id: 'image-area',
                        type: 'frame',
                        name: 'Image',
                      } as PenNode,
                    ],
                  } as PenNode,
                ],
              } as PenNode,
            ],
          } as PenNode,
        ],
      } as PenNode,
    ]);
    // Default maxHops=3; all 3 nearest parents are layout words.
    expect(findParentSemanticName('image-area')).toBeNull();
  });

  it('extractQueryForNode falls back to name when parent walk yields nothing', () => {
    const node = {
      id: 'orphan',
      type: 'frame',
      name: 'My Custom Photo',
    } as PenNode;
    // Not loaded into store — parent map is empty, but the name itself
    // is non-generic ("My Custom Photo" — has "Photo" but it's not the
    // bare generic literal in the GENERIC_PLACEHOLDER_NAMES set).
    expect(extractQueryForNode(node)).toBe('My Custom Photo');
  });
});

describe('isFramePlaceholderStillUnfilled (queue re-check predicate)', () => {
  // Codex review #N (2026-05-10): the queue processor's still-needs-fill
  // gate previously called isUnfilledImagePlaceholderFrame which rejects
  // anything without role='image-placeholder'. Heuristic frames passed
  // collectImageSearchTargets + enqueue but then got dropped here, so the
  // food-app card photos still never landed. This helper combines both
  // canonical and heuristic branches so the gate stays consistent with
  // the enqueue path.

  it('accepts canonical unfilled placeholder frame', () => {
    const node = {
      id: 'p',
      type: 'frame',
      role: 'image-placeholder',
      width: 200,
      height: 140,
      fill: [{ type: 'solid', color: '#F1F5F9' }],
    } as unknown as PenNode;
    expect(isFramePlaceholderStillUnfilled(node)).toBe(true);
  });

  it('accepts heuristic-matched frame named "Image" with solid fill', () => {
    const node = {
      id: 'h',
      type: 'frame',
      name: 'Image',
      width: 200,
      height: 140,
      fill: [{ type: 'solid', color: '#FCD34D' }],
    } as unknown as PenNode;
    expect(isFramePlaceholderStillUnfilled(node)).toBe(true);
  });

  it('rejects already-filled placeholder (image fill present)', () => {
    const node = {
      id: 'p',
      type: 'frame',
      role: 'image-placeholder',
      width: 200,
      height: 140,
      fill: [{ type: 'image', url: 'http://x.png' }],
    } as unknown as PenNode;
    expect(isFramePlaceholderStillUnfilled(node)).toBe(false);
  });

  it('rejects already-filled heuristic frame (image fill present)', () => {
    const node = {
      id: 'h',
      type: 'frame',
      name: 'Image',
      width: 200,
      height: 140,
      fill: [{ type: 'image', url: 'http://x.png' }],
    } as unknown as PenNode;
    expect(isFramePlaceholderStillUnfilled(node)).toBe(false);
  });

  it('rejects null / undefined', () => {
    expect(isFramePlaceholderStillUnfilled(null)).toBe(false);
    expect(isFramePlaceholderStillUnfilled(undefined)).toBe(false);
  });

  it('rejects unrelated frame (no role + name not in IMAGE_AREA_NAME_RE)', () => {
    const node = {
      id: 'x',
      type: 'frame',
      name: 'Card Wrapper',
      width: 200,
      height: 140,
      fill: [{ type: 'solid', color: '#FFF' }],
    } as unknown as PenNode;
    expect(isFramePlaceholderStillUnfilled(node)).toBe(false);
  });
});
