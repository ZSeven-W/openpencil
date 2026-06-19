import { vi } from 'vitest';
// In-memory stand-in for the wasm Viewer so unit tests run in jsdom.
export class MockViewer {
  private docJson = '{}';
  private pagesJson = '[]';
  private vp = { pan_x: 0, pan_y: 0, zoom: 1 };
  free = vi.fn();
  load_str = vi.fn((src: string) => { this.docJson = src; this.pagesJson = '[{"id":"p","name":"P","children":[]}]'; });
  page_count = vi.fn(() => 1);
  active_page_index = vi.fn(() => 0);
  attach_canvas = vi.fn(async () => {});
  detach = vi.fn();
  mark_dirty = vi.fn();
  push_scene = vi.fn();
  set_viewport = vi.fn((x: number, y: number, z: number) => { this.vp = { pan_x: x, pan_y: y, zoom: z }; });
  zoom_to_fit = vi.fn(() => { this.vp = { pan_x: 0, pan_y: 0, zoom: 2 }; });
  forward_wheel = vi.fn();
  document_json = vi.fn(() => this.docJson);
  pages_json = vi.fn(() => this.pagesJson);
  viewport_json = vi.fn(() => JSON.stringify(this.vp));
}
export const mockExport = vi.fn((_v: unknown, format: string) => {
  // Use a typed-array literal instead of TextEncoder to avoid jsdom cross-realm
  // Uint8Array instanceof failures (TextEncoder in jsdom returns a Node-realm Uint8Array).
  if (format === 'svg') return new Uint8Array([60, 115, 118, 103, 47, 62]); // '<svg/>'
  throw new Error('not available');
});
