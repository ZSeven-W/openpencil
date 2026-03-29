import type { PenDocument } from './pen.js';
import type { ViewportState, ToolType } from './canvas.js';

// ---------------------------------------------------------------------------
// Engine Options
// ---------------------------------------------------------------------------

export interface DesignEngineOptions {
  /** URL pattern for CanvasKit WASM files. */
  canvasKitPath?: string | ((file: string) => string);
  /** Base URL for bundled font files. */
  fontBasePath?: string;
  /** Custom Google Fonts CSS endpoint. */
  googleFontsCssUrl?: string;
  /** Icon lookup function for resolving icon names to SVG path data. */
  iconLookup?: IconLookupFn;
  /** Canvas background color. Default: '#1a1a1a' */
  backgroundColor?: string;
  /** Device pixel ratio override. */
  devicePixelRatio?: number;
  /** Maximum undo/redo history states. Default: 300 */
  maxHistoryStates?: number;
}

// ---------------------------------------------------------------------------
// Engine Events
// ---------------------------------------------------------------------------

export interface DesignEngineEvents {
  /** Fired after document mutation (batch-aware: only once per batch). Payload is immutable ref. */
  'document:change': (doc: PenDocument) => void;
  'selection:change': (ids: string[]) => void;
  'viewport:change': (state: ViewportState) => void;
  'tool:change': (tool: ToolType) => void;
  'history:change': (state: { canUndo: boolean; canRedo: boolean }) => void;
  'node:hover': (id: string | null) => void;
  'page:change': (pageId: string) => void;
  /** Fired after canvas re-render (browser adapter only). */
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
  | 'react-native';

/** Structured code generation result. */
export interface CodeResult {
  files: Array<{ filename: string; content: string; language: string }>;
  /** CSS variables block if the document uses design variables. */
  variables?: string;
}

// ---------------------------------------------------------------------------
// Icon Lookup
// ---------------------------------------------------------------------------

/** Injectable icon lookup function for resolving icon names to SVG path data. */
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
