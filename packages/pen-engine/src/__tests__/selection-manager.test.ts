import { describe, it, expect, vi, beforeEach } from 'vitest';
import { SelectionManager } from '../core/selection-manager';

describe('SelectionManager', () => {
  let sm: SelectionManager;
  let onChange: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    onChange = vi.fn();
    sm = new SelectionManager({ onChange });
  });

  it('should start with empty selection', () => {
    expect(sm.getSelection()).toEqual([]);
    expect(sm.getActiveId()).toBeNull();
  });

  it('select() should set selection and fire onChange', () => {
    sm.select(['node-1', 'node-2']);
    expect(sm.getSelection()).toEqual(['node-1', 'node-2']);
    expect(onChange).toHaveBeenCalledWith(['node-1', 'node-2']);
  });

  it('select() with activeId should set active node', () => {
    sm.select(['node-1', 'node-2'], 'node-2');
    expect(sm.getActiveId()).toBe('node-2');
  });

  it('clearSelection() should reset to empty', () => {
    sm.select(['node-1']);
    sm.clearSelection();
    expect(sm.getSelection()).toEqual([]);
    expect(sm.getActiveId()).toBeNull();
    expect(onChange).toHaveBeenCalledWith([]);
  });

  it('should return stable reference when selection unchanged', () => {
    sm.select(['a', 'b']);
    const ref1 = sm.getSelection();
    // Setting the same selection should keep the same reference
    const ref2 = sm.getSelection();
    expect(ref1).toBe(ref2);
  });

  it('should return new reference after mutation', () => {
    sm.select(['a']);
    const ref1 = sm.getSelection();
    sm.select(['b']);
    const ref2 = sm.getSelection();
    expect(ref1).not.toBe(ref2);
  });

  it('setHoveredId should update and fire callback', () => {
    const onHover = vi.fn();
    sm = new SelectionManager({ onChange, onHover });
    sm.setHoveredId('node-3');
    expect(sm.getHoveredId()).toBe('node-3');
    expect(onHover).toHaveBeenCalledWith('node-3');
  });

  it('setHoveredId(null) should clear hover', () => {
    const onHover = vi.fn();
    sm = new SelectionManager({ onChange, onHover });
    sm.setHoveredId('node-3');
    sm.setHoveredId(null);
    expect(sm.getHoveredId()).toBeNull();
    expect(onHover).toHaveBeenCalledWith(null);
  });
});
