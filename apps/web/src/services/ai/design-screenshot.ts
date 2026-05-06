/**
 * Screenshot
 *
 * 用于设计验证的捕获实用程序。 Backed by SkiaEngine.captureRegion()
 * 在实时画布上执行
 * CanvasKit readPixels。 Only 可从 Web 端使用（不能从 pen-mcp 中使用 — 请参阅 Phase 2
 了解基于 RPC 的外部 API）。
 */

import { getSkiaEngineRef } from '@/canvas/skia-engine-ref';
import { useDocumentStore } from '@/stores/document-store';
import type { PenNode } from '@/types/pen';

function uint8ToBase64(bytes: Uint8Array): string {
  let binary = '';
  const len = bytes.length;
  for (let i = 0; i < len; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

function computeBounds(node: PenNode): { x: number; y: number; w: number; h: number } {
  const n = node as unknown as { x?: number; y?: number; width?: number; height?: number };
  return {
    x: n.x ?? 0,
    y: n.y ?? 0,
    w: n.width ?? 100,
    h: n.height ?? 100,
  };
}

/**
 * Capture
 * 特定节点的屏幕截图。 Returns 一个 base64 PNG 数据 URL，如果画布未准备好或节点不存在，则为 null。
 */
export async function captureNodeScreenshot(nodeId: string): Promise<string | null> {
  const engine = getSkiaEngineRef();
  if (!engine) return null;

  const node = useDocumentStore.getState().getNodeById(nodeId);
  if (!node) return null;

  const bounds = computeBounds(node);
  const png = await engine.captureRegion(bounds);
  if (!png) return null;

  return `data:image/png;base64,${uint8ToBase64(png)}`;
}

/**
 * Capture a screenshot of the entire document root frame.
 * Returns a base64 PNG data URL, or null if canvas isn't ready.
 */
export async function captureRootFrameScreenshot(): Promise<string | null> {
  const engine = getSkiaEngineRef();
  if (!engine) return null;
  const png = await engine.captureRegion('root');
  if (!png) return null;
  return `data:image/png;base64,${uint8ToBase64(png)}`;
}
