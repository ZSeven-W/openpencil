import type { PenDocument } from './pen.js';
import type { ViewportState, ToolType } from './canvas.js';

// ---------------------------------------------------------------------------
// Engine Options
// ---------------------------------------------------------------------------

export interface DesignEngineOptions {
  /** CanvasKit WASM 文件的 URL 模式。 */
  canvasKitPath?: string | ((file: string) => string);
  /** Base URL 用于捆绑字体文件。 */
  fontBasePath?: string;
  /** Custom Google Fonts CSS 端点。 */
  googleFontsCssUrl?: string;
  /** Icon 查找函数，用于将图标名称解析为 SVG 路径数据。 */
  iconLookup?: IconLookupFn;
  /** Canvas 背景颜色。 Default: '#1a1a1a' */
  backgroundColor?: string;
  /** Device 像素比率覆盖。 */
  devicePixelRatio?: number;
  /** Maximum undo/redo 历史状态。 Default: 300 */
  maxHistoryStates?: number;
}

// ---------------------------------------------------------------------------
// Engine Events
// ---------------------------------------------------------------------------

export interface DesignEngineEvents {
  /** 文档突变后的 Fired （批次感知：每批次仅一次）。 Payload 是不可变的引用。 */
  'document:change': (doc: PenDocument) => void;
  'selection:change': (ids: string[]) => void;
  'viewport:change': (state: ViewportState) => void;
  'tool:change': (tool: ToolType) => void;
  'history:change': (state: { canUndo: boolean; canRedo: boolean }) => void;
  'node:hover': (id: string | null) => void;
  'page:change': (pageId: string) => void;
  /** Fired 画布重新渲染后（仅限浏览器适配器）。 */
  render: () => void;
}

// ---------------------------------------------------------------------------
// Code Generation
// ---------------------------------------------------------------------------

export type CodePlatform =
  | 'react'
  | 'html'
  | 'css'
  | 'vue'
  | 'svelte'
  | 'flutter'
  | 'swiftui'
  | 'compose'
  | 'react-native'
  | 'uniapp';

/** Structured 代码生成结果。 */
export interface CodeResult {
  files: Array<{ filename: string; content: string; language: string }>;
  /** 如果文档使用设计变量，则 CSS 变量块。 */
  variables?: string;
}

// ---------------------------------------------------------------------------
// Icon Lookup
// ---------------------------------------------------------------------------

/** Injectable 图标查找功能，用于将图标名称解析为 SVG 路径数据。 */
export interface IconLookupFn {
  (name: string): { d: string; iconId: string; style: 'stroke' | 'fill' } | null;
}

// ---------------------------------------------------------------------------
// Canvas Interaction Types
// ---------------------------------------------------------------------------

export interface TextEditState {
  nodeId: string;
  x: number;
  y: number;
  w: number;
  h: number;
  content: string;
  fontSize: number;
  fontFamily: string;
  fontWeight: number;
  textAlign: string;
  color: string;
  lineHeight: number;
}

export interface AgentIndicatorEntry {
  nodeId: string;
  color: string;
  name: string;
}

export interface AgentFrameEntry {
  frameId: string;
  color: string;
  name: string;
}

export interface InsertionIndicator {
  x: number;
  y: number;
  length: number;
  orientation: 'horizontal' | 'vertical';
}

export interface ContainerHighlight {
  x: number;
  y: number;
  w: number;
  h: number;
}
