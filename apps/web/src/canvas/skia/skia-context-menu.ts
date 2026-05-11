import type { SkiaEngine } from './skia-engine';
import { hitTestPathControl } from './skia-hit-handlers';
import { getPrimarySelectionIdForHit } from './skia-selection-hit';
import { useCanvasStore } from '@/stores/canvas-store';
import { useDocumentStore } from '@/stores/document-store';

export interface CanvasContextMenuState {
  x: number;
  y: number;
}

interface PathAnchorContextMenuState {
  x: number;
  y: number;
  nodeId: string;
  anchorIndex: number;
}

interface HandleCanvasContextMenuOptions {
  event: MouseEvent;
  scene: { x: number; y: number };
  engine: SkiaEngine;
  onPathAnchorContextMenu: (state: PathAnchorContextMenuState | null) => void;
  onCanvasContextMenu: (state: CanvasContextMenuState | null) => void;
}

export function handleCanvasContextMenu({
  event,
  scene,
  engine,
  onPathAnchorContextMenu,
  onCanvasContextMenu,
}: HandleCanvasContextMenuOptions) {
  const pathHit = hitTestPathControl(engine, scene.x, scene.y);
  if (pathHit) {
    onCanvasContextMenu(null);
    useCanvasStore.getState().setSelection([pathHit.nodeId], pathHit.nodeId);
    onPathAnchorContextMenu({
      x: event.clientX,
      y: event.clientY,
      nodeId: pathHit.nodeId,
      anchorIndex: pathHit.anchorIndex,
    });
    return;
  }

  onPathAnchorContextMenu(null);

  const hits = engine.spatialIndex.hitTest(scene.x, scene.y);
  if (hits.length === 0) {
    onCanvasContextMenu(null);
    return;
  }

  const nodeId = hits[0].node.id;
  const selectedIds = useCanvasStore.getState().selection.selectedIds;
  const docStore = useDocumentStore.getState();
  const isCoveredBySelection = selectedIds.some(
    (selectedId) => selectedId === nodeId || docStore.isDescendantOf(nodeId, selectedId),
  );

  if (!isCoveredBySelection) {
    const selectedNodeId = getPrimarySelectionIdForHit(nodeId);
    useCanvasStore.getState().setSelection([selectedNodeId], selectedNodeId);
  }

  if (useCanvasStore.getState().selection.selectedIds.length === 0) {
    onCanvasContextMenu(null);
    return;
  }

  onCanvasContextMenu({ x: event.clientX, y: event.clientY });
}
