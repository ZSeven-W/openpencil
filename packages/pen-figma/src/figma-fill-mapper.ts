import type { FigmaPaint, FigmaMatrix } from './figma-types';
import type { ImageOriginalSize, ImageTransform, PenFill } from '@zseven-w/pen-types';
import { figmaColorToHex } from './figma-color-utils';

const IMAGE_TRANSFORM_EPSILON = 0.000001;

/**
 * Convert Figma fillPaints (internal format) to PenFill[].
 */
export function mapFigmaFills(paints: FigmaPaint[] | undefined): PenFill[] | undefined {
  if (!paints || paints.length === 0) return undefined;
  const fills: PenFill[] = [];

  for (const paint of paints) {
    if (paint.visible === false) continue;
    const mapped = mapSingleFill(paint);
    if (mapped) fills.push(mapped);
  }

  return fills.length > 0 ? fills : undefined;
}

function mapSingleFill(paint: FigmaPaint): PenFill | null {
  switch (paint.type) {
    case 'SOLID': {
      if (!paint.color) return null;
      return {
        type: 'solid',
        color: figmaColorToHex(paint.color),
        opacity: paint.opacity,
      };
    }

    case 'GRADIENT_LINEAR': {
      if (!paint.stops) return null;
      const angle = paint.transform ? gradientAngleFromTransform(paint.transform) : 0;
      return {
        type: 'linear_gradient',
        angle,
        stops: paint.stops.map((s) => ({
          offset: s.position,
          color: figmaColorToHex(s.color),
        })),
        opacity: paint.opacity,
      };
    }

    case 'GRADIENT_RADIAL':
    case 'GRADIENT_ANGULAR':
    case 'GRADIENT_DIAMOND': {
      if (!paint.stops) return null;
      return {
        type: 'radial_gradient',
        cx: 0.5,
        cy: 0.5,
        radius: 0.5,
        stops: paint.stops.map((s) => ({
          offset: s.position,
          color: figmaColorToHex(s.color),
        })),
        opacity: paint.opacity,
      };
    }

    case 'IMAGE': {
      // Image fills reference blobs or ZIP image files; we'll resolve them later
      let url = '';
      if (paint.image?.hash && paint.image.hash.length > 0) {
        url = `__hash:${Array.from(paint.image.hash)
          .map((b) => b.toString(16).padStart(2, '0'))
          .join('')}`;
      } else if (paint.image?.dataBlob !== undefined) {
        url = `__blob:${paint.image.dataBlob}`;
      }
      return {
        type: 'image',
        url,
        mode: mapScaleMode(paint.imageScaleMode),
        originalSize: normalizeOriginalSize(paint.originalImageWidth, paint.originalImageHeight),
        transform: normalizeImageTransform(paint.transform),
        opacity: paint.opacity,
      };
    }

    default:
      return null;
  }
}

function gradientAngleFromTransform(m: FigmaMatrix): number {
  // Figma gradient direction is (m00, m10) in object space (default = horizontal).
  // atan2 gives the math-convention angle (0° = right, CCW).
  // Convert to CSS gradient convention (0° = bottom-to-top, 90° = left-to-right).
  const mathAngle = Math.atan2(m.m10, m.m00) * (180 / Math.PI);
  return Math.round(90 - mathAngle);
}

function normalizeOriginalSize(width?: number, height?: number): ImageOriginalSize | undefined {
  if (
    typeof width !== 'number' ||
    typeof height !== 'number' ||
    !Number.isFinite(width) ||
    !Number.isFinite(height) ||
    width <= 0 ||
    height <= 0
  ) {
    return undefined;
  }

  return { width, height };
}

function normalizeImageTransform(transform?: FigmaMatrix): ImageTransform | undefined {
  if (!transform) return undefined;

  if (
    Math.abs(transform.m00 - 1) <= IMAGE_TRANSFORM_EPSILON &&
    Math.abs(transform.m01) <= IMAGE_TRANSFORM_EPSILON &&
    Math.abs(transform.m02) <= IMAGE_TRANSFORM_EPSILON &&
    Math.abs(transform.m10) <= IMAGE_TRANSFORM_EPSILON &&
    Math.abs(transform.m11 - 1) <= IMAGE_TRANSFORM_EPSILON &&
    Math.abs(transform.m12) <= IMAGE_TRANSFORM_EPSILON
  ) {
    return undefined;
  }

  return {
    m00: transform.m00,
    m01: transform.m01,
    m02: transform.m02,
    m10: transform.m10,
    m11: transform.m11,
    m12: transform.m12,
  };
}

function mapScaleMode(mode?: string): 'stretch' | 'fill' | 'fit' {
  switch (mode) {
    case 'FIT':
      return 'fit';
    case 'STRETCH':
      return 'stretch';
    default:
      return 'fill';
  }
}
