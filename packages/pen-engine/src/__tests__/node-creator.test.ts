import { describe, it, expect } from 'vitest';
import { createNodeForTool, isDrawingTool } from '../core/node-creator';

describe('createNodeForTool', () => {
  it('should create a rectangle node', () => {
    const node = createNodeForTool('rectangle', 10, 20, 100, 50);
    expect(node).not.toBeNull();
    expect(node!.type).toBe('rectangle');
    expect(node!.x).toBe(10);
    expect(node!.y).toBe(20);
    expect((node as any).width).toBe(100);
    expect((node as any).height).toBe(50);
    expect(node!.id).toBeTruthy();
  });

  it('should create a frame node with empty children', () => {
    const node = createNodeForTool('frame', 0, 0, 200, 200);
    expect(node).not.toBeNull();
    expect(node!.type).toBe('frame');
    expect((node as any).children).toEqual([]);
  });

  it('should create an ellipse node', () => {
    const node = createNodeForTool('ellipse', 0, 0, 100, 100);
    expect(node).not.toBeNull();
    expect(node!.type).toBe('ellipse');
  });

  it('should create a line node', () => {
    const node = createNodeForTool('line', 10, 20, 100, 50);
    expect(node).not.toBeNull();
    expect(node!.type).toBe('line');
    expect((node as any).x2).toBe(110);
    expect((node as any).y2).toBe(70);
  });

  it('should create a text node', () => {
    const node = createNodeForTool('text', 50, 50, 0, 0);
    expect(node).not.toBeNull();
    expect(node!.type).toBe('text');
    expect((node as any).content).toBe('Type here');
  });

  it('should create a polygon node', () => {
    const node = createNodeForTool('polygon', 0, 0, 100, 100);
    expect(node).not.toBeNull();
    expect(node!.type).toBe('polygon');
    expect((node as any).polygonCount).toBe(3);
  });

  it('should return null for select tool', () => {
    expect(createNodeForTool('select', 0, 0, 0, 0)).toBeNull();
  });

  it('should return null for hand tool', () => {
    expect(createNodeForTool('hand', 0, 0, 0, 0)).toBeNull();
  });
});

describe('isDrawingTool', () => {
  it('should return false for select', () => {
    expect(isDrawingTool('select')).toBe(false);
  });

  it('should return false for hand', () => {
    expect(isDrawingTool('hand')).toBe(false);
  });

  it('should return true for rectangle', () => {
    expect(isDrawingTool('rectangle')).toBe(true);
  });

  it('should return true for path', () => {
    expect(isDrawingTool('path')).toBe(true);
  });
});
