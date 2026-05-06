import type { PenNode } from '@zseven-w/pen-types';

export type { ViewportState } from '@zseven-w/pen-types';

export interface RenderNode {
  node: PenNode;
  absX: number;
  absY: number;
  absW: number;
  absH: number;
  clipRect?: { x: number; y: number; w: number; h: number; rx: number };
}

/** Injectable 图标查找功能，用于将图标名称解析为 SVG 路径数据。 */
export interface IconLookupFn {
  (name: string): { d: string; iconId: string; style: 'stroke' | 'fill' } | null;
}

export interface PenRendererOptions {
  /** CanvasKit WASM 文件的 URL 模式。 Default: '/canvaskit/' */
  canvasKitPath?: string | ((file: string) => string);
  /** Base URL 用于捆绑字体文件。 Default: '/字体/' */
  fontBasePath?: string;
  /** Custom Google Fonts CSS 端点。 Default: 'https://fonts.googleapis.com/css2' */
  googleFontsCssUrl?: string;
  /** Icon 查找功能。 Default：null（图标呈现为后备圆圈） */
  iconLookup?: IconLookupFn;
  /** Theme 变体用于可变分辨率。 Default：每个轴的第一个变体 */
  themeVariant?: Record<string, string>;
  /** Background 颜色。 Default: '#1a1a1a' */
  backgroundColor?: string;
  /** Device 像素比率覆盖。 Default: 窗口.devicePixelRatio */
  devicePixelRatio?: number;
  /** 要预加载的 Default 字体。 Default: ['Inter', 'Noto Sans SC'] */
  defaultFonts?: string[];
}
