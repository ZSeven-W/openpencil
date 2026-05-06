import { useEffect, useState, useCallback } from 'react';
import { useDocumentStore } from '@/stores/document-store';
import { useCanvasStore } from '@/stores/canvas-store';
import { parseAndPrepareImportedDocument } from '@/utils/import-pen-document';
import type { PenDocument } from '@/types/pen';

/**
 * Parse a 将
 * File 放入 PenDocument。如果文件不是有效的
 .op/.pen/.json 文档，则 Returns null。
 */
async function parseDroppedFile(
  file: File,
): Promise<{ doc: PenDocument; fileName: string } | null> {
  const ext = file.name.split('.').pop()?.toLowerCase();
  if (ext !== 'op' && ext !== 'pen' && ext !== 'json') return null;

  try {
    const text = await file.text();
    const prepared = parseAndPrepareImportedDocument(text, {
      fileName: file.name,
    });
    if (!prepared) return null;
    const { doc } = prepared;
    return { doc, fileName: file.name };
  } catch {
    return null;
  }
}

/**
 * Hook 允许在编辑器上
 * 拖放文件打开。 Returns `isDragging` 渲染放置区域叠加层的状态。
 */
export function useFileDrop() {
  const [isDragging, setIsDragging] = useState(false);

  // Track 嵌套拖动 enter/leave 所以叠加不会闪烁
  const handleDragEnter = useCallback((e: DragEvent) => {
    e.preventDefault();
    if (e.dataTransfer?.types.includes('Files')) {
      setIsDragging(true);
    }
  }, []);

  const handleDragOver = useCallback((e: DragEvent) => {
    e.preventDefault();
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = 'copy';
    }
  }, []);

  const handleDragLeave = useCallback((e: DragEvent) => {
    // Only 离开窗口时关闭覆盖（relatedTarget 为空）
    if (!e.relatedTarget) {
      setIsDragging(false);
    }
  }, []);

  const handleDrop = useCallback(async (e: DragEvent) => {
    e.preventDefault();
    setIsDragging(false);

    const file = e.dataTransfer?.files?.[0];
    if (!file) return;

    // .fig 文件 → 打开 Figma 导入对话框并预加载文件
    const ext = file.name.split('.').pop()?.toLowerCase();
    if (ext === 'fig') {
      const store = useCanvasStore.getState();
      store.setPendingFigmaFile(file);
      store.setFigmaImportDialogOpen(true);
      return;
    }

    const result = await parseDroppedFile(file);
    if (!result) return;

    // In Electron，解析绝对文件系统路径，以便稍后可以单击最近的文件条目。 In 一个普通浏览器，返回 null。
    const filePath =
      (typeof window !== 'undefined' && window.electronAPI?.getPathForFile?.(file)) || null;

    useDocumentStore.getState().loadDocument(result.doc, result.fileName, null, filePath);

    // Let 画布同步，然后缩放以适合
    const { zoomToFitContent } = await import('@/canvas/skia-engine-ref');
    requestAnimationFrame(() => zoomToFitContent());
  }, []);

  useEffect(() => {
    window.addEventListener('dragenter', handleDragEnter);
    window.addEventListener('dragover', handleDragOver);
    window.addEventListener('dragleave', handleDragLeave);
    window.addEventListener('drop', handleDrop);
    return () => {
      window.removeEventListener('dragenter', handleDragEnter);
      window.removeEventListener('dragover', handleDragOver);
      window.removeEventListener('dragleave', handleDragLeave);
      window.removeEventListener('drop', handleDrop);
    };
  }, [handleDragEnter, handleDragOver, handleDragLeave, handleDrop]);

  return isDragging;
}
