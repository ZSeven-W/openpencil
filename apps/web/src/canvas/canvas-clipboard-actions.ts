import { useCanvasStore } from '@/stores/canvas-store';
import { useDocumentStore } from '@/stores/document-store';
import type { PenNode } from '@/types/pen';

export interface CopySelectedNodeIdsResult {
  copiedIds: string[];
  systemClipboardWritten: boolean;
}

function getSelectedDocumentNodes(): PenNode[] {
  const { selectedIds } = useCanvasStore.getState().selection;
  if (selectedIds.length === 0) return [];

  const documentStore = useDocumentStore.getState();
  return selectedIds
    .map((id) => documentStore.getNodeById(id))
    .filter((node): node is PenNode => node != null);
}

export function copySelectedNodesToInternalClipboard(): number {
  const nodes = getSelectedDocumentNodes();
  if (nodes.length === 0) return 0;

  useCanvasStore.getState().setClipboard(structuredClone(nodes));
  return nodes.length;
}

function formatNodeIdsForMcp(ids: string[]): string {
  if (ids.length === 1) return ids[0] ?? '';
  return JSON.stringify(ids);
}

async function writeSystemClipboardText(text: string): Promise<boolean> {
  try {
    if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return true;
    }
  } catch {
    // Fall through to Electron clipboard bridge when browser permissions fail.
  }

  try {
    if (typeof window !== 'undefined' && window.electronAPI?.writeClipboardText) {
      await window.electronAPI.writeClipboardText(text);
      return true;
    }
  } catch {
    // Ignore clipboard failures; the caller can inspect the result.
  }

  return false;
}

export async function copySelectedNodeIdsToClipboard(): Promise<CopySelectedNodeIdsResult> {
  const ids = getSelectedDocumentNodes().map((node) => node.id);
  if (ids.length === 0) {
    return { copiedIds: [], systemClipboardWritten: false };
  }

  return {
    copiedIds: ids,
    systemClipboardWritten: await writeSystemClipboardText(formatNodeIdsForMcp(ids)),
  };
}
