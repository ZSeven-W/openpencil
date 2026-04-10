import { describe, expect, it } from 'vitest';

import { createEmptyDocument } from '@/stores/document-tree-utils';
import { useCanvasStore } from '@/stores/canvas-store';
import { useDocumentStore } from '@/stores/document-store';
import type { PenNode } from '@/types/pen';

import { SkiaInteractionManager } from '../skia-interaction';

function createCanvasStub() {
  return {
    style: { cursor: 'default' },
  } as unknown as HTMLCanvasElement;
}

function createEngineStub(renderNodes: Array<any>) {
  let rebuildCount = 0;
  let dirtyCount = 0;
  const spatialIndex = {
    get: (id: string) => renderNodes.find((rn) => rn.node.id === id) ?? null,
    rebuild: () => {
      rebuildCount += 1;
    },
    hitTest: () => [],
    searchRect: () => [],
  };

  return {
    zoom: 1,
    panX: 0,
    panY: 0,
    renderNodes,
    spatialIndex,
    dragSyncSuppressed: false,
    getCanvasRect: () =>
      ({
        left: 0,
        top: 0,
        width: 1000,
        height: 1000,
      }) as DOMRect,
    markDirty: () => {
      dirtyCount += 1;
    },
    get rebuildCount() {
      return rebuildCount;
    },
    get dirtyCount() {
      return dirtyCount;
    },
  };
}

function resetStores() {
  useCanvasStore.setState({
    activeTool: 'select',
    selection: {
      ...useCanvasStore.getState().selection,
      selectedIds: [],
      activeId: null,
    },
  });
  useDocumentStore.setState({
    document: createEmptyDocument(),
    isDirty: false,
    fileHandle: null,
    fileName: null,
    filePath: null,
  } as any);
}

describe('SkiaInteractionManager continuous interaction commits', () => {
  it('defers resize store writes until mouseup', () => {
    resetStores();
    let node: any = {
      id: 'path-1',
      type: 'path',
      x: 10,
      y: 20,
      width: 100,
      height: 50,
      d: 'M 0 0 L 100 50',
      stroke: { thickness: 1, fill: [{ type: 'solid', color: '#000000' }] },
    } as PenNode;
    const updateNodeCalls: Array<[string, Partial<PenNode>]> = [];
    const scaleCalls: Array<[string, number, number]> = [];
    const updateNode = (id: string, updates: Partial<PenNode>) => {
      updateNodeCalls.push([id, updates]);
      expect(id).toBe('path-1');
      node = { ...node, ...updates };
    };
    const scaleDescendantsInStore = (id: string, scaleX: number, scaleY: number) => {
      scaleCalls.push([id, scaleX, scaleY]);
    };

    useDocumentStore.setState({
      getNodeById: (id: string) => (id === 'path-1' ? node : undefined),
      updateNode,
      scaleDescendantsInStore,
    } as any);

    const renderNode = {
      node: { ...node },
      absX: 110,
      absY: 220,
      absW: 100,
      absH: 50,
    };
    const engine = createEngineStub([renderNode]);
    const manager = new SkiaInteractionManager(
      { current: engine as any },
      createCanvasStub(),
      () => {},
    ) as any;

    manager.isResizing = true;
    manager.resizeHandle = 'se';
    manager.resizeNodeId = 'path-1';
    manager.resizeOrigX = 10;
    manager.resizeOrigY = 20;
    manager.resizeOrigW = 100;
    manager.resizeOrigH = 50;
    manager.resizeStartSceneX = 110;
    manager.resizeStartSceneY = 220;

    manager.handleResizeMove({ x: 150, y: 250 }, engine as any);

    expect(updateNodeCalls).toHaveLength(0);
    expect(scaleCalls).toHaveLength(0);
    expect(renderNode.absW).toBe(140);
    expect(renderNode.absH).toBe(80);
    expect(engine.dirtyCount).toBeGreaterThan(0);

    manager.onMouseUp();

    expect(updateNodeCalls).toHaveLength(1);
    expect(updateNodeCalls[0]?.[1]).toMatchObject({
      x: 10,
      y: 20,
      width: 140,
      height: 80,
    });
  });

  it('defers rotate store writes until mouseup', () => {
    resetStores();
    let node: any = {
      id: 'rect-1',
      type: 'rectangle',
      x: 100,
      y: 120,
      width: 80,
      height: 40,
      rotation: 0,
      fill: [{ type: 'solid', color: '#ffffff' }],
    } as PenNode;
    const updateNodeCalls: Array<[string, Partial<PenNode>]> = [];
    const updateNode = (id: string, updates: Partial<PenNode>) => {
      updateNodeCalls.push([id, updates]);
      expect(id).toBe('rect-1');
      node = { ...node, ...updates };
    };

    useDocumentStore.setState({
      getNodeById: (id: string) => (id === 'rect-1' ? node : undefined),
      updateNode,
    } as any);

    const renderNode = {
      node: { ...node },
      absX: 100,
      absY: 120,
      absW: 80,
      absH: 40,
    };
    const engine = createEngineStub([renderNode]);
    const manager = new SkiaInteractionManager(
      { current: engine as any },
      createCanvasStub(),
      () => {},
    ) as any;

    manager.isRotating = true;
    manager.rotateNodeId = 'rect-1';
    manager.rotateOrigAngle = 0;
    manager.rotateCenterX = 140;
    manager.rotateCenterY = 140;
    manager.rotateStartAngle = 0;

    manager.handleRotateMove({ x: 140, y: 200 }, false);

    expect(updateNodeCalls).toHaveLength(0);
    expect(renderNode.node.rotation).not.toBe(0);

    manager.onMouseUp();

    expect(updateNodeCalls).toHaveLength(1);
    expect(updateNodeCalls[0]?.[1]).toHaveProperty('rotation');
  });

  it('defers arc handle store writes until mouseup', () => {
    resetStores();
    let node: any = {
      id: 'ellipse-1',
      type: 'ellipse',
      x: 200,
      y: 200,
      width: 100,
      height: 100,
      startAngle: 0,
      sweepAngle: 360,
      innerRadius: 0,
      fill: [{ type: 'solid', color: '#ffffff' }],
      stroke: { thickness: 1, fill: [{ type: 'solid', color: '#000000' }] },
    } as PenNode;
    const updateNodeCalls: Array<[string, Partial<PenNode>]> = [];
    const updateNode = (id: string, updates: Partial<PenNode>) => {
      updateNodeCalls.push([id, updates]);
      expect(id).toBe('ellipse-1');
      node = { ...node, ...updates };
    };

    useDocumentStore.setState({
      getNodeById: (id: string) => (id === 'ellipse-1' ? node : undefined),
      updateNode,
    } as any);

    const renderNode = {
      node: { ...node },
      absX: 200,
      absY: 200,
      absW: 100,
      absH: 100,
    };
    const engine = createEngineStub([renderNode]);
    const manager = new SkiaInteractionManager(
      { current: engine as any },
      createCanvasStub(),
      () => {},
    ) as any;

    manager.isDraggingArc = true;
    manager.arcNodeId = 'ellipse-1';
    manager.arcHandleType = 'inner';

    manager.handleArcMove({ x: 225, y: 250 }, engine as any);

    expect(updateNodeCalls).toHaveLength(0);
    expect(renderNode.node.innerRadius).not.toBe(0);

    manager.onMouseUp();

    expect(updateNodeCalls).toHaveLength(1);
    expect(updateNodeCalls[0]?.[1]).toHaveProperty('innerRadius');
  });
});
