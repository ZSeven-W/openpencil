// Public types for the OpenPencil read-only web SDK.
export interface Viewport {
  panX: number;
  panY: number;
  zoom: number;
}

export interface CreateViewerOptions {
  canvas: HTMLCanvasElement;
  doc?: string | Uint8Array;
  wasmUrl?: string;
}

// Re-export generated document types from the vendored ops schema.
export type { PenDocument, PenPage } from './ops-types.js';
