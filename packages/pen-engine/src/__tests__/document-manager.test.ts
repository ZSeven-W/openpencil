import { describe, it, expect, vi, beforeEach } from 'vitest';
import { DocumentManager } from '../core/document-manager';
import { HistoryManager } from '../core/history-manager';
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

describe('DocumentManager', () => {
  let dm: DocumentManager;
  let hm: HistoryManager;
  let onChange: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    onChange = vi.fn();
    hm = new HistoryManager();
    dm = new DocumentManager({
      historyManager: hm,
      onChange,
    });
  });

  it('should start with an empty document', () => {
    const doc = dm.getDocument();
    expect(doc).toBeDefined();
    expect(doc.version).toBe('1.0.0');
  });

  it('loadDocument should replace document and reset history', () => {
    const custom: PenDocument = {
      version: '2.0.0',
      name: 'Custom',
      children: [makeRect('r1')],
    };
    dm.loadDocument(custom);
    expect(dm.getDocument().name).toBe('Custom');
    expect(hm.canUndo).toBe(false);
  });

  it('getDocument() should return immutable ref (same tick = same ref)', () => {
    const ref1 = dm.getDocument();
    const ref2 = dm.getDocument();
    expect(ref1).toBe(ref2);
  });

  it('addNode should mutate document and produce new ref', () => {
    const before = dm.getDocument();
    const node = makeRect('n1');
    dm.addNode(null, node);
    const after = dm.getDocument();
    expect(after).not.toBe(before);
    expect(dm.getNodeById('n1')).toBeDefined();
    expect(onChange).toHaveBeenCalled();
  });

  it('addNode should push history', () => {
    dm.addNode(null, makeRect('n1'));
    expect(hm.canUndo).toBe(true);
  });

  it('updateNode should update node properties', () => {
    dm.addNode(null, makeRect('n1'));
    dm.updateNode('n1', { x: 200 });
    const node = dm.getNodeById('n1');
    expect(node?.x).toBe(200);
  });

  it('removeNode should remove node from document', () => {
    dm.addNode(null, makeRect('n1'));
    dm.removeNode('n1');
    expect(dm.getNodeById('n1')).toBeUndefined();
  });

  it('moveNode should change node parent', () => {
    const frame: PenNode = {
      id: 'frame1',
      type: 'frame',
      name: 'Frame',
      x: 0,
      y: 0,
      width: 500,
      height: 500,
      children: [],
    } as PenNode;
    dm.addNode(null, frame);
    dm.addNode(null, makeRect('n1'));
    dm.moveNode('n1', 'frame1', 0);
    // n1 should now be inside frame1
    const f = dm.getNodeById('frame1') as PenNode & { children?: PenNode[] };
    expect(f.children?.some((c) => c.id === 'n1')).toBe(true);
  });

  it('duplicateNode should create a copy with new ID', () => {
    dm.addNode(null, makeRect('n1'));
    const newId = dm.duplicateNode('n1');
    expect(newId).not.toBeNull();
    expect(newId).not.toBe('n1');
    expect(dm.getNodeById(newId!)).toBeDefined();
  });

  it('groupNodes should wrap nodes in a group', () => {
    dm.addNode(null, makeRect('a', 10));
    dm.addNode(null, makeRect('b', 110));
    const groupId = dm.groupNodes(['a', 'b']);
    expect(groupId).not.toBeNull();
    const group = dm.getNodeById(groupId!) as PenNode & { children?: PenNode[] };
    expect(group.type).toBe('group');
    expect(group.children?.length).toBe(2);
  });

  it('ungroupNode should dissolve group', () => {
    dm.addNode(null, makeRect('a'));
    dm.addNode(null, makeRect('b'));
    const groupId = dm.groupNodes(['a', 'b'])!;
    dm.ungroupNode(groupId);
    expect(dm.getNodeById(groupId)).toBeUndefined();
    // Children should still exist at root level
    expect(dm.getNodeById('a')).toBeDefined();
    expect(dm.getNodeById('b')).toBeDefined();
  });
});
