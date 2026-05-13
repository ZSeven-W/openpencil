import { useEffect } from 'react';
import { useCanvasStore } from '@/stores/canvas-store';
import { useDocumentStore } from '@/stores/document-store';
import { useHistoryStore } from '@/stores/history-store';
import { getCanvasSize } from '@/canvas/skia-engine-ref';
import {
  isFigmaClipboardHtml,
  extractFigmaClipboardData,
  figmaClipboardToNodes,
} from '@/services/figma/figma-clipboard';
import type { PenNode } from '@/types/pen';

/**
 * Compute 一组 PenNodes 的边界框。
 */
function computeBounds(nodes: PenNode[]): {
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
} {
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;

  for (const node of nodes) {
    const x = node.x ?? 0;
    const y = node.y ?? 0;

    let right: number;
    let bottom: number;
    if (node.type === 'line') {
      right = Math.max(x, node.x2 ?? x);
      bottom = Math.max(y, node.y2 ?? y);
    } else {
      const w = 'width' in node && typeof node.width === 'number' ? node.width : 100;
      const h = 'height' in node && typeof node.height === 'number' ? node.height : 100;
      right = x + w;
      bottom = y + h;
    }

    minX = Math.min(minX, x);
    minY = Math.min(minY, y);
    maxX = Math.max(maxX, right);
    maxY = Math.max(maxY, bottom);
  }

  return { minX, minY, maxX, maxY };
}

/**
 * Get 使用 Skia 画布视口在场景坐标中的视口中心。
 */
function getViewportCenter(): { cx: number; cy: number } {
  const { viewport } = useCanvasStore.getState();
  const { width, height } = getCanvasSize();
  const cx = (-viewport.panX + width / 2) / viewport.zoom;
  const cy = (-viewport.panY + height / 2) / viewport.zoom;
  return { cx, cy };
}

/**
 * Process
 * Figma HTML 剪贴板数据 — 提取、解码并添加到画布。如果粘贴了 Figma 节点，则 Returns true。
 */
function processFigmaHtml(html: string): boolean {
  const clipData = extractFigmaClipboardData(html);
  if (!clipData) return false;

  const { nodes } = figmaClipboardToNodes(clipData.buffer, html);
  if (nodes.length === 0) return false;

  // Center 在视口中心粘贴节点
  const bounds = computeBounds(nodes);
  const { cx, cy } = getViewportCenter();
  const offsetX = cx - (bounds.minX + bounds.maxX) / 2;
  const offsetY = cy - (bounds.minY + bounds.maxY) / 2;

  for (const node of nodes) {
    node.x = (node.x ?? 0) + offsetX;
    node.y = (node.y ?? 0) + offsetY;
  }

  // Batch 所有插入到单个撤消步骤中
  const doc = useDocumentStore.getState().document;
  useHistoryStore.getState().startBatch(doc);

  const newIds: string[] = [];
  for (const node of nodes) {
    useDocumentStore.getState().addNode(null, node);
    newIds.push(node.id);
  }

  useHistoryStore.getState().endBatch(useDocumentStore.getState().document);

  // Select 粘贴的节点
  useCanvasStore.getState().setSelection(newIds, null);
  return true;
}

/**
 * Try 通过 Clipb
 * oard API 从系统剪贴板读取 Figma 数据。 Used 作为 `paste` 事件可能不会触发时的后备（例如，当像
 * <canvas> 这样的不可编辑元素具有焦点时）。
 */
export async function tryPasteFigmaFromClipboard(): Promise<boolean> {
  try {
    if (navigator.clipboard?.read) {
      const items = await navigator.clipboard.read();
      for (const item of items) {
        if (item.types.includes('text/html')) {
          const blob = await item.getType('text/html');
          const html = await blob.text();
          if (isFigmaClipboardHtml(html)) {
            return processFigmaHtml(html);
          }
        }
      }
    }
  } catch {
    // Clipboard API 可能不可用或权限被拒绝
  }
  return false;
}

/**
 * Listens
 * 用于浏览器 `paste` 事件来检测 Figma 剪贴板数据。 Also 提供
 * `tryPasteFigmaFromClipboard()` 供 keydown 处理程序使用，作为粘贴事件可能未触发时的后备。
 */
export function useFigmaPaste() {
  useEffect(() => {
    const handlePaste = (e: ClipboardEvent) => {
      // Skip 如果用户输入 input/textarea/contentEditable
      const target = e.target as HTMLElement;
      if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) {
        return;
      }

      const html = e.clipboardData?.getData('text/html');
      if (!html || !isFigmaClipboardHtml(html)) return;

      e.preventDefault();

      try {
        processFigmaHtml(html);
      } catch (err) {
        console.error('[figma-paste] Failed to paste Figma clipboard data:', err);
      }
    };

    document.addEventListener('paste', handlePaste);
    return () => document.removeEventListener('paste', handlePaste);
  }, []);
}
