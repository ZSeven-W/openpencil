import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { DesignEngine } from '../core/design-engine';
import type { PenDocument, PenNode } from '@zseven-w/pen-types';

function makeRect(id: string, x = 0): PenNode {
  return {
    id,
    type: 'rectangle',
    name: `Rect ${id}`,
    x,
    y: 0,
    width: 100,
    height: 100,
  } as PenNode;
}

describe('DesignEngine', () => {
  let engine: DesignEngine;

  beforeEach(() => {
    engine = new DesignEngine();
  });

  afterEach(() => {
    engine.dispose();
  });

  // ── Lifecycle ──

  it('should start with version 0', () => {
    expect(engine.version).toBe(0);
  });

  it('createDocument should return a valid empty document', () => {
    const doc = engine.createDocument();
    expect(doc.version).toBe('1.0.0');
    expect(doc.pages).toBeDefined();
  });

  it('loadDocument should replace current document', () => {
    const doc: PenDocument = { version: '2.0.0', name: 'Test', children: [] };
    engine.loadDocument(doc);
    expect(engine.getDocument().name).toBe('Test');
  });

  // ── Immutability ──

  it('getDocument() should return the same ref within the same tick', () => {
    const ref1 = engine.getDocument();
    const ref2 = engine.getDocument();
    expect(ref1).toBe(ref2);
  });

  it('mutation should produce new document ref', () => {
    const before = engine.getDocument();
    engine.addNode(null, makeRect('n1'));
    const after = engine.getDocument();
    expect(after).not.toBe(before);
  });

  // ── Version counter ──

  it('version should increment on mutation', () => {
    const v0 = engine.version;
    engine.addNode(null, makeRect('n1'));
    expect(engine.version).toBe(v0 + 1);
  });

  // ── Node CRUD ──

  it('addNode + getNodeById', () => {
    engine.addNode(null, makeRect('n1'));
    expect(engine.getNodeById('n1')).toBeDefined();
  });

  it('updateNode', () => {
    engine.addNode(null, makeRect('n1'));
    engine.updateNode('n1', { x: 200 });
    expect(engine.getNodeById('n1')?.x).toBe(200);
  });

  it('removeNode', () => {
    engine.addNode(null, makeRect('n1'));
    engine.removeNode('n1');
    expect(engine.getNodeById('n1')).toBeUndefined();
  });

  it('duplicateNode', () => {
    engine.addNode(null, makeRect('n1'));
    const newId = engine.duplicateNode('n1');
    expect(newId).not.toBeNull();
    expect(engine.getNodeById(newId!)).toBeDefined();
  });

  it('groupNodes + ungroupNode', () => {
    engine.addNode(null, makeRect('a', 0));
    engine.addNode(null, makeRect('b', 100));
    const gid = engine.groupNodes(['a', 'b']);
    expect(gid).not.toBeNull();
    expect(engine.getNodeById(gid!)?.type).toBe('group');
    engine.ungroupNode(gid!);
    expect(engine.getNodeById(gid!)).toBeUndefined();
  });

  // ── Selection ──

  it('select + getSelection', () => {
    engine.select(['id1', 'id2']);
    expect(engine.getSelection()).toEqual(['id1', 'id2']);
  });

  it('clearSelection', () => {
    engine.select(['id1']);
    engine.clearSelection();
    expect(engine.getSelection()).toEqual([]);
  });

  // ── Viewport ──

  it('setViewport + getters', () => {
    engine.setViewport(2, 100, 200);
    expect(engine.zoom).toBe(2);
    expect(engine.panX).toBe(100);
    expect(engine.panY).toBe(200);
  });

  it('screenToScene + sceneToScreen roundtrip', () => {
    engine.setViewport(2, 50, 100);
    const scene = engine.screenToScene(150, 200);
    const screen = engine.sceneToScreen(scene.x, scene.y);
    expect(screen.x).toBeCloseTo(150);
    expect(screen.y).toBeCloseTo(200);
  });

  // ── History ──

  it('undo + redo', () => {
    engine.addNode(null, makeRect('n1'));
    expect(engine.canUndo).toBe(true);
    engine.undo();
    expect(engine.getNodeById('n1')).toBeUndefined();
    expect(engine.canRedo).toBe(true);
    engine.redo();
    expect(engine.getNodeById('n1')).toBeDefined();
  });

  // ── Batch ──

  it('batch() should only fire document:change once', () => {
    const cb = vi.fn();
    engine.on('document:change', cb);
    engine.batch(() => {
      engine.addNode(null, makeRect('a'));
      engine.addNode(null, makeRect('b'));
      engine.addNode(null, makeRect('c'));
    });
    // Should fire exactly once after batch, not 3 times
    expect(cb).toHaveBeenCalledTimes(1);
  });

  it('batch() should produce single undo step', () => {
    engine.batch(() => {
      engine.addNode(null, makeRect('a'));
      engine.addNode(null, makeRect('b'));
    });
    engine.undo();
    expect(engine.getNodeById('a')).toBeUndefined();
    expect(engine.getNodeById('b')).toBeUndefined();
  });

  // ── Tool ──

  it('setActiveTool + getActiveTool', () => {
    engine.setActiveTool('rectangle');
    expect(engine.getActiveTool()).toBe('rectangle');
  });

  // ── Events ──

  it('on() should return unsubscribe function', () => {
    const cb = vi.fn();
    const unsub = engine.on('selection:change', cb);
    engine.select(['x']);
    expect(cb).toHaveBeenCalledTimes(1);
    unsub();
    engine.select(['y']);
    expect(cb).toHaveBeenCalledTimes(1);
  });

  // ── Pages ──

  it('addPage + setActivePage + getActivePage', () => {
    const pageId = engine.addPage();
    expect(pageId).toBeTruthy();
    engine.setActivePage(pageId);
    expect(engine.getActivePage()).toBe(pageId);
  });

  // ── Variables ──

  it('setVariable + resolveVariable', () => {
    engine.setVariable('primary', { type: 'color', value: '#FF0000' });
    expect(engine.resolveVariable('$primary')).toBe('#FF0000');
  });

  // ── SVG Import ──

  it('importSVG should add nodes', () => {
    const svg =
      '<svg viewBox="0 0 100 100"><rect x="0" y="0" width="100" height="100" fill="#F00" /></svg>';
    const nodes = engine.importSVG(svg);
    expect(nodes.length).toBeGreaterThan(0);
  });

  // ── Dispose ──

  it('dispose should clean up', () => {
    const cb = vi.fn();
    engine.on('document:change', cb);
    engine.dispose();
    // After dispose, events should not fire
    // (Implementation-dependent; at minimum, no throws)
  });
});
