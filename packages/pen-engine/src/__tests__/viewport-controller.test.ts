import { describe, it, expect, vi, beforeEach } from 'vitest';
import { ViewportController } from '../core/viewport-controller';

describe('ViewportController', () => {
  let vc: ViewportController;
  let onChange: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    onChange = vi.fn();
    vc = new ViewportController({ onChange });
  });

  it('should start at zoom=1, pan=(0,0)', () => {
    expect(vc.zoom).toBe(1);
    expect(vc.panX).toBe(0);
    expect(vc.panY).toBe(0);
  });

  it('setViewport should update zoom and pan', () => {
    vc.setViewport(2, 100, 200);
    expect(vc.zoom).toBe(2);
    expect(vc.panX).toBe(100);
    expect(vc.panY).toBe(200);
    expect(onChange).toHaveBeenCalledWith({ zoom: 2, panX: 100, panY: 200 });
  });

  it('setViewport should clamp zoom to MIN/MAX', () => {
    vc.setViewport(0.001, 0, 0);
    expect(vc.zoom).toBeGreaterThanOrEqual(0.01);
    vc.setViewport(100, 0, 0);
    expect(vc.zoom).toBeLessThanOrEqual(64);
  });

  it('screenToScene should convert coordinates', () => {
    vc.setViewport(2, 50, 100);
    const scene = vc.screenToScene(150, 200);
    // scene.x = (150 - 50) / 2 = 50
    // scene.y = (200 - 100) / 2 = 50
    expect(scene.x).toBe(50);
    expect(scene.y).toBe(50);
  });

  it('sceneToScreen should be inverse of screenToScene', () => {
    vc.setViewport(2, 50, 100);
    const scene = vc.screenToScene(150, 200);
    const screen = vc.sceneToScreen(scene.x, scene.y);
    expect(screen.x).toBeCloseTo(150);
    expect(screen.y).toBeCloseTo(200);
  });

  it('zoomToRect should fit a rectangle in the container', () => {
    vc.zoomToRect(0, 0, 800, 600, 400, 300);
    // Container is 400x300, content is 800x600
    // Scale to fit with padding: min(400/800, 300/600) = 0.5
    expect(vc.zoom).toBeCloseTo(0.5, 1);
  });

  it('zoomToRect should not zoom past 1x', () => {
    vc.zoomToRect(0, 0, 100, 100, 800, 600);
    // Content 100x100 fits easily in 800x600; zoom capped at 1
    expect(vc.zoom).toBeLessThanOrEqual(1);
  });
});
