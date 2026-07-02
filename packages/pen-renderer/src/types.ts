import type { PenNode } from '@zseven-w/pen-types';

export type { ViewportState } from '@zseven-w/pen-types';

export interface ClipInfo {
  x: number;
  y: number;
  w: number;
  h: number;
  rx: number;
}

export interface RenderNode {
  node: PenNode;
  absX: number;
  absY: number;
  absW: number;
  absH: number;
  /**
   * Ancestor-to-self stack of clip rectangles to apply when painting this
   * node. Stored as an array (not a single intersected ClipInfo) so each
   * level's rounded corner can be enforced independently — a single rrect
   * can't encode `(rrect ∩ rrect)` faithfully when one rect cuts inside
   * the other's corner. The renderer pushes a canvas clip per entry and
   * pops them in reverse before drawing the next node.
   *
   * Outer-most ancestor first; the entry derived from the immediate
   * `clipContent: true` parent (or the root frame artboard) is last.
   * Empty array (or undefined) means the node paints unclipped.
   */
  clipStack?: ClipInfo[];
}

/** Injectable icon lookup function for resolving icon names to SVG path data. */
export interface IconLookupFn {
  (name: string): { d: string; iconId: string; style: 'stroke' | 'fill' } | null;
}

export interface PenRendererOptions {
  /** URL pattern for CanvasKit WASM files. Default: '/canvaskit/' */
  canvasKitPath?: string | ((file: string) => string);
  /** Base URL for bundled font files. Default: '/fonts/' */
  fontBasePath?: string;
  /** Custom Google Fonts CSS endpoint. Default: 'https://fonts.googleapis.com/css2' */
  googleFontsCssUrl?: string;
  /** Icon lookup function. Default: null (icons render as fallback circle) */
  iconLookup?: IconLookupFn;
  /** Theme variant to use for variable resolution. Default: first variant per axis */
  themeVariant?: Record<string, string>;
  /** Background color. Default: '#1a1a1a' */
  backgroundColor?: string;
  /** Device pixel ratio override. Default: window.devicePixelRatio */
  devicePixelRatio?: number;
  /** Default fonts to preload. Default: ['Inter', 'Noto Sans SC'] */
  defaultFonts?: string[];
}
