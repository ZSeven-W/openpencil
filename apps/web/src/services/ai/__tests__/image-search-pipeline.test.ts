import { describe, it, expect, beforeEach } from 'vitest';
import {
  inferAspectRatio,
  isImagePlaceholderFrame,
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
});
